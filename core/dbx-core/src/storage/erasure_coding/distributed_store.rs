//! Grid Distributed Erasure Coding Store
//!
//! Provides a high-level API to store and retrieve data with erasure coding
//! across multiple nodes in the Grid.

use crate::error::DbxResult;
use crate::grid::protocol::{GridMessage, StorageMessage};
use crate::grid::quic::QuicChannel;
use crate::sharding::router::ShardRouter;
use crate::storage::erasure_coding::store::ErasureCodingStore;
use futures::stream::{FuturesUnordered, StreamExt};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, warn};

/// 그리드 분산 Erasure Coding 스토어
///
/// 로컬 ErasureCodingStore와 ShardRouter를 결합하여 데이터를 분산 저장합니다.
pub struct DistributedErasureCodingStore {
    local_store: ErasureCodingStore,
    router: Arc<ShardRouter>,
    quic_channel: Option<Arc<QuicChannel>>,
}

impl DistributedErasureCodingStore {
    pub fn new(
        local_store: ErasureCodingStore,
        router: Arc<ShardRouter>,
        quic_channel: Option<Arc<QuicChannel>>,
    ) -> Self {
        Self {
            local_store,
            router,
            quic_channel,
        }
    }

    /// 데이터를 인코딩하여 그리드 노드들에 분산 저장 요청
    pub async fn store(&self, key: &str, data: &[u8]) -> DbxResult<()> {
        let mut buffer = Vec::with_capacity(8 + data.len());
        buffer.extend_from_slice(&(data.len() as u64).to_be_bytes());
        buffer.extend_from_slice(data);

        // 1. EC 인코딩 (K 데이터 + M 패리티)
        let shards = self.local_store.encode(&buffer)?;

        let mut futures = FuturesUnordered::new();

        // 2. 샤드별 대상 노드 결정 및 전송
        for (i, shard) in shards.iter().enumerate() {
            let shard_key = format!("{}:shard_{}", key, i);

            // 만약 quic_channel이 존재한다면 원격 노드로 전송
            if let Some(channel) = &self.quic_channel {
                if let Some(node) = self.router.route(shard_key.as_bytes()) {
                    if let Ok(addr) = SocketAddr::from_str(&node.address) {
                        let msg = GridMessage::Storage(StorageMessage::StoreShard {
                            key: key.to_string(),
                            shard_id: i,
                            data: shard.clone(),
                        });
                        let ch = channel.clone();
                        futures.push(tokio::spawn(async move {
                            if let Err(e) = ch.send_message(addr, msg).await {
                                error!(?e, "Failed to send shard {} to {}", i, addr);
                            }
                        }));
                    }
                }
            }
        }

        // 백그라운 전송 대기
        while let Some(_) = futures.next().await {}

        // 3. 현재 노드의 로컬 스토리지에도 샤드 기록 (장애 복구 보장용 혹은 로컬 캐싱)
        self.local_store.store_shards(key, &shards, buffer.len())?;

        Ok(())
    }

