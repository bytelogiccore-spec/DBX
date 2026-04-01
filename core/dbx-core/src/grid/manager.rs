use crate::error::DbxResult;
use crate::grid::quic::{QuicChannel, GridMessageWrapper};
use crate::grid::protocol::{GridMessage, StorageMessage, QueryMessage};
use crate::storage::erasure_coding::distributed_store::DistributedErasureCodingStore;
use tokio::sync::mpsc;
use tracing::{info, error, warn};
use std::sync::Arc;
use dashmap::DashMap;

/// 그리드 중앙 제어기 (Thin Dispatcher)
/// 
/// 네트워크 채널로부터 오는 메시지를 분류하여 도메인별 핸들러로 배달합니다.
pub struct GridManager {
    quic_channel: Arc<QuicChannel>,
    ec_store: Arc<DistributedErasureCodingStore>,
    receiver: mpsc::Receiver<GridMessageWrapper>,
    query_streams: Arc<DashMap<String, mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
}

impl GridManager {
    pub fn new(
        quic_channel: Arc<QuicChannel>,
        ec_store: Arc<DistributedErasureCodingStore>,
        receiver: mpsc::Receiver<GridMessageWrapper>,
    ) -> Self {
        Self {
            quic_channel,
            ec_store,
            receiver,
            query_streams: Arc::new(DashMap::new()),
        }
    }
    
    pub fn get_query_streams(&self) -> Arc<DashMap<String, mpsc::Sender<DbxResult<Option<Vec<u8>>>>>> {
        Arc::clone(&self.query_streams)
    }

    /// 수신 루프 시작
    pub async fn run(mut self) {
        info!("GridManager receiver loop started on {}", self.quic_channel.local_addr);
        
        while let Some(wrapper) = self.receiver.recv().await {
            let ec_store = Arc::clone(&self.ec_store);
            let query_streams = Arc::clone(&self.query_streams);
            
            // 각 메시지를 비동기적으로 처리하여 병목 방지
            tokio::spawn(async move {
                if let Err(e) = Self::handle_message(ec_store, query_streams, wrapper).await {
                    error!("Error handling GridMessage: {:?}", e);
                }
            });
        }
        
        info!("GridManager receiver loop terminated");
    }

    /// 메시지 종류별 분기 처리
    async fn handle_message(
        ec_store: Arc<DistributedErasureCodingStore>,
        query_streams: Arc<DashMap<String, mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
        wrapper: GridMessageWrapper,
    ) -> DbxResult<()> {
        let GridMessageWrapper { msg, mut stream } = wrapper;
        
        match msg {
            GridMessage::Storage(storage_msg) => {
                Self::handle_storage_message(ec_store, storage_msg, &mut stream).await
            }
            GridMessage::Query(query_msg) => {
                Self::handle_query_message(query_streams, query_msg).await
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
        query_streams: Arc<DashMap<String, mpsc::Sender<DbxResult<Option<Vec<u8>>>>>>,
        msg: QueryMessage,
    ) -> DbxResult<()> {
        match msg {
            QueryMessage::ExecuteFragment { execution_id, plan_json: _ } => {
                info!("ExecuteFragment received for ID: {}", execution_id);
                // 모의 테스트에서는 워커가 직접 데이터를 쏘므로 여기선 무시.
                Ok(())
            }
            QueryMessage::ExchangeData { execution_id, is_eof, batch_data } => {
                // 코디네이터가 큐를 통해 Operator로 데이터 밀어넣기
                // DashMap lock을 await 전에 해제하기 위해 Sender를 복제합니다.
                let sender_opt = query_streams.get(&execution_id).map(|kv| kv.value().clone());
                
                if let Some(sender) = sender_opt {
                    if is_eof {
                        let _ = sender.send(Ok(None)).await;
                    } else {
                        // 큐가 가득차면 여기서 await 걸림 -> Task 정지 -> QUIC 수신 스레드는 계속 돌지만...
                        // 실제로는 여러 스트림이 계속 배압을 유발해 궁극적으로 Backpressure 달성.
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
            StorageMessage::StoreShard { key, shard_id, data } => {
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
                    ::tracing::debug!("Sending ShardResponse for {}:{} on stream...", key, shard_id);
                    if let Err(e) = QuicChannel::send_response(s, reply).await {
                        ::tracing::error!("Failed to send ShardResponse for {}:{}: {:?}", key, shard_id, e);
                    } else {
                        ::tracing::debug!("Successfully sent ShardResponse for {}:{}", key, shard_id);
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
