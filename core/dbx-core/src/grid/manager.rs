use crate::error::{DbxError, DbxResult};
use crate::grid::protocol::{GridMessage, QueryMessage, StorageMessage};
use crate::grid::quic::{GridMessageWrapper, QuicChannel};
use crate::sql::executor::local_executor::LocalExecutor;
use crate::sql::planner::types::PhysicalPlan;
use crate::storage::erasure_coding::distributed_store::DistributedErasureCodingStore;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

/// 그리드 중앙 제어기 (Thin Dispatcher)
///
/// 네트워크 채널로부터 오는 메시지를 분류하여 도메인별 핸들러로 배달합니다.
pub struct GridManager {
    quic_channel: Arc<QuicChannel>,
    ec_store: Arc<DistributedErasureCodingStore>,
    receiver: mpsc::Receiver<GridMessageWrapper>,
    query_streams: Arc<DashMap<(String, usize), mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
    stage_barriers: Arc<DashMap<(String, usize, std::net::SocketAddr), mpsc::Sender<()>>>,
    /// 워커 측 로컬 실행 엔진 (워커 노드일 때 사용)
    local_executor: Option<Arc<LocalExecutor>>,
    /// 이 노드의 식별자
    node_id: u32,
}

impl GridManager {
    pub fn new(
        quic_channel: Arc<QuicChannel>,
        ec_store: Arc<DistributedErasureCodingStore>,
        receiver: mpsc::Receiver<GridMessageWrapper>,
    ) -> Self {
        Self::with_node_id(quic_channel, ec_store, receiver, 0)
    }

    pub fn with_node_id(
        quic_channel: Arc<QuicChannel>,
        ec_store: Arc<DistributedErasureCodingStore>,
        receiver: mpsc::Receiver<GridMessageWrapper>,
        node_id: u32,
    ) -> Self {
        Self {
            quic_channel,
            ec_store,
            receiver,
            query_streams: Arc::new(DashMap::new()),
            stage_barriers: Arc::new(DashMap::new()),
            local_executor: None,
            node_id,
        }
    }

    /// 워커 노드 로컬 실행기 설정
    pub fn with_local_executor(mut self, executor: Arc<LocalExecutor>) -> Self {
        self.local_executor = Some(executor);
        self
    }

    pub fn get_query_streams(
        &self,
    ) -> Arc<DashMap<(String, usize), mpsc::Sender<DbxResult<Option<Vec<u8>>>>>> {
        Arc::clone(&self.query_streams)
    }

    pub fn get_stage_barriers(
        &self,
    ) -> Arc<DashMap<(String, usize, std::net::SocketAddr), mpsc::Sender<()>>> {
        Arc::clone(&self.stage_barriers)
    }

    /// 수신 루프 시작
    pub async fn run(mut self) {
        info!(
            "GridManager receiver loop started on {}",
            self.quic_channel.local_addr
        );

        while let Some(wrapper) = self.receiver.recv().await {
            let ec_store = Arc::clone(&self.ec_store);
            let query_streams = Arc::clone(&self.query_streams);
            let stage_barriers = Arc::clone(&self.stage_barriers);
            let quic_channel = Arc::clone(&self.quic_channel);
            let local_executor = self.local_executor.clone();
            let node_id = self.node_id;

            // 각 메시지를 비동기적으로 처리하여 병목 방지
            tokio::spawn(async move {
                if let Err(e) = Self::handle_message(
                    ec_store,
                    query_streams,
                    stage_barriers,
                    quic_channel,
                    local_executor,
                    node_id,
                    wrapper,
                )
                .await
                {
                    error!("Error handling GridMessage: {:?}", e);
                }
            });
        }

        info!("GridManager receiver loop terminated");
    }

    /// 메시지 종류별 분기 처리
    async fn handle_message(
        ec_store: Arc<DistributedErasureCodingStore>,
        query_streams: Arc<DashMap<(String, usize), mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
        stage_barriers: Arc<DashMap<(String, usize, std::net::SocketAddr), mpsc::Sender<()>>>,
        quic_channel: Arc<QuicChannel>,
        local_executor: Option<Arc<LocalExecutor>>,
        node_id: u32,
        wrapper: GridMessageWrapper,
    ) -> DbxResult<()> {
        let GridMessageWrapper { msg, mut stream } = wrapper;

        match msg {
            GridMessage::Storage(storage_msg) => {
                Self::handle_storage_message(ec_store, storage_msg, &mut stream).await
            }
            GridMessage::Query(query_msg) => {
                Self::handle_query_message(
                    query_streams,
                    stage_barriers,
                    quic_channel,
                    local_executor,
                    node_id,
                    query_msg,
                )
                .await
            }
            GridMessage::Lock(_) => {
                warn!("LockMessage received but not implemented yet");
                Ok(())
            }
            GridMessage::Replication(_) => {
                warn!("ReplicationMessage received but not implemented yet");
                Ok(())
            }
        }
    }

