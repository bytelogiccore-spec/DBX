//! SQL 실행 계획 캐싱 (Phase 2 강화)
//!
//! DashMap 기반 lock-free 캐시 + 2단계 캐싱(L1 메모리, L2 디스크) + 통계

use dashmap::DashMap;
use sqlparser::ast::Statement;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// SQL 실행 계획 캐시 (Phase 2: DashMap + 2-level + 통계)
pub struct PlanCache {
    /// L1: lock-free in-memory cache
    l1: Arc<DashMap<String, CachedPlan>>,
    /// L2: disk-based cache (optional)
    l2_dir: Option<PathBuf>,
    /// Maximum L1 entries
    max_l1_size: usize,
    /// Cache statistics
    stats: CacheStats,
}

/// 캐시된 실행 계획
#[derive(Clone)]
pub struct CachedPlan {
    pub statement: Statement,
    pub hit_count: u64,
}

/// 캐시 통계
pub struct CacheStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
    pub l2_hits: AtomicU64,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
            l2_hits: AtomicU64::new(0),
        }
    }

    /// Hit rate (0.0 ~ 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let total = hits + self.misses.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Total requests
    pub fn total(&self) -> u64 {
        self.hits.load(Ordering::Relaxed) + self.misses.load(Ordering::Relaxed)
    }
}

impl PlanCache {
    /// 새 실행 계획 캐시 생성
    pub fn new(max_size: usize) -> Self {
        Self {
            l1: Arc::new(DashMap::with_capacity(max_size)),
            l2_dir: None,
            max_l1_size: max_size,
            stats: CacheStats::new(),
        }
    }

    /// 기본 크기 (1,000개)
    pub fn with_default_size() -> Self {
        Self::new(1_000)
    }

