//! Shard 라우터 — FNV1a 해시 기반 키→샤드 매핑

/// FNV-1a 32-bit 해시 (결정론적, 빠른 비암호학적 해시)
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// 샤드 노드 정보
#[derive(Debug, Clone, PartialEq)]
pub struct ShardNode {
    /// 고유 샤드 ID (0-based)
    pub id: usize,
    /// 연결 주소 (예: "10.0.0.1:5432")
    pub address: String,
    /// 노드 가중치 — 1.0이 기본
    /// 가중치가 높을수록 vnode가 더 많이 배정되어 더 많은 데이터를 담당
    pub weight: f64,
}

use crate::sharding::node_ring::NodeRing;

/// 샤드 라우터
///
/// 키를 받아 어느 ShardNode로 라우팅할지 결정합니다.
/// 일관된 해싱(Consistent Hashing) 링을 통해 노드 간 부하 분산을 수행합니다.
#[derive(Debug)]
pub struct ShardRouter {
    shards: Vec<ShardNode>,
    ring: NodeRing,
}

impl ShardRouter {
    /// n개의 로컬 샤드 라우터 생성 (테스트/개발용)
    pub fn new_local(n: usize) -> Self {
        let shards: Vec<ShardNode> = (0..n)
            .map(|i| ShardNode {
                id: i,
                address: format!("127.0.0.1:{}", 5000 + i),
                weight: 1.0, // 기본 가중치
            })
            .collect();

        let mut ring = NodeRing::new(100);
        for s in &shards {
            ring.add_node(s);
        }

        Self { shards, ring }
    }

    /// 커스텀 노드 목록으로 라우터 생성
    pub fn new(shards: Vec<ShardNode>) -> Self {
        use crate::sharding::node_ring::NodeRing;
        let mut ring = NodeRing::new(100);
        for s in &shards {
            ring.add_node(s);
        }
        Self { shards, ring }
    }

    /// 노드 주소 목록으로 라우터 생성
    pub fn new_with_addresses(addresses: Vec<String>) -> Self {
        let shards: Vec<ShardNode> = addresses
            .into_iter()
            .enumerate()
            .map(|(i, addr)| ShardNode {
                id: i,
                address: addr,
                weight: 1.0,
            })
            .collect();
        Self::new(shards)
    }

    /// 샤드 수
    pub fn num_shards(&self) -> usize {
        self.shards.len()
    }

    /// key 바이트로부터 물리 샤드 노드 ID 인덱스 결정 (Consistent Hashing)
    pub fn shard_index(&self, key: &[u8]) -> usize {
        if self.shards.is_empty() {
            return 0;
        }
        let hash = fnv1a_hash(key);
        self.ring.get_node(hash).unwrap_or(0)
    }

    /// key로부터 담당 ShardNode 반환
    pub fn route(&self, key: &[u8]) -> Option<&ShardNode> {
        if self.shards.is_empty() {
            return None;
        }
        let idx = self.shard_index(key);
        Some(&self.shards[idx])
    }

    /// 모든 샤드 목록 반환 (scatter-gather broadcast 용)
    pub fn all_shards(&self) -> &[ShardNode] {
        &self.shards
    }

    /// sub-table 이름 생성: `{base_table}__shard_{idx}`
    pub fn sub_table_name(&self, base_table: &str, key: &[u8]) -> String {
        let idx = self.shard_index(key);
        format!("{}__shard_{}", base_table, idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_routing_deterministic() {
        let router = ShardRouter::new_local(4);
        let idx1 = router.shard_index(b"user:42");
        let idx2 = router.shard_index(b"user:42");
        assert_eq!(idx1, idx2, "동일 키는 항상 같은 샤드");
    }

    #[test]
    fn test_shard_index_in_range() {
        let router = ShardRouter::new_local(8);
        for i in 0u64..200 {
            let key = format!("row:{}", i);
            let idx = router.shard_index(key.as_bytes());
            assert!(idx < 8, "인덱스 {}가 샤드 수 이상", idx);
        }
    }

    #[test]
    fn test_route_returns_node() {
        let router = ShardRouter::new_local(4);
        let node = router.route(b"my_key").unwrap();
        assert!(node.address.starts_with("127.0.0.1:"), "로컬 주소");
    }

    #[test]
    fn test_sub_table_name() {
        let router = ShardRouter::new_local(4);
        let name = router.sub_table_name("orders", b"order:1");
        assert!(name.starts_with("orders__shard_"));
        let idx: usize = name.split('_').last().unwrap().parse().unwrap();
        assert!(idx < 4);
    }

    #[test]
    fn test_num_shards() {
        let router = ShardRouter::new_local(6);
        assert_eq!(router.num_shards(), 6);
    }

    #[test]
    fn test_all_shards() {
        let router = ShardRouter::new_local(3);
        assert_eq!(router.all_shards().len(), 3);
    }
}