    /// 쿼리(스트리밍) 메시지 처리
    async fn handle_query_message(
        query_streams: Arc<DashMap<(String, usize), mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
        stage_barriers: Arc<DashMap<(String, usize, std::net::SocketAddr), mpsc::Sender<()>>>,
        quic_channel: Arc<QuicChannel>,
        local_executor: Option<Arc<LocalExecutor>>,
        node_id: u32,
        msg: QueryMessage,
    ) -> DbxResult<()> {
        match msg {
            QueryMessage::ExecuteFragment {
                execution_id,
                stage_id,
                plans_bytes,
                coordinator_addr,
            } => {
                info!(
                    "ExecuteFragment received for ID: {}, Stage: {} from coordinator: {}",
                    execution_id, stage_id, coordinator_addr
                );

                let executor = match local_executor {
                    Some(e) => e,
                    None => {
                        warn!(
                            "ExecuteFragment received but no LocalExecutor configured — ignoring"
                        );
                        return Ok(());
                    }
                };

                // 코디네이터 주소 파싱
                let coord_addr: std::net::SocketAddr = match coordinator_addr.parse() {
                    Ok(a) => a,
                    Err(e) => {
                        return Err(DbxError::Network(format!(
                            "Invalid coordinator addr: {}",
                            e
                        )));
                    }
                };

                let exec_id = execution_id.clone();
                let query_streams = Arc::clone(&query_streams);

                // 역직렬화
                let mut plans: Vec<PhysicalPlan> = Vec::new();
                for bytes in plans_bytes {
                    let plan = bincode::deserialize(&bytes)
                        .map_err(|e| DbxError::Serialization(e.to_string()))?;
                    plans.push(plan);
                }

                let quic_master = Arc::clone(&quic_channel);
                tokio::spawn(async move {
                    info!(
                        "Worker spawning execution for exec_id: {}, stage_id: {}",
                        exec_id, stage_id
                    );

                    let mut join_set = tokio::task::JoinSet::new();

                    for worker_plan in plans {
                        let executor_ref = Arc::clone(&executor);
                        let quic_ref = Arc::clone(&quic_master);
                        let exec_id_ref = exec_id.clone();
                        let q_streams = Arc::clone(&query_streams);

                        join_set.spawn(async move {
                            // 1. CPU-bound 쿼리 실행을 spawn_blocking 기반으로 넘김
                            let (batches, channels) = match tokio::task::spawn_blocking(move || {
                                let mut chs = crate::sql::executor::local_executor::DistributedChannels::default();
                                let b = executor_ref.execute_collect_distributed(&worker_plan, &mut chs)?;
                                Ok::<(Vec<arrow::array::RecordBatch>, _), DbxError>((b, chs))
                            }).await {
                                Ok(Ok(res)) => res,
                                Ok(Err(e)) => {
                                    error!("Worker execution error for {}: {:?}", exec_id_ref, e);
                                    let eof_msg = GridMessage::Query(QueryMessage::ExchangeData {
                                        execution_id: exec_id_ref.clone(),
                                        exchange_id: 0,
                                        node_id,
                                        is_eof: true,
                                        batch_data: vec![],
                                    });
                                    let _ = quic_ref.send_message(coord_addr, eof_msg).await;
                                    return;
                                }
                                Err(e) => {
                                    error!("Worker spawn_blocking panic: {:?}", e);
                                    return;
                                }
                            };

                            // 2. 수동생성된 수신 채널(tx)들을 DashMap에 등록 (GridExchange 용도)
                            for (e_id, tx) in channels.exchanges {
                                q_streams.insert((exec_id_ref.clone(), e_id), tx);
                            }

                            // 3. ShuffleWriter 발신 채널(rx)들을 타겟별로 묶어 송신 태스크 스폰
                            let mut shuffle_join_set = tokio::task::JoinSet::new();
                            for (e_id, receivers) in channels.shuffles {
                                for (target_addr, mut rx) in receivers {
                                    let quic_sub = Arc::clone(&quic_ref);
                                    let exec_sub = exec_id_ref.clone();
                                    shuffle_join_set.spawn(async move {
                                        while let Some(Ok(Some(batch_bytes))) = rx.recv().await {
                                            let msg = GridMessage::Query(QueryMessage::ExchangeData {
                                                execution_id: exec_sub.clone(),
                                                exchange_id: e_id,
                                                node_id,
                                                is_eof: false,
                                                batch_data: batch_bytes,
                                            });
                                            let _ = quic_sub.send_message(target_addr, msg).await;
                                        }
                                        let eof_msg = GridMessage::Query(QueryMessage::ExchangeData {
                                            execution_id: exec_sub,
                                            exchange_id: e_id,
                                            node_id,
                                            is_eof: true,
                                            batch_data: vec![],
                                        });
                                        let _ = quic_sub.send_message(target_addr, eof_msg).await;
                                    });
                                }
                            }

                            // 4. (분산 Agg 등) 최상위 Return RecordBatch들을 스트리밍 송신
                            for batch in batches {
                                let ipc_bytes = match crate::grid::protocol::serialize_batch_to_ipc(&batch) {
                                    Ok(b) => b,
                                    Err(_) => continue,
                                };
                                let msg = GridMessage::Query(QueryMessage::ExchangeData {
                                    execution_id: exec_id_ref.clone(),
                                    exchange_id: 0,
                                    node_id,
                                    is_eof: false,
                                    batch_data: ipc_bytes,
                                });
                                let _ = quic_ref.send_message(coord_addr, msg).await;
                            }

                            // EOF
                            let _ = quic_ref.send_message(coord_addr, GridMessage::Query(QueryMessage::ExchangeData {
                                execution_id: exec_id_ref.clone(),
                                exchange_id: 0,
                                node_id,
                                is_eof: true,
                                batch_data: vec![],
                            })).await;

                            // 셔플 백그라운드 송출 태스크들이 다 이빨이 맞을 때까지 대기
                            while let Some(_) = shuffle_join_set.join_next().await {}
                        });
                    }

                    // 모든 Plan 태스크들 처리가 끝날 때까지 대기
                    while let Some(_) = join_set.join_next().await {}

                    // 코디네이터에게 해당 Stage의 모든 수행이 종료되었음을 알림
                    let complete_msg = GridMessage::Query(QueryMessage::FragmentCompleted {
                        execution_id: exec_id.clone(),
                        stage_id,
                    });
                    let _ = quic_master.send_message(coord_addr, complete_msg).await;
                    info!(
                        "Worker completed all plans for exec_id: {}, stage_id: {}",
                        exec_id, stage_id
                    );
                });

                Ok(())
            }
            QueryMessage::FragmentCompleted {
                execution_id,
                stage_id,
            } => {
                let barriers = stage_barriers;
                let matched_keys: Vec<_> = barriers
                    .iter()
                    .filter(|entry| entry.key().0 == execution_id && entry.key().1 == stage_id)
                    .map(|entry| entry.key().clone())
                    .collect();

                for key in matched_keys {
                    if let Some(sender) = barriers.get_mut(&key) {
                        let _ = sender.try_send(());
                    }
                }
                Ok(())
            }
            QueryMessage::ExchangeData {
                execution_id,
                exchange_id,
                node_id: _,
                is_eof,
                batch_data,
            } => {
                // 코디네이터가 큐를 통해 Operator로 데이터 밀어넣기
                // DashMap lock을 await 전에 해제하기 위해 Sender를 복제합니다.
                let sender_opt = query_streams
                    .get(&(execution_id.clone(), exchange_id))
                    .map(|kv| kv.value().clone());

                if let Some(sender) = sender_opt {
                    if is_eof {
                        let _ = sender.send(Ok(None)).await;
                    } else {
                        let _ = sender.send(Ok(Some(batch_data))).await;
                    }
                } else {
                    warn!("ExchangeData for unknown execution_id: {}", execution_id);
                }
                Ok(())
            }
        }
    }

