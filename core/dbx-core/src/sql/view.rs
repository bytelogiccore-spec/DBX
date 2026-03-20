//! SQL Views — CREATE VIEW / DROP VIEW 지원
//!
//! 뷰는 이름 → SQL 문자열 매핑을 DashMap으로 저장합니다.
//! SQL 실행 전 FROM 절에서 뷰 이름 발견 시 서브쿼리로 인라인 치환.

use dashmap::DashMap;
use std::sync::Arc;

use crate::error::{DbxError, DbxResult};

/// SQL 뷰 레지스트리
///
/// 뷰 이름 → SQL 정의 문자열을 스레드 안전하게 관리합니다.
#[derive(Debug, Default)]
pub struct ViewRegistry {
    /// view_name (소문자) → SQL 텍스트
    views: DashMap<String, String>,
}

impl ViewRegistry {
    /// 새 ViewRegistry 생성
    pub fn new() -> Self {
        Self::default()
    }

    /// 뷰 생성 (이미 존재하면 덮어씀)
    pub fn create(&self, name: &str, sql: &str) -> DbxResult<()> {
        self.views.insert(name.to_lowercase(), sql.to_string());
        Ok(())
    }

    /// 뷰 삭제. 없으면 Err 반환.
    pub fn drop(&self, name: &str) -> DbxResult<()> {
        self.views
            .remove(&name.to_lowercase())
            .map(|_| ())
            .ok_or_else(|| DbxError::InvalidArguments(format!("뷰 '{}' 를 찾을 수 없음", name)))
    }

    /// 뷰 존재 여부 확인
    pub fn exists(&self, name: &str) -> bool {
        self.views.contains_key(&name.to_lowercase())
    }

    /// SQL의 FROM 절에서 뷰 이름을 서브쿼리로 치환
    ///
    /// 예: `SELECT * FROM active_users`
    ///   → `SELECT * FROM (SELECT id, name FROM users WHERE active = true) AS active_users`
    pub fn expand(&self, sql: &str) -> String {
        let mut result = sql.to_string();
        for entry in self.views.iter() {
            let name = entry.key();
            let view_sql = entry.value();

            // "FROM <name>" 패턴 치환 (대소문자 무시)
            let pattern = format!("from {}", name);
            let replacement = format!("FROM ({}) AS {}", view_sql, name);

            // 대소문자 유지하면서 치환
            let lower = result.to_lowercase();
            if let Some(pos) = lower.find(&pattern) {
                result = format!(
                    "{}{}{}",
                    &result[..pos],
                    replacement,
                    &result[pos + pattern.len()..]
                );
            }
        }
        result
    }

    /// 등록된 뷰 목록 반환
    pub fn list_views(&self) -> Vec<String> {
        self.views.iter().map(|e| e.key().clone()).collect()
    }
}

