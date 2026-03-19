//! 데이터 리밸런싱 (Data Rebalancing)
//!
//! 노드가 추가/제거될 때 영향받는 해시 범위의 데이터를 새 담당 노드로 이관합니다.
//!
//! ## 동작 원리
//! 1. `NodeRing`에서 노드 추가/제거 후 영향받는 해시 범위 계산
//! 2. `Rebalancer`는 해당 범위의 키를 이전 담당 노드에서 읽어 새 담당 노드로 이관
//! 3. 이관 완료 전까지 읽기는 old/new 양쪽 모두 시도 (double-read 전략)

use crate::sharding::node_ring::NodeRing;

/// 키 이관 작업 단위
#[derive(Debug, Clone)]
pub struct MigrationTask {
    /// 이관 대상 키
    pub key: Vec<u8>,
    /// 기존 담당 노드 ID
    pub from_node: usize,
    /// 새 담당 노드 ID
    pub to_node: usize,
}

/// 리밸런서
///
/// 노드 추가/제거 시 영향받는 키 집합을 계산하고 이관 태스크를 생성합니다.
pub struct Rebalancer<'a> {
    old_ring: &'a NodeRing,
    new_ring: &'a NodeRing,
}

impl<'a> Rebalancer<'a> {
    /// 이전 링과 새 링 비교로 리밸런서 생성
    pub fn new(old_ring: &'a NodeRing, new_ring: &'a NodeRing) -> Self {
        Self { old_ring, new_ring }
    }

    /// 주어진 키 목록 중 이관이 필요한 키를 찾아 `MigrationTask` 목록으로 반환
    ///
    /// 동일 해시에 대해 old/new 담당 노드가 다를 경우 이관 대상으로 판단합니다.
    pub fn compute_tasks(&self, keys: &[Vec<u8>]) -> Vec<MigrationTask> {
        let mut tasks = Vec::new();

        for key in keys {
            let hash = fnv1a_hash(key);
            let old_node = self.old_ring.get_node(hash);
            let new_node = self.new_ring.get_node(hash);

            match (old_node, new_node) {
                (Some(old), Some(new)) if old != new => {
                    tasks.push(MigrationTask {
                        key: key.clone(),
                        from_node: old,
                        to_node: new,
                    });
                }
                _ => {} // 담당 노드가 그대로이거나 링이 비어있는 경우 스킵
            }
        }

        tasks
    }

    /// 이관 태스크를 실행합니다.
    ///
    /// `read_fn(node_id, key)` → 데이터 조회
    /// `write_fn(node_id, key, value)` → 이관 대상에 저장
    /// `delete_fn(node_id, key)` → 이전 노드에서 삭제
    pub fn execute<R, W, D>(&self, tasks: &[MigrationTask], mut read_fn: R, mut write_fn: W, mut delete_fn: D)
    where
        R: FnMut(usize, &[u8]) -> Option<Vec<u8>>,
        W: FnMut(usize, &[u8], &[u8]),
        D: FnMut(usize, &[u8]),
    {
        for task in tasks {
            if let Some(data) = read_fn(task.from_node, &task.key) {
                write_fn(task.to_node, &task.key, &data);
                delete_fn(task.from_node, &task.key);
            }
        }
    }
}

/// FNV-1a 해시 (node_ring과 동일)
fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    const FNV_PRIME: u64 = 1_099_511_628_211;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// 새 노드 추가 시 리밸런서를 생성하는 헬퍼
pub fn rebalancer_on_add<'a>(old_ring: &'a NodeRing, new_ring: &'a NodeRing) -> Rebalancer<'a> {
    Rebalancer::new(old_ring, new_ring)
}

