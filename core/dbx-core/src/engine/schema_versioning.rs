//! Schema Versioning — Phase 2: Section 6.1
//!
//! MVCC 기반 스키마 버전 관리: 무중단 DDL (ALTER TABLE) 지원
//!
//! 최적화: DashMap + 현재 스키마 캐싱으로 get_current O(1)

use crate::error::{DbxError, DbxResult};
use arrow::datatypes::Schema;
use dashmap::DashMap;
use std::sync::Arc;

/// 스키마 버전 정보
#[derive(Debug, Clone)]
pub struct SchemaVersion {
    /// 버전 번호
    pub version: u64,
    /// Arrow 스키마
    pub schema: Arc<Schema>,
    /// 생성 타임스탬프
    pub created_at: u64,
    /// 변경 설명
    pub description: String,
}

/// 스키마 버전 관리자
///
/// 테이블별 스키마 히스토리를 관리하고, 특정 시점의 스키마를 조회합니다.
/// MVCC 패턴으로 무중단 DDL을 지원합니다.
///
/// DashMap 기반 락-프리 설계로 높은 동시성 읽기 성능을 제공합니다.
pub struct SchemaVersionManager {
    /// 테이블별 스키마 버전 히스토리
    versions: DashMap<String, Vec<SchemaVersion>>,
    /// 현재 버전 번호
    current_versions: DashMap<String, u64>,
    /// 현재 스키마 캐시 (get_current O(1) 최적화)
    current_cache: DashMap<String, Arc<Schema>>,
}

impl SchemaVersionManager {
    /// 새 스키마 버전 관리자 생성
    pub fn new() -> Self {
        Self {
            versions: DashMap::new(),
            current_versions: DashMap::new(),
            current_cache: DashMap::new(),
        }
    }

    /// 초기 스키마 등록
    pub fn register_table(&self, table: &str, schema: Arc<Schema>) -> DbxResult<u64> {
        let version = SchemaVersion {
            version: 1,
            schema: schema.clone(),
            created_at: Self::now(),
            description: "Initial schema".to_string(),
        };

        self.versions.insert(table.to_string(), vec![version]);
        self.current_versions.insert(table.to_string(), 1);
        self.current_cache.insert(table.to_string(), schema);

        Ok(1)
    }

    /// 스키마 변경 (새 버전 생성 — 무중단)
    pub fn alter_table(
        &self,
        table: &str,
        new_schema: Arc<Schema>,
        description: &str,
    ) -> DbxResult<u64> {
        let mut history = self
            .versions
            .get_mut(table)
            .ok_or_else(|| DbxError::TableNotFound(table.to_string()))?;

        let new_version = history.last().map(|v| v.version + 1).unwrap_or(1);

        history.push(SchemaVersion {
            version: new_version,
            schema: new_schema.clone(),
            created_at: Self::now(),
            description: description.to_string(),
        });

        self.current_versions.insert(table.to_string(), new_version);
        self.current_cache.insert(table.to_string(), new_schema);

        Ok(new_version)
    }

    /// 현재 스키마 조회 — O(1) DashMap 캐시 히트
    pub fn get_current(&self, table: &str) -> DbxResult<Arc<Schema>> {
        self.current_cache
            .get(table)
            .map(|r| r.value().clone())
            .ok_or_else(|| DbxError::TableNotFound(table.to_string()))
    }

    /// 특정 시점의 스키마 조회 (MVCC 스냅샷)
    pub fn get_at_version(&self, table: &str, version: u64) -> DbxResult<Arc<Schema>> {
        let history = self
            .versions
            .get(table)
            .ok_or_else(|| DbxError::TableNotFound(table.to_string()))?;

        history
            .iter()
            .find(|v| v.version == version)
            .map(|v| v.schema.clone())
            .ok_or_else(|| {
                DbxError::Serialization(format!("Version {version} not found for {table}"))
            })
    }

    /// 스키마 버전 히스토리 조회
    pub fn version_history(&self, table: &str) -> DbxResult<Vec<SchemaVersion>> {
        self.versions
            .get(table)
            .map(|r| r.value().clone())
            .ok_or_else(|| DbxError::TableNotFound(table.to_string()))
    }