/// Arc로 래핑된 공유 ViewRegistry 타입 별칭
pub type SharedViewRegistry = Arc<ViewRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_exists() {
        let reg = ViewRegistry::new();
        assert!(!reg.exists("active_users"));
        reg.create(
            "active_users",
            "SELECT id, name FROM users WHERE active = true",
        )
        .unwrap();
        assert!(reg.exists("active_users"));
    }

    #[test]
    fn test_create_case_insensitive() {
        let reg = ViewRegistry::new();
        reg.create("MyView", "SELECT 1").unwrap();
        assert!(reg.exists("myview"));
        assert!(reg.exists("MyView")); // 대소문자 무시
        assert!(reg.exists("MYVIEW"));
    }

    #[test]
    fn test_drop_view() {
        let reg = ViewRegistry::new();
        reg.create("v", "SELECT 1 AS x").unwrap();
        assert!(reg.exists("v"));
        reg.drop("v").unwrap();
        assert!(!reg.exists("v"));
    }

    #[test]
    fn test_drop_nonexistent_fails() {
        let reg = ViewRegistry::new();
        assert!(reg.drop("nonexistent").is_err());
    }

    #[test]
    fn test_expand_replaces_from_clause() {
        let reg = ViewRegistry::new();
        reg.create(
            "active_users",
            "SELECT id, name FROM users WHERE active = true",
        )
        .unwrap();

        let sql = "SELECT * FROM active_users";
        let expanded = reg.expand(sql);
        assert!(
            expanded.contains("(SELECT id, name FROM users WHERE active = true)"),
            "서브쿼리가 삽입되어야 함: {}",
            expanded
        );
        assert!(
            expanded.contains("AS active_users"),
            "별칭이 지정되어야 함: {}",
            expanded
        );
    }

    #[test]
    fn test_expand_no_match() {
        let reg = ViewRegistry::new();
        reg.create("v", "SELECT 1").unwrap();
        let sql = "SELECT * FROM users"; // 'v' 가 아닌 테이블
        let expanded = reg.expand(sql);
        assert_eq!(expanded, sql, "치환 없이 원래 SQL 유지");
    }

    #[test]
    fn test_list_views() {
        let reg = ViewRegistry::new();
        reg.create("v1", "SELECT 1").unwrap();
        reg.create("v2", "SELECT 2").unwrap();
        let mut views = reg.list_views();
        views.sort();
        assert_eq!(views, vec!["v1", "v2"]);
    }
}

// ════════════════════════════════════════════
// Materialized View Registry
// ════════════════════════════════════════════

use arrow::record_batch::RecordBatch;
use std::sync::RwLock;
use std::time::Instant;

/// 구체화된 뷰 항목 (내부 저장 단위)
struct MatViewEntry {
    /// 뷰를 정의하는 SQL 쿼리
    sql: String,
    /// 사전 계산된 쿼리 결과 캐시 (최초에는 None — stale 상태)
    cache: Option<Vec<RecordBatch>>,
    /// 마지막 갱신 시각
    refreshed_at: Option<Instant>,
    /// 자동 갱신 주기 (초 단위). None이면 수동 갱신만.
    refresh_interval_secs: Option<u64>,
}

/// 구체화된 뷰 레지스트리
///
/// CREATE MATERIALIZED VIEW / REFRESH / DROP 명령어를 처리합니다.
/// 각 뷰는 등록된 SQL과 사전 계산된 Arrow RecordBatch 캐시를 가집니다.
#[derive(Default)]
pub struct MaterializedViewRegistry {
    views: DashMap<String, RwLock<MatViewEntry>>,
}

impl MaterializedViewRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 구체화된 뷰 등록 (최초에는 stale — cache 없음)
    pub fn create(
        &self,
        name: &str,
        sql: &str,
        refresh_interval_secs: Option<u64>,
    ) -> DbxResult<()> {
        self.views.insert(
            name.to_lowercase(),
            RwLock::new(MatViewEntry {
                sql: sql.to_string(),
                cache: None,
                refreshed_at: None,
                refresh_interval_secs,
            }),
        );
        Ok(())
    }

    /// 캐시 저장 (REFRESH MATERIALIZED VIEW 호출 후)
    pub fn set_cache(&self, name: &str, batches: Vec<RecordBatch>) -> DbxResult<()> {
        let entry = self.views.get(&name.to_lowercase()).ok_or_else(|| {
            DbxError::InvalidArguments(format!("'{}' 구체화된 뷰를 찾을 수 없음", name))
        })?;
        let mut e = entry.write().unwrap();
        e.cache = Some(batches);
        e.refreshed_at = Some(Instant::now());
        Ok(())
    }

    /// 캐시가 유효한지 확인
    ///
    /// - 한 번도 갱신되지 않았으면 false (stale)
    /// - 수동 갱신 전용(interval 없음): 갱신된 적 있으면 true
    /// - interval이 있으면: 마지막 갱신으로부터 interval 초 미만이면 true
    pub fn is_fresh(&self, name: &str) -> bool {
        let entry = match self.views.get(&name.to_lowercase()) {
            Some(e) => e,
            None => return false,
        };
        let e = entry.read().unwrap();
        match (e.refreshed_at, e.refresh_interval_secs) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(t), Some(secs)) => t.elapsed().as_secs() < secs,
        }
    }

    /// 캐시 읽기 (SELECT 캐시 히트 시 사용)
    pub fn get_cache(&self, name: &str) -> Option<Vec<RecordBatch>> {
        let entry = self.views.get(&name.to_lowercase())?;
        entry.read().unwrap().cache.clone()
    }

    /// 뷰 SQL 읽기 (REFRESH 시 재실행 대상)
    pub fn get_sql(&self, name: &str) -> Option<String> {
        Some(
            self.views
                .get(&name.to_lowercase())?
                .read()
                .unwrap()
                .sql
                .clone(),
        )
    }

    /// 등록된 구체화된 뷰 이름 목록
    pub fn list(&self) -> Vec<String> {
        self.views.iter().map(|e| e.key().clone()).collect()
    }

    /// 구체화된 뷰 삭제 (remove: Rust의 Drop 트레이트와 이름 충돌 방지)
    pub fn remove(&self, name: &str) -> DbxResult<()> {
        self.views
            .remove(&name.to_lowercase())
            .map(|_| ())
            .ok_or_else(|| {
                DbxError::InvalidArguments(format!("'{}' 구체화된 뷰를 찾을 수 없음", name))
            })
    }
}

