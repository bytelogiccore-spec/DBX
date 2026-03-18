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
            .ok_or_else(|| {
                DbxError::InvalidArguments(format!("뷰 '{}' 를 찾을 수 없음", name))
            })
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
        reg.create("active_users", "SELECT id, name FROM users WHERE active = true")
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