    /// 그리드 노드들로부터 샤드들을 수집하여 데이터 복구 (빠른 K개 수집)
    pub async fn retrieve(&self, key: &str) -> DbxResult<Option<Vec<u8>>> {
        // 로컬 조회 시도 (가장 빠름)
        if let Ok(Some(data)) = self.local_store.retrieve_and_decode(key) {
            if data.len() >= 8 {
                let mut len_bytes = [0u8; 8];
                len_bytes.copy_from_slice(&data[0..8]);
                let original_len = u64::from_be_bytes(len_bytes) as usize;
                if 8 + original_len <= data.len() {
                    return Ok(Some(data[8..8 + original_len].to_vec()));
                }
            }
            return Ok(Some(data));
        }

        // 로컬 실패 시, QuicChannel이 없으면 복구 불가
        let channel = match &self.quic_channel {
            Some(ch) => ch,
            None => return Ok(None),
        };

        // K+M 전체 샤드 개수는 ErasureCodingStore 설정값에 의존
        // ShardRouter를 통해 0..N(충분히 큰 임의의 샤드 개수)까지 FetchShard를 Broadcast
        // (현재 EC 구현에서는 샤드 개수를 알기 어려움, 다만 ErasureCodingStore가 (data+parity)를 알고 있음)
        let total_shards = self.local_store.k() + self.local_store.m();
        let mut futures = FuturesUnordered::new();

        for i in 0..total_shards {
            let shard_key = format!("{}:shard_{}", key, i);
            if let Some(node) = self.router.route(shard_key.as_bytes()) {
                if let Ok(addr) = SocketAddr::from_str(&node.address) {
                    let msg = GridMessage::Storage(StorageMessage::FetchShard {
                        key: key.to_string(),
                        shard_id: i,
                    });
                    let ch = channel.clone();

                    futures.push(tokio::spawn(async move {
                        match ch.send_request_and_wait(addr, msg).await {
                            Ok(GridMessage::Storage(StorageMessage::ShardResponse {
                                shard_id,
                                data: Some(shard_data),
                                ..
                            })) => Some((shard_id, shard_data)),
                            Ok(other) => {
                                warn!(?other, "Unexpected response for shard {}", i);
                                None
                            }
                            Err(e) => {
                                error!(?e, "Error fetching shard {} from {}", i, addr);
                                None
                            }
                        }
                    }));
                }
            }
        }

        let mut collected_shards: std::collections::HashMap<usize, Vec<u8>> =
            std::collections::HashMap::new();
        let target_k = self.local_store.k();

        // 도착하는 순서대로 K개를 모음
        while let Some(res) = futures.next().await {
            if let Ok(Some((id, data))) = res {
                collected_shards.insert(id, data);
                if collected_shards.len() >= target_k {
                    // K개를 모았으므로 복구 시도
                    // collected_shards를 Option 배열 꼴로 맞춰서 decode_missing
                    break;
                }
            }
        }

        if collected_shards.len() >= target_k {
            let mut shards_opt: Vec<Option<Vec<u8>>> = vec![None; total_shards];
            for (id, data) in collected_shards {
                if id < total_shards {
                    shards_opt[id] = Some(data);
                }
            }
            // missing 복원 (메타데이터 8바이트 매직헤더 활용)
            if let Ok(restored) = self.local_store.decode(shards_opt, std::usize::MAX) {
                if restored.len() >= 8 {
                    let mut len_bytes = [0u8; 8];
                    len_bytes.copy_from_slice(&restored[0..8]);
                    let original_len = u64::from_be_bytes(len_bytes) as usize;
                    if 8 + original_len <= restored.len() {
                        return Ok(Some(restored[8..8 + original_len].to_vec()));
                    }
                }
                return Ok(Some(restored));
            }
        }

        Ok(None)
    }

    /// EC 데이터 삭제
    pub async fn delete(&self, key: &str) -> DbxResult<bool> {
        // 모든 노드에 DeleteShard 요청 전송 확장 가능
        self.local_store.delete(key)
    }

    /// 로컬 스토어에 단일 샤드 기록 (GridManager에서 호출)
    pub fn local_store_shard(&self, key: &str, shard_id: usize, data: &[u8]) -> DbxResult<()> {
        self.local_store.store_shard(key, shard_id, data)
    }

    /// 로컬 스토어에서 단일 샤드 조회 (GridManager에서 호출)
    pub fn local_fetch_shard(&self, key: &str, shard_id: usize) -> DbxResult<Option<Vec<u8>>> {
        self.local_store.fetch_shard(key, shard_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharding::router::ShardRouter;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_distributed_store_basic() {
        let dir = tempdir().unwrap();
        let local_store = ErasureCodingStore::new(dir.path(), 4, 2);
        let router = Arc::new(ShardRouter::new_local(4)); // 4 nodes simulation

        let d_store = DistributedErasureCodingStore::new(local_store, router, None);
        let key = "distributed_obj_1";
        let data = b"Distributed Erasure Coding on Grid Test Data";

        // 저장
        d_store.store(key, data).await.unwrap();

        // 조회
        let recovered = d_store.retrieve(key).await.unwrap().unwrap();
        assert_eq!(recovered, data);
    }

    #[tokio::test]
    async fn test_distributed_store_shard_routing() {
        let dir = tempdir().unwrap();
        // data shards = 4, parity = 2 -> total 6 shards
        let local_store = ErasureCodingStore::new(dir.path(), 4, 2);
        // Simulate a 10-node Grid cluster
        let router = Arc::new(ShardRouter::new_local(10));

        let key = "routing_test_obj";

        let mut target_nodes = std::collections::HashSet::new();
        // Simulate routing of 100 shards
        for i in 0..100 {
            let shard_key = format!("{}:shard_{}", key, i);
            let node_idx = router.shard_index(shard_key.as_bytes());
            target_nodes.insert(node_idx);
        }

        // uniform hashing should map 100 shards to multiple distinct nodes
        assert!(
            target_nodes.len() > 1,
            "Shards should be distributed across multiple nodes"
        );
    }
}