/// 노드 제거 시 리밸런서를 생성하는 헬퍼
pub fn rebalancer_on_remove<'a>(old_ring: &'a NodeRing, new_ring: &'a NodeRing) -> Rebalancer<'a> {
    Rebalancer::new(old_ring, new_ring)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharding::router::ShardNode;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_rebalance_on_add_node() {
        // 초기: 노드 0, 1
        let mut old_ring = NodeRing::new(50);
        old_ring.add_node(&ShardNode { id: 0, address: "A".into(), weight: 1.0 });
        old_ring.add_node(&ShardNode { id: 1, address: "B".into(), weight: 1.0 });

        // 새 노드 2 추가
        let mut new_ring = NodeRing::new(50);
        new_ring.add_node(&ShardNode { id: 0, address: "A".into(), weight: 1.0 });
        new_ring.add_node(&ShardNode { id: 1, address: "B".into(), weight: 1.0 });
        new_ring.add_node(&ShardNode { id: 2, address: "C".into(), weight: 1.0 });

        let rebalancer = Rebalancer::new(&old_ring, &new_ring);

        // 테스트용 키 목록
        let keys: Vec<Vec<u8>> = (0..100u32).map(|i| format!("key-{}", i).into_bytes()).collect();
        let tasks = rebalancer.compute_tasks(&keys);

        // 노드가 추가되었으므로 일부 키는 이관 대상이어야 함
        assert!(!tasks.is_empty(), "노드 추가 시 이관 태스크가 있어야 함");

        // 이관 태스크는 to_node = 2 (새 노드)여야 함
        for task in &tasks {
            assert_eq!(task.to_node, 2, "새 노드(2)로 이관되어야 함");
        }
    }

    #[test]
    fn test_rebalance_execute() {
        let mut old_ring = NodeRing::new(50);
        old_ring.add_node(&ShardNode { id: 0, address: "A".into(), weight: 1.0 });
        old_ring.add_node(&ShardNode { id: 1, address: "B".into(), weight: 1.0 });

        let mut new_ring = NodeRing::new(50);
        new_ring.add_node(&ShardNode { id: 0, address: "A".into(), weight: 1.0 });
        new_ring.add_node(&ShardNode { id: 1, address: "B".into(), weight: 1.0 });
        new_ring.add_node(&ShardNode { id: 2, address: "C".into(), weight: 1.0 });

        // 시뮬레이션 스토어 (노드 ID → 키-값 맵)
        let store: Arc<Mutex<HashMap<(usize, Vec<u8>), Vec<u8>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // 초기 데이터 삽입
        let keys: Vec<Vec<u8>> = (0..20u32).map(|i| format!("key-{}", i).into_bytes()).collect();
        for key in &keys {
            let hash = fnv1a_hash(key);
            let node = old_ring.get_node(hash).unwrap();
            store.lock().unwrap().insert((node, key.clone()), b"value".to_vec());
        }

        let rebalancer = Rebalancer::new(&old_ring, &new_ring);
        let tasks = rebalancer.compute_tasks(&keys);

        let store_r = Arc::clone(&store);
        let store_w = Arc::clone(&store);
        let store_d = Arc::clone(&store);

        rebalancer.execute(
            &tasks,
            move |node_id, key| store_r.lock().unwrap().get(&(node_id, key.to_vec())).cloned(),
            move |node_id, key, value| {
                store_w.lock().unwrap().insert((node_id, key.to_vec()), value.to_vec());
            },
            move |node_id, key| {
                store_d.lock().unwrap().remove(&(node_id, key.to_vec()));
            },
        );

        // 이관된 키들은 이제 새 담당 노드에 있어야 함
        let snapshot = store.lock().unwrap();
        for task in &tasks {
            assert!(
                snapshot.contains_key(&(task.to_node, task.key.clone())),
                "이관된 키는 새 노드에 있어야 함"
            );
            assert!(
                !snapshot.contains_key(&(task.from_node, task.key.clone())),
                "이관된 키는 이전 노드에서 삭제되어야 함"
            );
        }
    }

    #[test]
    fn test_no_migration_same_ring() {
        let mut ring = NodeRing::new(50);
        ring.add_node(&ShardNode { id: 0, address: "A".into(), weight: 1.0 });
        ring.add_node(&ShardNode { id: 1, address: "B".into(), weight: 1.0 });

        let rebalancer = Rebalancer::new(&ring, &ring);
        let keys: Vec<Vec<u8>> = (0..50u32).map(|i| format!("k-{}", i).into_bytes()).collect();
        let tasks = rebalancer.compute_tasks(&keys);
        assert!(tasks.is_empty(), "동일한 링이면 이관 없음");
    }
}
