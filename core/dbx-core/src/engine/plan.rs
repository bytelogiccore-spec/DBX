//! SQL 실행 계획 캐싱
//!
//! Prepared Statement와 실행 계획을 캐싱하여 파싱 오버헤드 제거

use sqlparser::ast::Statement;
use std::collections::HashMap;
use std::sync::Mutex;

/// SQL 실행 계획 캐시
pub struct PlanCache {
    cache: Mutex<HashMap<String, CachedPlan>>,
    max_size: usize,
}

/// 캐시된 실행 계획
#[derive(Clone)]
pub struct CachedPlan {
    pub statement: Statement,
    pub hit_count: u64,
}

impl PlanCache {
    /// 새 실행 계획 캐시 생성
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_size,
        }
    }

    /// 기본 크기 (1,000개)
    pub fn with_default_size() -> Self {
        Self::new(1_000)
    }

    /// 실행 계획 조회
    pub fn get(&self, sql: &str) -> Option<Statement> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(plan) = cache.get_mut(sql) {
            plan.hit_count += 1;
            Some(plan.statement.clone())
        } else {
            None
        }
    }

    /// 실행 계획 저장
    pub fn insert(&self, sql: String, statement: Statement) {
        let mut cache = self.cache.lock().unwrap();

        // 캐시가 가득 차면 가장 적게 사용된 항목 제거
        if cache.len() >= self.max_size
            && let Some(lru_key) = cache
                .iter()
                .min_by_key(|(_, plan)| plan.hit_count)
                .map(|(k, _)| k.clone())
        {
            cache.remove(&lru_key);
        }

        cache.insert(
            sql,
            CachedPlan {
                statement,
                hit_count: 0,
            },
        );
    }

    /// 캐시 초기화
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// 캐시 크기
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// 캐시가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    #[test]
    fn test_plan_cache_basic() {
        let cache = PlanCache::new(10);
        let sql = "SELECT * FROM users";

        let dialect = GenericDialect {};
        let statement = Parser::parse_sql(&dialect, sql)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        cache.insert(sql.to_string(), statement.clone());

        let cached = cache.get(sql);
        assert!(cached.is_some());
    }

    #[test]
    fn test_plan_cache_eviction() {
        let cache = PlanCache::new(2);
        let dialect = GenericDialect {};

        let sql1 = "SELECT * FROM users";
        let sql2 = "SELECT * FROM orders";
        let sql3 = "SELECT * FROM products";

        let stmt1 = Parser::parse_sql(&dialect, sql1)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let stmt2 = Parser::parse_sql(&dialect, sql2)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let stmt3 = Parser::parse_sql(&dialect, sql3)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        cache.insert(sql1.to_string(), stmt1);
        cache.insert(sql2.to_string(), stmt2);
        cache.insert(sql3.to_string(), stmt3); // sql1 or sql2 evicted

        assert_eq!(cache.len(), 2);
    }
}
