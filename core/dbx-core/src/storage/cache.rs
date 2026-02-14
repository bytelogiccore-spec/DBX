//! LRU Row Cache — Tier 2 캐시
//!
//! 핫 데이터를 메모리에 캐싱하여 WOS/ROS 접근 최소화

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// LRU Row Cache
pub struct RowCache {
    inner: Mutex<LruCache<CacheKey, Vec<u8>>>,
    hit_count: AtomicU64,
    miss_count: AtomicU64,
}

/// Cache Key: (table_name, row_key)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    table: String,
    key: Vec<u8>,
}

impl RowCache {
    /// 새로운 LRU Cache 생성
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).expect("capacity must be > 0");
        Self {
            inner: Mutex::new(LruCache::new(cap)),
            hit_count: AtomicU64::new(0),
            miss_count: AtomicU64::new(0),
        }
    }

    /// 기본 용량 (10,000개)
    pub fn with_default_capacity() -> Self {
        Self::new(10_000)
    }

    /// 자동 튜닝: 히트율 기반 용량 조정
    /// 히트율이 낮으면 용량 증가, 높으면 유지
    pub fn auto_tune(&self) -> Option<usize> {
        let ratio = self.hit_ratio();
        let current_size = {
            let cache = self.inner.lock().unwrap();
            cache.len()
        };

        // 히트율 95% 이상 목표
        if ratio < 0.95 && current_size > 0 {
            // 히트율이 낮으면 용량을 1.5배로 증가 권장
            Some((current_size as f64 * 1.5) as usize)
        } else {
            None // 현재 용량 유지
        }
    }

    /// Cache에서 값 조회
    pub fn get(&self, table: &str, key: &[u8]) -> Option<Vec<u8>> {
        let cache_key = CacheKey {
            table: table.to_string(),
            key: key.to_vec(),
        };

        let mut cache = self.inner.lock().unwrap();
        if let Some(value) = cache.get(&cache_key) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            Some(value.clone())
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Cache에 값 삽입
    pub fn insert(&self, table: &str, key: &[u8], value: &[u8]) {
        let cache_key = CacheKey {
            table: table.to_string(),
            key: key.to_vec(),
        };

        let mut cache = self.inner.lock().unwrap();
        cache.put(cache_key, value.to_vec());
    }

    /// Cache에서 특정 키 무효화
    pub fn invalidate(&self, table: &str, key: &[u8]) {
        let cache_key = CacheKey {
            table: table.to_string(),
            key: key.to_vec(),
        };

        let mut cache = self.inner.lock().unwrap();
        cache.pop(&cache_key);
    }

    /// 테이블 전체 무효화
    pub fn invalidate_table(&self, _table: &str) {
        let mut cache = self.inner.lock().unwrap();
        cache.clear();
        // TODO: 특정 테이블만 제거하도록 개선
    }

    /// Cache 히트율
    pub fn hit_ratio(&self) -> f64 {
        let hits = self.hit_count.load(Ordering::Relaxed);
        let misses = self.miss_count.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Cache 통계
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hit_count.load(Ordering::Relaxed),
            misses: self.miss_count.load(Ordering::Relaxed),
            hit_ratio: self.hit_ratio(),
        }
    }

    /// Cache 초기화
    pub fn clear(&self) {
        let mut cache = self.inner.lock().unwrap();
        cache.clear();
        self.hit_count.store(0, Ordering::Relaxed);
        self.miss_count.store(0, Ordering::Relaxed);
    }
}

/// Cache 통계
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic() {
        let cache = RowCache::new(3);

        // Insert
        cache.insert("users", b"key1", b"value1");
        cache.insert("users", b"key2", b"value2");

        // Get
        assert_eq!(cache.get("users", b"key1"), Some(b"value1".to_vec()));
        assert_eq!(cache.get("users", b"key2"), Some(b"value2".to_vec()));
        assert_eq!(cache.get("users", b"key3"), None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = RowCache::new(2);

        cache.insert("users", b"key1", b"value1");
        cache.insert("users", b"key2", b"value2");
        cache.insert("users", b"key3", b"value3"); // key1 evicted

        assert_eq!(cache.get("users", b"key1"), None);
        assert_eq!(cache.get("users", b"key2"), Some(b"value2".to_vec()));
        assert_eq!(cache.get("users", b"key3"), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_cache_invalidate() {
        let cache = RowCache::new(3);

        cache.insert("users", b"key1", b"value1");
        cache.insert("users", b"key2", b"value2");

        cache.invalidate("users", b"key1");

        assert_eq!(cache.get("users", b"key1"), None);
        assert_eq!(cache.get("users", b"key2"), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_cache_hit_ratio() {
        let cache = RowCache::new(3);

        cache.insert("users", b"key1", b"value1");

        cache.get("users", b"key1"); // hit
        cache.get("users", b"key1"); // hit
        cache.get("users", b"key2"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_ratio - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_cache_clear() {
        let cache = RowCache::new(3);

        cache.insert("users", b"key1", b"value1");
        cache.get("users", b"key1");

        cache.clear();

        assert_eq!(cache.get("users", b"key1"), None);
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 1);
    }
}