/// Arc로 래핑된 공유 MaterializedViewRegistry 타입 별칭
pub type SharedMaterializedViewRegistry = Arc<MaterializedViewRegistry>;

#[cfg(test)]
mod matview_tests {
    use super::*;
    use arrow::array::Int64Array;
    use arrow::datatypes::{DataType, Field, Schema};

    fn make_batch(n: i64) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int64, false)]));
        RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(vec![n]))]).unwrap()
    }

    #[test]
    fn test_materialized_view_cache() {
        let reg = MaterializedViewRegistry::new();
        reg.create("mv_users", "SELECT id FROM users", None).unwrap();
        assert!(!reg.is_fresh("mv_users")); // 생성 직후 stale

        reg.set_cache("mv_users", vec![make_batch(1), make_batch(2)])
            .unwrap();
        assert!(reg.is_fresh("mv_users")); // 갱신 후 fresh

        let cached = reg.get_cache("mv_users").unwrap();
        assert_eq!(cached.len(), 2);
    }

    #[test]
    fn test_matview_drop() {
        let reg = MaterializedViewRegistry::new();
        reg.create("mv_test", "SELECT 1", None).unwrap();
        assert!(reg.get_sql("mv_test").is_some());
        reg.remove("mv_test").unwrap();
        assert!(reg.get_sql("mv_test").is_none());
    }

    #[test]
    fn test_matview_drop_nonexistent_fails() {
        let reg = MaterializedViewRegistry::new();
        assert!(reg.remove("nonexistent").is_err());
    }

    #[test]
    fn test_matview_with_interval() {
        let reg = MaterializedViewRegistry::new();
        // 300초 갱신 주기
        reg.create("mv_sales", "SELECT * FROM sales", Some(300)).unwrap();
        assert!(!reg.is_fresh("mv_sales")); // 아직 갱신 안 됨

        reg.set_cache("mv_sales", vec![make_batch(42)]).unwrap();
        assert!(reg.is_fresh("mv_sales")); // 방금 갱신 — 300초 미만이므로 fresh

        let cached = reg.get_cache("mv_sales").unwrap();
        assert_eq!(cached[0].num_rows(), 1);
    }

    #[test]
    fn test_matview_list() {
        let reg = MaterializedViewRegistry::new();
        reg.create("mv_a", "SELECT 1", None).unwrap();
        reg.create("mv_b", "SELECT 2", None).unwrap();
        let mut names = reg.list();
        names.sort();
        assert_eq!(names, vec!["mv_a", "mv_b"]);
    }
}