    /// 스토리지(EC 샤드) 메시지 처리
    async fn handle_storage_message(
        ec_store: Arc<DistributedErasureCodingStore>,
        msg: StorageMessage,
        stream: &mut Option<s2n_quic::stream::BidirectionalStream>,
    ) -> DbxResult<()> {
        match msg {
            StorageMessage::StoreShard {
                key,
                shard_id,
                data,
            } => {
                info!("Storing shard {}:{} locally", key, shard_id);
                ec_store.local_store_shard(&key, shard_id, &data)?;
                Ok(())
            }
            StorageMessage::FetchShard { key, shard_id } => {
                info!("Fetching shard {}:{} for remote request", key, shard_id);
                let shard_data = ec_store.local_fetch_shard(&key, shard_id)?;

                // 응답 전송
                if let Some(s) = stream {
                    let reply = GridMessage::Storage(StorageMessage::ShardResponse {
                        key: key.clone(),
                        shard_id,
                        data: shard_data,
                    });
                    ::tracing::debug!(
                        "Sending ShardResponse for {}:{} on stream...",
                        key,
                        shard_id
                    );
                    if let Err(e) = QuicChannel::send_response(s, reply).await {
                        ::tracing::error!(
                            "Failed to send ShardResponse for {}:{}: {:?}",
                            key,
                            shard_id,
                            e
                        );
                    } else {
                        ::tracing::debug!(
                            "Successfully sent ShardResponse for {}:{}",
                            key,
                            shard_id
                        );
                    }
                }
                Ok(())
            }
            StorageMessage::ShardResponse { .. } => {
                // ShardResponse는 보통 send_request_and_wait에서 직접 받으므로
                // 메인 루프에 도달했다면 무시하거나 에러 처리
                warn!("Received unexpected ShardResponse in main handler loop");
                Ok(())
            }
        }
    }
}
