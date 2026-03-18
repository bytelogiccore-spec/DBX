//! Scatter-Gather 실행기
//!
//! 분산 쿼리를 모든 샤드에 분산(scatter)하고, 결과를 병합(gather)합니다.
//!
//! # MVP 범위
//!
//! - 인메모리 시뮬레이션 (실제 네트워크 없음)
//! - `Vec<(key, value)>` 레벨에서 scatter/gather

use crate::sharding::router::ShardRouter;

/// Scatter-Gather 실행기
pub struct ScatterGather<'a> {
    router: &'a ShardRouter,
}

impl<'a> ScatterGather<'a> {
    pub fn new(router: &'a ShardRouter) -> Self {
        Self { router }
    }

    /// 단일 키를 적절한 샤드로 scatter
    ///
    /// `handler(shard_id, key, value)` 를 해당 샤드에서 실행
    pub fn scatter_write<F>(&self, key: &[u8], value: &[u8], handler: F)
    where
        F: Fn(usize, &[u8], &[u8]),
    {
        let idx = self.router.shard_index(key);
        handler(idx, key, value);
    }

    /// 단일 키 읽기 — 담당 샤드에서 조회
    ///
    /// `handler(shard_id, key)` 로부터 Option<Vec<u8>> 반환
    pub fn scatter_read<F>(&self, key: &[u8], handler: F) -> Option<Vec<u8>>
    where
        F: Fn(usize, &[u8]) -> Option<Vec<u8>>,
    {
        let idx = self.router.shard_index(key);
        handler(idx, key)
    }

    /// 모든 샤드에 스캔 요청 후 결과 병합 (gather)
    ///
    /// `handler(shard_id)` → `Vec<(Vec<u8>, Vec<u8>)>` (key, value pairs)
    pub fn gather<F>(&self, handler: F) -> Vec<(Vec<u8>, Vec<u8>)>
    where
        F: Fn(usize) -> Vec<(Vec<u8>, Vec<u8>)>,
    {
        let mut results = Vec::new();
        for shard in self.router.all_shards() {
            let mut shard_results = handler(shard.id);
            results.append(&mut shard_results);
        }
        results
    }

    /// 각 샤드 결과를 정렬된 순서로 gather (ORDER BY key 시뮬레이션)
    pub fn gather_sorted<F>(&self, handler: F) -> Vec<(Vec<u8>, Vec<u8>)>
    where
        F: Fn(usize) -> Vec<(Vec<u8>, Vec<u8>)>,
    {
        let mut results = self.gather(handler);
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn make_store(n: usize) -> Vec<Mutex<HashMap<Vec<u8>, Vec<u8>>>> {
        (0..n).map(|_| Mutex::new(HashMap::new())).collect()
    }

    #[test]
    fn test_scatter_write_and_read() {
        let router = ShardRouter::new_local(4);
        let sg = ScatterGather::new(&router);
        let store = make_store(4);

        // 쓰기
        sg.scatter_write(b"user:1", b"Alice", |shard_id, key, val| {
            store[shard_id].lock().unwrap().insert(key.to_vec(), val.to_vec());
        });

        // 읽기
        let result = sg.scatter_read(b"user:1", |shard_id, key| {
            store[shard_id].lock().unwrap().get(key).cloned()
        });

        assert_eq!(result, Some(b"Alice".to_vec()));
    }

    #[test]
    fn test_gather_all_shards() {
        let router = ShardRouter::new_local(3);
        let sg = ScatterGather::new(&router);
        let store = make_store(3);

        // 각 샤드에 한 건씩 직접 넣기
        for i in 0..3usize {
            store[i].lock().unwrap().insert(
                format!("key-shard-{}", i).into_bytes(),
                format!("val-{}", i).into_bytes(),
            );
        }

        let all = sg.gather(|shard_id| {
            store[shard_id]
                .lock()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        });

        assert_eq!(all.len(), 3, "3개 샤드에서 3개 항목 수집");
    }

    #[test]
    fn test_gather_sorted() {
        let router = ShardRouter::new_local(2);
        let sg = ScatterGather::new(&router);
        let store = make_store(2);

        // 정렬 역순으로 삽입
        store[0].lock().unwrap().insert(b"c".to_vec(), b"C".to_vec());
        store[0].lock().unwrap().insert(b"a".to_vec(), b"A".to_vec());
        store[1].lock().unwrap().insert(b"b".to_vec(), b"B".to_vec());

        let sorted = sg.gather_sorted(|shard_id| {
            store[shard_id]
                .lock()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        });

        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].0, b"a");
        assert_eq!(sorted[1].0, b"b");
        assert_eq!(sorted[2].0, b"c");
    }
}
