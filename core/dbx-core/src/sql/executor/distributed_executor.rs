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
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::planner::types::PhysicalPlan;
use crate::storage::metadata::MetadataRegistry;
use arrow::array::RecordBatch;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// 분산 쿼리 코디네이터 실행기
pub struct DistributedExecutor {
    quic_channel: Arc<QuicChannel>,
    grid_manager: Arc<GridManager>,
    local_executor: Arc<LocalExecutor>,
    /// 알려진 워커 노드 목록 (생성자 주입)
    peer_addrs: Vec<SocketAddr>,
    metadata_registry: Arc<MetadataRegistry>,
}

impl DistributedExecutor {
    pub fn new(
        quic_channel: Arc<QuicChannel>,
        grid_manager: Arc<GridManager>,
        local_executor: Arc<LocalExecutor>,
        peer_addrs: Vec<SocketAddr>,
        metadata_registry: Arc<MetadataRegistry>,
    ) -> Self {
        Self {
            quic_channel,
            grid_manager,
            local_executor,
            peer_addrs,
            metadata_registry,
        }
    }

    /// PhysicalPlan을 분산 실행하고 최종 RecordBatch를 반환합니다.
    pub async fn execute(&self, plan: PhysicalPlan) -> DbxResult<Vec<RecordBatch>> {
        // [Phase 9] 로컬 전용 최적화: 피어가 없으면 분산 오버헤드(Split, Thread, Channel) 전체 우회
        if self.peer_addrs.is_empty() {
            return self.local_executor.execute_collect(&plan);
        }

        let dag = FragmentSplitter::split(plan)?;

        // 코디네이터 플랜이 없으면 단건 로컬 실행으로 Fallback
        let coord_plan = match dag.coordinator_plan {
            None => {
                info!("No distributed split found — executing locally");
                // stage가 1개 있다는 의미이므로 그냥 로컬 실행
                let worker_plan = dag
                    .stages
                    .into_iter()
                    .next()
                    .unwrap()
                    .plans
                    .into_iter()
                    .next()
                    .unwrap();
                return self.local_executor.execute_collect(&worker_plan);
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

        // 1. 코디네이터의 `GridExchange` 수신 채널 준비 및 Operator 빌드
        let mut channels = crate::sql::executor::local_executor::DistributedChannels::default();
        let mut root_op = self
            .local_executor
            .build_operator_distributed(&coord_plan, &mut channels)?;

        // 2. DashMap에 `channels.exchanges` Sender 등록 (GridManager가 통신을 수신하여 전달할 수 있도록 함)
        let query_streams = self.grid_manager.get_query_streams();
        for (e_id, tx) in channels.exchanges {
            query_streams.insert((execution_id.clone(), e_id), tx);
        }

        // 3. spawn_blocking으로 코디네이터 플랜 실행을 백그라운드로 밀어넣음
        // (root_op가 내부적으로 GridExchange로부터 튜플을 Pull-based로 소모하기 시작함)
        let coord_task = tokio::task::spawn_blocking(move || {
            let mut results = Vec::new();
            while let Some(batch) = root_op.next()? {
                if batch.num_rows() > 0 {
                    results.push(batch);
                }
            }
            Ok::<_, DbxError>(results)
        });

        // 4. DAG 기반 Multi-Stage 스케줄링 (Stage-by-Stage 순차 장벽 배포)
        let coordinator_addr = self.quic_channel.local_addr.to_string();
        for stage in dag.stages {
            let stage_id = stage.stage_id;

            let mut pending_workers = self.peer_addrs.clone();

            // Phase 6: Locality Routing (가중치 소팅)
            let mut node_scores: std::collections::HashMap<std::net::SocketAddr, usize> =
                std::collections::HashMap::new();

            fn collect_table_scans(p: &PhysicalPlan) -> Vec<&PhysicalPlan> {
                let mut scans = Vec::new();
                match p {
                    PhysicalPlan::TableScan { .. } => scans.push(p),
                    PhysicalPlan::Projection { input, .. }
                    | PhysicalPlan::SortMerge { input, .. }
                    | PhysicalPlan::Limit { input, .. }
                    | PhysicalPlan::HashAggregate { input, .. } => {
                        scans.extend(collect_table_scans(input))
                    }
                    PhysicalPlan::HashJoin { left, right, .. } => {
                        scans.extend(collect_table_scans(left));
                        scans.extend(collect_table_scans(right));
                    }
                    _ => {}
                }
                scans
            }

            for p in &stage.plans {
                let scans = collect_table_scans(p);
                for scan in scans {
                    if let PhysicalPlan::TableScan {
                        table, ros_files, ..
                    } = scan
                        && let Some(table_meta) = self.metadata_registry.tables.get(table)
                    {
                        for part_ref in table_meta.partitions.iter() {
                            let part = part_ref.value();
                            // ros_files가 비어있으면(로컬 fallback) 모든 파티션을 고려, 아니면 ros_files 대상만
                            if (ros_files.is_empty() || ros_files.contains(&part.file_path))
                                && let Some(ref addr_str) = part.node_addr
                                && let Ok(addr) = addr_str.parse::<std::net::SocketAddr>()
                            {
                                *node_scores.entry(addr).or_insert(0) += part.row_count.max(1);
                            }
                        }
                    }
                }
            }

            // node_scores가 높은 순으로 워커 우선순위 정렬
            pending_workers.sort_by(|a, b| {
                let score_a = node_scores.get(a).copied().unwrap_or(0);
                let score_b = node_scores.get(b).copied().unwrap_or(0);
                score_b.cmp(&score_a)
            });

            // 동일 스테이지 내의 플랜들을 직렬화
            let mut plans_bytes = Vec::new();
            for p in stage.plans {
                let bytes =
                    bincode::serialize(&p).map_err(|e| DbxError::Serialization(e.to_string()))?;
                plans_bytes.push(bytes);
            }

            let max_retries = 3;
            // 테스트를 위해 환경변수로 Timeout 조정 (기본값 30초)
            let timeout_secs = std::env::var("DBX_WORKER_TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse::<u64>()
                .unwrap_or(30);
            let timeout_duration = std::time::Duration::from_secs(timeout_secs);

            // 재시도 루프
            for retry_count in 0..=max_retries {
                if pending_workers.is_empty() {
                    break;
                }

                info!(
                    "Dispatching Stage {} to {} workers (exec_id: {}, retry: {})",
                    stage_id,
                    pending_workers.len(),
                    execution_id,
                    retry_count
                );

                let stage_barriers = self.grid_manager.get_stage_barriers();
                let mut awaiters = Vec::new();

                for peer in &pending_workers {
                    // 수신 확인을 위한 1회용 채널
                    let (tx, mut rx) = mpsc::channel(1);
                    stage_barriers.insert((execution_id.clone(), stage_id, *peer), tx);

                    let msg = GridMessage::Query(QueryMessage::ExecuteFragment {
                        execution_id: execution_id.clone(),
                        stage_id,
                        plans_bytes: plans_bytes.clone(),
                        coordinator_addr: coordinator_addr.clone(),
                    });

                    if let Err(e) = self.quic_channel.send_message(*peer, msg).await {
                        warn!("Failed to send Stage {} to {}: {:?}", stage_id, peer, e);
                    }

                    let peer_addr = *peer;
                    awaiters.push(async move {
                        let res = tokio::time::timeout(timeout_duration, rx.recv()).await;
                        (peer_addr, res.is_ok())
                    });
                }

                // 모든 워커의 FragmentCompleted 신호 수신 대기 (Barrier)
                let results = futures::future::join_all(awaiters).await;

                let mut retry_peers = Vec::new();
                for (peer_addr, success) in results {
                    stage_barriers.remove(&(execution_id.clone(), stage_id, peer_addr));
                    if !success {
                        warn!("Worker {} timed out on Stage {}", peer_addr, stage_id);
                        retry_peers.push(peer_addr);
                    }
                }

                pending_workers = retry_peers;

                if !pending_workers.is_empty() && retry_count == max_retries {
                    error!(
                        "Max retries exceeded for Stage {} on workers: {:?}",
                        stage_id, pending_workers
                    );
                    return Err(DbxError::Network(format!(
                        "Stage {} timed out after {} retries",
                        stage_id, max_retries
                    )));
                }
            }
        }

        // 5. 모든 Stage 완료 후, 코디네이터 태스크(조인 등 최종 집계)가 종료되길 기다림
        let final_results = coord_task
            .await
            .map_err(|e| DbxError::Network(format!("Coordinator thread panic: {:?}", e)))??;

        // 리소스 정리
        let keys_to_remove: Vec<_> = query_streams
            .iter()
            .filter(|k| k.key().0 == execution_id)
            .map(|k| k.key().clone())
            .collect();
        for k in keys_to_remove {
            query_streams.remove(&k);
        }

        info!(
            "Distributed execution {} finished successfully",
            execution_id
        );
        Ok(final_results)
    }
}
