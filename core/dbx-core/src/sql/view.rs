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
// Materialized View Registry (Event-Driven)
// ════════════════════════════════════════════

use arrow::record_batch::RecordBatch;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

/// SQL 문에서 FROM / JOIN 절의 테이블명을 간단히 추출합니다.
///
/// 완전한 SQL 파서 대신, `FROM <table>` 및 `JOIN <table>` 패턴을 대소문자 무시로 추출합니다.
/// 서브쿼리 내 테이블은 포함하지 않지만, 대부분의 MV 사용 사례에서 충분합니다.
fn extract_source_tables(sql: &str) -> Vec<String> {
    let upper = sql.to_uppercase();
    let tokens: Vec<&str> = upper.split_whitespace().collect();
    let original_tokens: Vec<&str> = sql.split_whitespace().collect();
    let mut tables = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        if (*token == "FROM" || *token == "JOIN") && i + 1 < tokens.len() {
            let table_name = original_tokens[i + 1]
                .trim_matches(|c: char| c == '(' || c == ')' || c == ',' || c == ';')
                .to_lowercase();
            if !table_name.is_empty() && table_name != "select" && table_name != "(" {
                tables.push(table_name);
            }
        }
    }
    tables.sort();
    tables.dedup();
    tables
}

/// 구체화된 뷰 항목 (내부 저장 단위)
struct MatViewEntry {
    /// 뷰를 정의하는 SQL 쿼리
    sql: String,
    /// 사전 계산된 쿼리 결과 캐시 (최초에는 None — stale 상태)
    cache: Option<Vec<RecordBatch>>,
    /// 마지막 갱신 시각
    refreshed_at: Option<Instant>,
    /// 자동 갱신 주기 (초 단위). None이면 이벤트 기반 갱신만.
    refresh_interval_secs: Option<u64>,
    /// MV SQL이 참조하는 소스 테이블 목록 (FROM/JOIN 절에서 추출)
    source_tables: Vec<String>,
}

/// 이벤트 기반 Materialized View 알림 채널
///
/// DML 발생 시 `notify_change(table)` → dirty set에 추가 → Condvar 깨움
/// 백그라운드 스레드가 즉시 깨어나 debounce 후 갱신 실행
pub struct MatViewNotifier {
    /// 갱신 대기 중인 MV 이름 세트
    dirty: Mutex<HashSet<String>>,
    /// 블로킹 대기용 Condvar
    condvar: Condvar,
}

impl MatViewNotifier {
    fn new() -> Self {
        Self {
            dirty: Mutex::new(HashSet::new()),
            condvar: Condvar::new(),
        }
    }

    /// dirty set에 MV 이름 추가 + Condvar 깨움
    fn mark_dirty(&self, mv_name: &str) {
        let mut dirty = self.dirty.lock().unwrap();
        dirty.insert(mv_name.to_string());
        self.condvar.notify_one();
    }

    /// dirty set이 비어있으면 블로킹 대기, 채워지면 drain하여 반환
    pub fn wait_and_take(&self) -> HashSet<String> {
        let mut dirty = self.dirty.lock().unwrap();
        while dirty.is_empty() {
            dirty = self.condvar.wait(dirty).unwrap();
        }
        dirty.drain().collect()
    }

    /// non-blocking: 현재 dirty set을 drain하여 반환 (비어있으면 빈 세트)
    pub fn take(&self) -> HashSet<String> {
        let mut dirty = self.dirty.lock().unwrap();
        dirty.drain().collect()
    }
}

/// 구체화된 뷰 레지스트리 (이벤트 기반 갱신)
///
/// CREATE MATERIALIZED VIEW / REFRESH / DROP 명령어를 처리합니다.
/// 각 뷰는 등록된 SQL과 사전 계산된 Arrow RecordBatch 캐시를 가집니다.
///
/// DML 발생 시 `notify_change(table)` 호출로 해당 테이블을 참조하는 MV를 즉시 갱신합니다.
/// `min_refresh_interval_ms`로 갱신 폭풍을 방지합니다 (debounce).
pub struct MaterializedViewRegistry {
    views: DashMap<String, RwLock<MatViewEntry>>,
    /// 이벤트 알림 채널
    notifier: MatViewNotifier,
    /// 최소 갱신 주기 (밀리초 단위, 기본 1000ms). debounce용.
    min_refresh_interval_ms: std::sync::atomic::AtomicU64,
}

impl Default for MaterializedViewRegistry {
    fn default() -> Self {
        Self {
            views: DashMap::new(),
            notifier: MatViewNotifier::new(),
            min_refresh_interval_ms: std::sync::atomic::AtomicU64::new(1000),
        }
    }
}

