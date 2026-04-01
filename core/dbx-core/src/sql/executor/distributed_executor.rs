//! DistributedExecutor — 분산 쿼리 코디네이터
//!
//! 실행 흐름:
//! 1. FragmentSplitter로 PhysicalPlan을 (coordinator_plan, worker_plan)으로 분할
//! 2. worker_plan을 bincode로 직렬화
//! 3. 모든 peer_addrs에 `ExecuteFragment { plan_bytes }` QUIC 전송
//! 4. GridManager에 수신 큐 등록 (execution_id 기반 DashMap)
//! 5. coordinator_plan에서 GridExchange 플레이스홀더를 GridExchangeOperator로 교체
//! 6. coordinator_plan 실행 → GridExchangeOperator::next()로 배압 수신
//! 7. 최종 RecordBatch 집계 반환

use crate::error::{DbxError, DbxResult};
use crate::grid::manager::GridManager;
use crate::grid::protocol::{GridMessage, QueryMessage};
use crate::grid::quic::QuicChannel;
use crate::sql::executor::fragment_splitter::FragmentSplitter;
use crate::sql::executor::local_executor::LocalExecutor;
use crate::sql::executor::operators::{GridExchangeOperator, PhysicalOperator};
use crate::sql::planner::types::{AggregateFunction, AggregateMode, PhysicalPlan};
use arrow::array::RecordBatch;
use arrow::datatypes::{DataType, Field, Schema};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// 분산 쿼리 코디네이터 실행기
pub struct DistributedExecutor {
    quic_channel: Arc<QuicChannel>,
    grid_manager: Arc<GridManager>,
    local_executor: Arc<LocalExecutor>,
    /// 알려진 워커 노드 목록 (생성자 주입)
    peer_addrs: Vec<SocketAddr>,
}

impl DistributedExecutor {
    pub fn new(
        quic_channel: Arc<QuicChannel>,
        grid_manager: Arc<GridManager>,
        local_executor: Arc<LocalExecutor>,
        peer_addrs: Vec<SocketAddr>,
    ) -> Self {
        Self { quic_channel, grid_manager, local_executor, peer_addrs }
    }

    /// PhysicalPlan을 분산 실행하고 최종 RecordBatch를 반환합니다.
    pub async fn execute(&self, plan: PhysicalPlan) -> DbxResult<Vec<RecordBatch>> {
        let pair = FragmentSplitter::split(plan)?;

        // 코디네이터 플랜이 없으면 로컬 실행 (단일 노드 fallback)
        let coord_plan = match pair.coordinator_plan {
            None => {
                info!("No distributed split found — executing locally");
                return self.local_executor.execute_collect(&pair.worker_plan);
            }
            Some(p) => p,
        };

        let execution_id = {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!("exec-{}-{}", secs, nanos)
        };
        let worker_plan = pair.worker_plan;

        // 워커 플랜 직렬화
        let plan_bytes = bincode::serialize(&worker_plan)
            .map_err(|e| DbxError::Serialization(e.to_string()))?;

        let coordinator_addr = self.quic_channel.local_addr.to_string();

        // 워커 수만큼 배압 채널 생성 (bounded 8로 OOM 방지)
        let channel_size = 8usize;
        let mut senders: Vec<mpsc::Sender<DbxResult<Option<Vec<u8>>>>> = Vec::new();
        let mut receivers: Vec<mpsc::Receiver<DbxResult<Option<Vec<u8>>>>> = Vec::new();

        for _ in &self.peer_addrs {
            let (tx, rx) = mpsc::channel(channel_size);
            senders.push(tx);
            receivers.push(rx);
        }

        // GridManager DashMap에 모든 워커 채널 등록
        // execution_id에 워커 인덱스를 붙여 독립적인 큐 보장
        let query_streams = self.grid_manager.get_query_streams();
        for (idx, sender) in senders.into_iter().enumerate() {
            let key = format!("{}:{}", execution_id, idx);
            query_streams.insert((key, 0), sender);
        }

        // 모든 워커에게 ExecuteFragment 전송
        info!("Dispatching ExecuteFragment to {} workers (exec_id: {})", self.peer_addrs.len(), execution_id);
        for (idx, peer) in self.peer_addrs.iter().enumerate() {
            let msg = GridMessage::Query(QueryMessage::ExecuteFragment {
                execution_id: format!("{}:{}", execution_id, idx),
                plan_bytes: plan_bytes.clone(),
                coordinator_addr: coordinator_addr.clone(),
            });
            if let Err(e) = self.quic_channel.send_message(*peer, msg).await {
                warn!("Failed to send ExecuteFragment to {}: {:?}", peer, e);
            }
        }

        // coordinator_plan의 GridExchange 플레이스홀더를 실제 GridExchangeOperator로 교체
        // 각 워커의 수신 큐를 모아 MergeExchangeOperator처럼 동작하도록 구성
        let merged_rx = self.merge_receivers(receivers);
        let exchange_schema = self.infer_exchange_schema(&coord_plan);
        let exchange_op = Box::new(GridExchangeOperator::new(exchange_schema, merged_rx));

        // coordinator_plan 실행 (GridExchange를 실제 연산자로 교체하여)
        let final_plan = self.inject_exchange(coord_plan, exchange_op);
        let results = self.execute_plan_with_operator(final_plan)?;

        // 완료 후 DashMap 정리
        for idx in 0..self.peer_addrs.len() {
            let key = format!("{}:{}", execution_id, idx);
            query_streams.remove(&(key, 0));
        }

        Ok(results)
    }

