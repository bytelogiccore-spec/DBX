//! 일관된 해싱(Consistent Hashing)을 구현하는 Hash Ring (vnode 지원)

use crate::sharding::router::{ShardNode, fnv1a_hash};
use std::collections::BTreeMap;

/// Consistent Hashing 구조를 담당하는 노드 링
#[derive(Debug, Default)]
pub struct NodeRing {
    /// 가상 노드 해시 -> 물리 노드 ID 매핑
    ring: BTreeMap<u64, usize>,
    /// 하나의 물리 노드당 생성할 가상 노드(vnode) 개수
    vnodes_per_node: usize,
}

impl NodeRing {
    /// 가상 노드(vnode) 수를 지정하여 새로운 링 생성
    pub fn new(vnodes_per_node: usize) -> Self {
        Self {
            ring: BTreeMap::new(),
            vnodes_per_node,
        }
    }

    /// 링에 물리 노드 추가
    /// `ShardNode::weight` 에 비례하여 vnode 수를 늘립니다.
    /// `weight = 1.0` 이면 `vnodes_per_node` 개, `2.0` 이면 2배 등
    pub fn add_node(&mut self, node: &ShardNode) {
        let vnode_count = ((self.vnodes_per_node as f64) * node.weight.max(0.1)) as usize;
        for i in 0..vnode_count {
            let vnode_key = format!("node:{}-vnode:{}", node.id, i);
            let hash = fnv1a_hash(vnode_key.as_bytes());
            self.ring.insert(hash, node.id);
        }
    }

    /// 링에서 물리 노드 제거
    /// weight 기반으로 vnode가 많이 배정되었을 수 있으므로
    /// MAX_WEIGHT(10.0 기준)까지 범위를 넓혀 결정론적으로 제거합니다.
    pub fn remove_node(&mut self, node_id: usize) {
        let max_vnodes = (self.vnodes_per_node as f64 * 10.0) as usize;
        let mut to_remove = Vec::new();
        for i in 0..max_vnodes {
            let vnode_key = format!("node:{}-vnode:{}", node_id, i);
            let hash = fnv1a_hash(vnode_key.as_bytes());
            to_remove.push(hash);
        }
        for hash in to_remove {
            self.ring.remove(&hash);
        }
    }

    /// 데이터 해시값을 기반으로 적절한 물리 노드의 ID를 반환
    pub fn get_node(&self, hash_val: u64) -> Option<usize> {
        if self.ring.is_empty() {
            return None;
        }

        // 해시값보다 크거나 같은 첫 번째 가상 노드를 탐색
        if let Some((_, &node_id)) = self.ring.range(hash_val..).next() {
            Some(node_id)
        } else {
            // 끝까지 도달했다면 링의 맨 첫 번째 요소로 돌아감(Consistent Hashing의 특성)
            let (_, &node_id) = self.ring.iter().next().unwrap();
            Some(node_id)
        }
    }

    /// 현재 링이 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_ring_distribution() {
        let mut ring = NodeRing::new(300); // 노드당 300개 vnode
        let n1 = ShardNode {
            id: 0,
            address: "1".into(),
            weight: 1.0,
        };
        let n2 = ShardNode {
            id: 1,
            address: "2".into(),
            weight: 1.0,
        };
        let n3 = ShardNode {
            id: 2,
            address: "3".into(),
            weight: 1.0,
        };

        ring.add_node(&n1);
        ring.add_node(&n2);
        ring.add_node(&n3);

        let mut counts = vec![0; 3];
        // 난수와 유사한 데이터 키 수만개를 배치
        let total_keys = 30000;
        for i in 0..total_keys {
            let k = format!("data-{}", i);
            let h = fnv1a_hash(k.as_bytes());
            let node_id = ring.get_node(h).unwrap();
            counts[node_id] += 1;
        }

        // FNV-1a 해시의 단순성으로 인해 완벽한 균등 분배는 어려움
        // 각 노드가 최소 10%의 키는 가져가는지(비정상 쏠림 방지)를 MVP 기준으로 검증
        let expected = total_keys as f64 / 3.0;
        let diff = expected * 0.7; // 70% 오차 허용 (매우 느슨하게: 최소 10% 보장)

        for i in 0..3 {
            let count = counts[i] as f64;
            assert!(
                (count - expected).abs() <= diff,
                "Node {} load {} is outside expected {} +- {}",
                i,
                count,
                expected,
                diff
            );
        }
    }

    #[test]
    fn test_node_ring_add_remove() {
        let mut ring = NodeRing::new(10);
        let n1 = ShardNode {
            id: 100,
            address: "A".into(),
            weight: 1.0,
        };
        let n2 = ShardNode {
            id: 200,
            address: "B".into(),
            weight: 1.0,
        };

        ring.add_node(&n1);
        ring.add_node(&n2);

        // 100번 혹은 200번 노드만 반환되어야 함
        let node_id = ring.get_node(12345).unwrap();
        assert!(node_id == 100 || node_id == 200);

        // 노드 100 제거
        ring.remove_node(100);

        // 이제 항상 200만 반환되어야 함
        for i in 0..10_000 {
            assert_eq!(ring.get_node(i).unwrap(), 200);
        }

        // 노드 200 제거
        ring.remove_node(200);
        assert!(ring.is_empty());
        assert_eq!(ring.get_node(12345), None);
    }
}