    /// 현재 버전 번호 조회
    pub fn current_version(&self, table: &str) -> DbxResult<u64> {
        self.current_versions
            .get(table)
            .map(|r| *r.value())
            .ok_or_else(|| DbxError::TableNotFound(table.to_string()))
    }

    /// 스키마 롤백 (이전 버전으로)
    pub fn rollback(&self, table: &str, target_version: u64) -> DbxResult<()> {
        // 대상 버전이 존재하는지 확인 + 스키마 가져오기
        let schema = {
            let history = self
                .versions
                .get(table)
                .ok_or_else(|| DbxError::TableNotFound(table.to_string()))?;

            history
                .iter()
                .find(|v| v.version == target_version)
                .map(|v| v.schema.clone())
                .ok_or_else(|| {
                    DbxError::Serialization(format!(
                        "Version {target_version} not found for {table}"
                    ))
                })?
        };

        self.current_versions
            .insert(table.to_string(), target_version);
        self.current_cache.insert(table.to_string(), schema);

        Ok(())
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

impl Default for SchemaVersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::{DataType, Field};

    fn make_schema(fields: &[(&str, DataType)]) -> Arc<Schema> {
        Arc::new(Schema::new(
            fields
                .iter()
                .map(|(n, t)| Field::new(*n, t.clone(), true))
                .collect::<Vec<_>>(),
        ))
    }

    #[test]
    fn test_register_and_get() {
        let mgr = SchemaVersionManager::new();
        let schema = make_schema(&[("id", DataType::Int64), ("name", DataType::Utf8)]);
        mgr.register_table("users", schema.clone()).unwrap();

        let current = mgr.get_current("users").unwrap();
        assert_eq!(current.fields().len(), 2);
        assert_eq!(mgr.current_version("users").unwrap(), 1);
    }

    #[test]
    fn test_alter_table() {
        let mgr = SchemaVersionManager::new();
        let v1 = make_schema(&[("id", DataType::Int64), ("name", DataType::Utf8)]);
        mgr.register_table("users", v1).unwrap();

        let v2 = make_schema(&[
            ("id", DataType::Int64),
            ("name", DataType::Utf8),
            ("email", DataType::Utf8),
        ]);
        let ver = mgr.alter_table("users", v2, "Add email column").unwrap();
        assert_eq!(ver, 2);

        let current = mgr.get_current("users").unwrap();
        assert_eq!(current.fields().len(), 3);
    }

    #[test]
    fn test_version_history() {
        let mgr = SchemaVersionManager::new();
        let v1 = make_schema(&[("id", DataType::Int64)]);
        mgr.register_table("users", v1).unwrap();

        let v2 = make_schema(&[("id", DataType::Int64), ("name", DataType::Utf8)]);
        mgr.alter_table("users", v2, "Add name").unwrap();

        let history = mgr.version_history("users").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].version, 1);
        assert_eq!(history[1].version, 2);
    }

    #[test]
    fn test_get_at_version() {
        let mgr = SchemaVersionManager::new();
        let v1 = make_schema(&[("id", DataType::Int64)]);
        mgr.register_table("users", v1).unwrap();

        let v2 = make_schema(&[("id", DataType::Int64), ("name", DataType::Utf8)]);
        mgr.alter_table("users", v2, "Add name").unwrap();

        // v1: 1 field, v2: 2 fields
        let old = mgr.get_at_version("users", 1).unwrap();
        assert_eq!(old.fields().len(), 1);

        let new = mgr.get_at_version("users", 2).unwrap();
        assert_eq!(new.fields().len(), 2);
    }

    #[test]
    fn test_rollback() {
        let mgr = SchemaVersionManager::new();
        let v1 = make_schema(&[("id", DataType::Int64)]);
        mgr.register_table("users", v1).unwrap();

        let v2 = make_schema(&[("id", DataType::Int64), ("name", DataType::Utf8)]);
        mgr.alter_table("users", v2, "Add name").unwrap();

        assert_eq!(mgr.current_version("users").unwrap(), 2);

        mgr.rollback("users", 1).unwrap();
        assert_eq!(mgr.current_version("users").unwrap(), 1);

        let current = mgr.get_current("users").unwrap();
        assert_eq!(current.fields().len(), 1);
    }
}