    /// 여러 워커의 수신 큐를 단일 mpsc 채널로 합칩니다 (Fan-in Merge)
    fn merge_receivers(
        &self,
        receivers: Vec<mpsc::Receiver<DbxResult<Option<Vec<u8>>>>>,
    ) -> mpsc::Receiver<DbxResult<Option<Vec<u8>>>> {
        let (merge_tx, merge_rx) = mpsc::channel::<DbxResult<Option<Vec<u8>>>>(64);

        tokio::spawn(async move {
            let merge_tx = Arc::new(merge_tx);

            for mut rx in receivers {
                let tx = Arc::clone(&merge_tx);
                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Some(Ok(Some(bytes))) => {
                                let _ = tx.send(Ok(Some(bytes))).await;
                            }
                            Some(Ok(None)) => {
                                // 이 워커의 EOF — merge 채널로 EOF 포워드
                                // 마지막 워커가 EOF를 보낼 때 추적
                                let _ = tx.send(Ok(None)).await;
                                break;
                            }
                            Some(Err(e)) => {
                                let _ = tx.send(Err(e)).await;
                                break;
                            }
                            None => break,
                        }
                    }
                });
            }
        });

        merge_rx
    }

    /// coordinator_plan에서 GridExchange 스키마를 추론
    fn infer_exchange_schema(&self, plan: &PhysicalPlan) -> Arc<Schema> {
        // Final Agg의 입력 스키마 힌트에서 추론
        // 간단히: group_key + agg_columns 패턴
        match plan {
            PhysicalPlan::HashAggregate { group_by, aggregates, .. } => {
                let mut fields = Vec::new();
                for i in 0..group_by.len() {
                    fields.push(Field::new(format!("group_{}", i), DataType::Int64, true));
                }
                for agg in aggregates {
                    let name = agg.alias.clone().unwrap_or_else(|| format!("agg_{}", agg.input));
                    fields.push(Field::new(name, DataType::Int64, true));
                }
                Arc::new(Schema::new(fields))
            }
            _ => Arc::new(Schema::new(vec![Field::new("value", DataType::Int64, true)])),
        }
    }

    /// coordinator_plan에서 GridExchange 플레이스홀더를 주입된 exchange_op으로 교체하고
    /// 남은 상위 노드를 직렬 실행합니다.
    fn inject_exchange(
        &self,
        plan: PhysicalPlan,
        exchange_op: Box<dyn PhysicalOperator>,
    ) -> (PhysicalPlan, Box<dyn PhysicalOperator>) {
        // GridExchange 플레이스홀더를 만나면 exchange_op 반환
        // 상위 플랜은 이 operator를 input으로 래핑
        (plan, exchange_op)
    }

    fn execute_plan_with_operator(
        &self,
        (plan, exchange_op): (PhysicalPlan, Box<dyn PhysicalOperator>),
    ) -> DbxResult<Vec<RecordBatch>> {
        // Final Agg를 exchange_op 위에 직접 조립하여 실행
        let mut final_op: Box<dyn PhysicalOperator> = match plan {
            PhysicalPlan::HashAggregate { group_by, aggregates, mode, .. } => {
                use crate::sql::executor::operators::HashAggregateOperator;
                use arrow::datatypes::{DataType, Field, Schema};

                let input_schema = exchange_op.schema().clone();
                let mut output_fields = Vec::new();
                for &col_idx in &group_by {
                    if col_idx < input_schema.fields().len() {
                        output_fields.push(input_schema.field(col_idx).clone());
                    }
                }
                for agg in &aggregates {
                    let name = agg.alias.clone().unwrap_or_else(|| format!("agg_{}", agg.input));
                    output_fields.push(Field::new(&name, DataType::Int64, true));
                }
                let output_schema = Arc::new(Schema::new(output_fields));

                Box::new(HashAggregateOperator::new(
                    exchange_op,
                    output_schema,
                    group_by,
                    aggregates,
                    mode,
                ))
            }
            _ => exchange_op, // GridExchange 자체가 루트인 경우
        };

        let mut results = Vec::new();
        while let Some(batch) = final_op.next()? {
            if batch.num_rows() > 0 {
                results.push(batch);
            }
        }
        Ok(results)
    }
}