    /// L2 디스크 캐시 활성화
    pub fn with_l2_cache(mut self, dir: PathBuf) -> Self {
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }
        self.l2_dir = Some(dir);
        self
    }

    /// 실행 계획 조회 (L1 → L2 순서)
    pub fn get(&self, sql: &str) -> Option<Statement> {
        // L1 lookup (lock-free)
        if let Some(mut entry) = self.l1.get_mut(sql) {
            entry.hit_count += 1;
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            return Some(entry.statement.clone());
        }

        // L2 lookup (disk)
        if let Some(stmt) = self.get_l2(sql) {
            self.stats.l2_hits.fetch_add(1, Ordering::Relaxed);
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            // Promote to L1
            self.insert_l1(sql.to_string(), stmt.clone());
            return Some(stmt);
        }

        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// 실행 계획 저장 (L1 + L2)
    pub fn insert(&self, sql: String, statement: Statement) {
        self.insert_l1(sql.clone(), statement.clone());
        self.put_l2(&sql, &statement);
    }

    /// L1에만 삽입 (LFU eviction)
    fn insert_l1(&self, sql: String, statement: Statement) {
        // Eviction: L1이 가득 차면 가장 적게 사용된 항목 제거
        if self.l1.len() >= self.max_l1_size
            && let Some(lru_key) = self.find_lfu_key()
        {
            self.l1.remove(&lru_key);
            self.stats.evictions.fetch_add(1, Ordering::Relaxed);
        }

        self.l1.insert(
            sql,
            CachedPlan {
                statement,
                hit_count: 0,
            },
        );
    }

    /// LFU(Least Frequently Used) key 찾기
    fn find_lfu_key(&self) -> Option<String> {
        let mut min_hits = u64::MAX;
        let mut lfu_key = None;
        for entry in self.l1.iter() {
            if entry.value().hit_count < min_hits {
                min_hits = entry.value().hit_count;
                lfu_key = Some(entry.key().clone());
            }
        }
        lfu_key
    }

    /// L2 디스크에 저장 (SQL 해시를 파일명으로)
    fn put_l2(&self, sql: &str, statement: &Statement) {
        if let Some(dir) = &self.l2_dir {
            let hash = Self::hash_sql(sql);
            let path = dir.join(format!("{hash}.plan"));
            // Statement를 Debug format으로 직렬화 (간단한 구현)
            let data = format!("{statement:?}");
            let _ = std::fs::write(path, data.as_bytes());
        }
    }

    /// L2 디스크에서 조회
    fn get_l2(&self, sql: &str) -> Option<Statement> {
        let dir = self.l2_dir.as_ref()?;
        let hash = Self::hash_sql(sql);
        let path = dir.join(format!("{hash}.plan"));

        if path.exists() {
            // L2에서는 SQL을 다시 파싱하여 복원 (간단한 구현)
            use sqlparser::dialect::GenericDialect;
            use sqlparser::parser::Parser;
            let dialect = GenericDialect {};
            Parser::parse_sql(&dialect, sql).ok()?.into_iter().next()
        } else {
            None
        }
    }

    /// SQL 해시 (FNV-1a for fast hashing)
    fn hash_sql(sql: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in sql.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// 캐시 초기화
    pub fn clear(&self) {
        self.l1.clear();
        if let Some(dir) = &self.l2_dir {
            let _ = std::fs::remove_dir_all(dir);
            let _ = std::fs::create_dir_all(dir);
        }
    }

    /// L1 캐시 크기
    pub fn len(&self) -> usize {
        self.l1.len()
    }

    /// 캐시가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.l1.is_empty()
    }

    /// 캐시 통계 조회
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 특정 SQL이 캐시에 있는지 확인 (O(1))
    pub fn contains(&self, sql: &str) -> bool {
        self.l1.contains_key(sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    fn parse_one(sql: &str) -> Statement {
        let dialect = GenericDialect {};
        Parser::parse_sql(&dialect, sql)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }

    #[test]
    fn test_plan_cache_basic() {
        let cache = PlanCache::new(10);
        let sql = "SELECT * FROM users";
        let stmt = parse_one(sql);

        cache.insert(sql.to_string(), stmt.clone());

        let cached = cache.get(sql);
        assert!(cached.is_some());
        assert_eq!(cache.stats().hits.load(Ordering::Relaxed), 1);
        assert_eq!(cache.stats().misses.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_plan_cache_eviction() {
        let cache = PlanCache::new(2);

        let sql1 = "SELECT * FROM users";
        let sql2 = "SELECT * FROM orders";
        let sql3 = "SELECT * FROM products";

        cache.insert(sql1.to_string(), parse_one(sql1));
        cache.insert(sql2.to_string(), parse_one(sql2));
        cache.insert(sql3.to_string(), parse_one(sql3)); // eviction happens

        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_plan_cache_hit_rate() {
        let cache = PlanCache::new(10);
        let sql = "SELECT * FROM users";
        cache.insert(sql.to_string(), parse_one(sql));

        cache.get(sql); // hit
        cache.get(sql); // hit
        cache.get("SELECT 1"); // miss

        assert_eq!(cache.stats().hits.load(Ordering::Relaxed), 2);
        assert_eq!(cache.stats().misses.load(Ordering::Relaxed), 1);
        assert!((cache.stats().hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_plan_cache_l2_disk() {
        let tmp_dir = std::env::temp_dir().join("dbx_plan_cache_test");
        let _ = std::fs::remove_dir_all(&tmp_dir);

        let cache = PlanCache::new(1).with_l2_cache(tmp_dir.clone());
        let sql1 = "SELECT * FROM users";
        let sql2 = "SELECT * FROM orders";

        cache.insert(sql1.to_string(), parse_one(sql1));
        cache.insert(sql2.to_string(), parse_one(sql2)); // sql1 evicted from L1

        // sql1 should be retrievable from L2
        let result = cache.get(sql1);
        assert!(result.is_some());
        assert_eq!(cache.stats().l2_hits.load(Ordering::Relaxed), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_plan_cache_contains() {
        let cache = PlanCache::new(10);
        let sql = "SELECT * FROM users";
        assert!(!cache.contains(sql));

        cache.insert(sql.to_string(), parse_one(sql));
        assert!(cache.contains(sql));
    }

    #[test]
    fn test_plan_cache_concurrent_access() {
        use std::thread;

        let cache = Arc::new(PlanCache::new(100));
        let mut handles = vec![];

        for i in 0..8 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                let sql = format!("SELECT * FROM table_{i}");
                let stmt = parse_one(&sql);
                cache.insert(sql.clone(), stmt);
                assert!(cache.get(&sql).is_some());
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(cache.len(), 8);
    }
}