impl MaterializedViewRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 최소 갱신 주기를 밀리초 단위로 설정합니다.
    ///
    /// 이 값은 DML 이벤트 발생 후 실제 MV 갱신까지의 최소 대기 시간입니다.
    /// 빈번한 쓰기 시 갱신 폭풍을 방지하는 debounce 역할을 합니다.
    ///
    /// # 예시
    ///
    /// ```rust,ignore
    /// registry.set_min_refresh_interval_ms(500); // 0.5초 debounce
    /// ```
    pub fn set_min_refresh_interval_ms(&self, ms: u64) {
        self.min_refresh_interval_ms
            .store(ms, std::sync::atomic::Ordering::Relaxed);
    }

    /// 현재 설정된 최소 갱신 주기 (밀리초)
    pub fn min_refresh_interval_ms(&self) -> u64 {
        self.min_refresh_interval_ms
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 최소 갱신 주기를 Duration으로 반환
    pub fn min_refresh_interval(&self) -> Duration {
        Duration::from_millis(self.min_refresh_interval_ms())
    }

    /// 구체화된 뷰 등록 (최초에는 stale — cache 없음)
    ///
    /// SQL에서 FROM/JOIN 절을 파싱하여 source_tables를 자동 추출합니다.
    pub fn create(
        &self,
        name: &str,
        sql: &str,
        refresh_interval_secs: Option<u64>,
    ) -> DbxResult<()> {
        let source_tables = extract_source_tables(sql);
        self.views.insert(
            name.to_lowercase(),
            RwLock::new(MatViewEntry {
                sql: sql.to_string(),
                cache: None,
                refreshed_at: None,
                refresh_interval_secs,
                source_tables,
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

    // ════════════════════════════════════════════
    // Event-Driven Notification API
    // ════════════════════════════════════════════

    /// DML(INSERT/UPDATE/DELETE) 발생 시 호출.
    ///
    /// 변경된 테이블을 참조하는 모든 MV를 dirty로 마킹하고
    /// 백그라운드 갱신 스레드를 즉시 깨웁니다.
    ///
    /// 이 메서드는 멱등적이며, 빈번하게 호출해도 안전합니다.
    pub fn notify_change(&self, table: &str) {
        if self.views.is_empty() {
            return; // MV 없으면 즉시 반환 (zero overhead)
        }
        let table_lower = table.to_lowercase();
        // shard 접미사 제거 (예: "users__shard_0" → "users")
        let base_table = if let Some(idx) = table_lower.find("__shard_") {
            &table_lower[..idx]
        } else {
            &table_lower
        };
        for entry in self.views.iter() {
            let mv_name = entry.key();
            let e = entry.value().read().unwrap();
            if e.source_tables.iter().any(|t| t == base_table) {
                drop(e); // lock 해제 후 알림
                self.notifier.mark_dirty(mv_name);
            }
        }
    }

    /// 백그라운드 스레드용: dirty MV가 생길 때까지 블로킹 대기 후 drain
    pub fn wait_and_take_dirty(&self) -> HashSet<String> {
        self.notifier.wait_and_take()
    }

    /// 백그라운드 스레드용: 현재 dirty set을 non-blocking으로 drain
    pub fn take_dirty(&self) -> HashSet<String> {
        self.notifier.take()
    }

    /// 등록된 MV 중 stale(갱신 필요)인 것들의 이름 반환 (폴링 호환)
    pub fn stale_views(&self) -> Vec<String> {
        self.views
            .iter()
            .filter(|e| {
                let entry = e.value().read().unwrap();
                match (entry.refreshed_at, entry.refresh_interval_secs) {
                    (None, _) => true,
                    (Some(_), None) => false,
                    (Some(t), Some(secs)) => t.elapsed().as_secs() >= secs,
                }
            })
            .map(|e| e.key().clone())
            .collect()
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
        reg.create("mv_users", "SELECT id FROM users", None)
            .unwrap();
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
        reg.create("mv_sales", "SELECT * FROM sales", Some(300))
            .unwrap();
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

    #[test]
    fn test_extract_source_tables() {
        assert_eq!(
            extract_source_tables("SELECT id, name FROM users WHERE active = true"),
            vec!["users"]
        );
        assert_eq!(
            extract_source_tables("SELECT * FROM orders JOIN users ON orders.uid = users.id"),
            vec!["orders", "users"]
        );
        assert_eq!(
            extract_source_tables("SELECT AVG(price) FROM products"),
            vec!["products"]
        );
        // duplicate elimination
        assert_eq!(
            extract_source_tables("SELECT * FROM t1 JOIN t1 ON t1.a = t1.b"),
            vec!["t1"]
        );
    }

    #[test]
    fn test_notify_change_marks_dirty() {
        let reg = MaterializedViewRegistry::new();
        reg.create("mv_users", "SELECT id FROM users", None)
            .unwrap();
        reg.create("mv_orders", "SELECT * FROM orders", None)
            .unwrap();

        // users 변경 → mv_users만 dirty
        reg.notify_change("users");
        let dirty = reg.take_dirty();
        assert!(dirty.contains("mv_users"));
        assert!(!dirty.contains("mv_orders"));

        // orders 변경 → mv_orders만 dirty
        reg.notify_change("orders");
        let dirty = reg.take_dirty();
        assert!(dirty.contains("mv_orders"));
        assert!(!dirty.contains("mv_users"));
    }

    #[test]
    fn test_notify_change_shard_table() {
        let reg = MaterializedViewRegistry::new();
        reg.create("mv_users", "SELECT id FROM users", None)
            .unwrap();

        // shard 테이블 변경도 인식
        reg.notify_change("users__shard_0");
        let dirty = reg.take_dirty();
        assert!(dirty.contains("mv_users"));
    }

    #[test]
    fn test_configurable_min_refresh_interval() {
        let reg = MaterializedViewRegistry::new();
        assert_eq!(reg.min_refresh_interval_ms(), 1000); // 기본 1초

        reg.set_min_refresh_interval_ms(500);
        assert_eq!(reg.min_refresh_interval_ms(), 500);
        assert_eq!(reg.min_refresh_interval(), Duration::from_millis(500));
    }

    #[test]
    fn test_notify_no_views_is_noop() {
        let reg = MaterializedViewRegistry::new();
        // MV 없을 때 호출해도 패닉 없음
        reg.notify_change("some_table");
        let dirty = reg.take_dirty();
        assert!(dirty.is_empty());
    }
}
