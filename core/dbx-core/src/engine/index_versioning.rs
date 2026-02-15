//! Index Versioning — Phase 2: Section 6.2
//!
//! 무중단 REINDEX 지원: 인덱스 메타데이터 버전 관리

use crate::error::{DbxError, DbxResult};
use std::collections::HashMap;
use std::sync::RwLock;

/// 인덱스 메타데이터
#[derive(Debug, Clone)]
pub struct IndexMeta {
    /// 인덱스 이름
    pub name: String,
    /// 대상 테이블
    pub table: String,
    /// 인덱스 컬럼
    pub columns: Vec<String>,
    /// 인덱스 종류
    pub index_type: IndexType,
    /// 버전 번호
    pub version: u64,
    /// 빌드 상태
    pub status: IndexStatus,
}

/// 인덱스 종류
#[derive(Debug, Clone, PartialEq)]
pub enum IndexType {
    Hash,
    BTree,
    Bitmap,
}

/// 인덱스 빌드 상태
#[derive(Debug, Clone, PartialEq)]
pub enum IndexStatus {
    /// 빌드 중 (이전 버전 사용 가능)
    Building,
    /// 사용 가능
    Ready,
    /// 비활성화
    Disabled,
}

/// 인덱스 버전 관리자
///
/// 인덱스를 무중단으로 재구축합니다:
/// 1. 새 버전 생성 (Building 상태)
/// 2. 백그라운드에서 인덱스 빌드
/// 3. Ready 상태로 전환 → 이전 버전 제거
pub struct IndexVersionManager {
    /// index_name → 버전 히스토리
    versions: RwLock<HashMap<String, Vec<IndexMeta>>>,
    /// index_name → 현재 활성 버전
    active: RwLock<HashMap<String, u64>>,
}

impl IndexVersionManager {
    pub fn new() -> Self {
        Self {
            versions: RwLock::new(HashMap::new()),
            active: RwLock::new(HashMap::new()),
        }
    }

    /// 인덱스 생성
    pub fn create_index(
        &self,
        name: &str,
        table: &str,
        columns: Vec<String>,
        index_type: IndexType,
    ) -> DbxResult<u64> {
        let meta = IndexMeta {
            name: name.to_string(),
            table: table.to_string(),
            columns,
            index_type,
            version: 1,
            status: IndexStatus::Ready,
        };

        let mut versions = self.versions.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let mut active = self.active.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;

        versions.insert(name.to_string(), vec![meta]);
        active.insert(name.to_string(), 1);

        Ok(1)
    }

    /// 무중단 REINDEX 시작 (새 버전을 Building 상태로 생성)
    pub fn start_reindex(
        &self,
        name: &str,
        columns: Vec<String>,
        index_type: IndexType,
    ) -> DbxResult<u64> {
        let mut versions = self.versions.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let history = versions.get_mut(name).ok_or_else(|| DbxError::Serialization(format!("Index {name} not found")))?;

        let last = history.last().ok_or_else(|| DbxError::Serialization("Empty history".into()))?;
        let new_version = last.version + 1;

        history.push(IndexMeta {
            name: name.to_string(),
            table: last.table.clone(),
            columns,
            index_type,
            version: new_version,
            status: IndexStatus::Building,
        });

        Ok(new_version)
    }

    /// REINDEX 완료 (Building → Ready, 이전 버전 비활성화)
    pub fn complete_reindex(&self, name: &str, version: u64) -> DbxResult<()> {
        let mut versions = self.versions.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let mut active = self.active.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;

        let history = versions.get_mut(name).ok_or_else(|| DbxError::Serialization(format!("Index {name} not found")))?;

        for meta in history.iter_mut() {
            if meta.version == version {
                meta.status = IndexStatus::Ready;
            } else if meta.status == IndexStatus::Ready {
                meta.status = IndexStatus::Disabled;
            }
        }

        active.insert(name.to_string(), version);
        Ok(())
    }

    /// 현재 활성 인덱스 메타데이터 조회
    pub fn get_active(&self, name: &str) -> DbxResult<IndexMeta> {
        let versions = self.versions.read().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let active = self.active.read().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;

        let active_ver = active.get(name).ok_or_else(|| DbxError::Serialization(format!("Index {name} not found")))?;
        let history = versions.get(name).ok_or_else(|| DbxError::Serialization(format!("Index {name} not found")))?;

        history
            .iter()
            .find(|m| m.version == *active_ver)
            .cloned()
            .ok_or_else(|| DbxError::Serialization(format!("Version {active_ver} not found")))
    }

    /// 인덱스 삭제
    pub fn drop_index(&self, name: &str) -> DbxResult<()> {
        let mut versions = self.versions.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let mut active = self.active.write().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;

        versions.remove(name);
        active.remove(name);
        Ok(())
    }

    /// 테이블의 모든 인덱스 조회
    pub fn list_indexes(&self, table: &str) -> DbxResult<Vec<IndexMeta>> {
        let versions = self.versions.read().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;
        let active = self.active.read().map_err(|_| DbxError::Serialization("Lock poisoned".into()))?;

        let mut result = Vec::new();
        for (name, history) in versions.iter() {
            if let Some(&active_ver) = active.get(name) {
                if let Some(meta) = history.iter().find(|m| m.version == active_ver && m.table == table) {
                    result.push(meta.clone());
                }
            }
        }
        Ok(result)
    }
}

impl Default for IndexVersionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index() {
        let mgr = IndexVersionManager::new();
        let ver = mgr.create_index("idx_users_email", "users", vec!["email".into()], IndexType::Hash).unwrap();
        assert_eq!(ver, 1);

        let meta = mgr.get_active("idx_users_email").unwrap();
        assert_eq!(meta.table, "users");
        assert_eq!(meta.status, IndexStatus::Ready);
    }

    #[test]
    fn test_reindex_zero_downtime() {
        let mgr = IndexVersionManager::new();
        mgr.create_index("idx1", "users", vec!["name".into()], IndexType::Hash).unwrap();

        // Start reindex (v1 still active)
        let v2 = mgr.start_reindex("idx1", vec!["name".into(), "email".into()], IndexType::BTree).unwrap();
        assert_eq!(v2, 2);

        // v1 is still active during build
        let active = mgr.get_active("idx1").unwrap();
        assert_eq!(active.version, 1);

        // Complete reindex → v2 becomes active
        mgr.complete_reindex("idx1", 2).unwrap();
        let active = mgr.get_active("idx1").unwrap();
        assert_eq!(active.version, 2);
        assert_eq!(active.columns.len(), 2);
        assert_eq!(active.index_type, IndexType::BTree);
    }

    #[test]
    fn test_drop_index() {
        let mgr = IndexVersionManager::new();
        mgr.create_index("idx1", "users", vec!["name".into()], IndexType::Hash).unwrap();
        mgr.drop_index("idx1").unwrap();

        assert!(mgr.get_active("idx1").is_err());
    }

    #[test]
    fn test_list_indexes() {
        let mgr = IndexVersionManager::new();
        mgr.create_index("idx1", "users", vec!["name".into()], IndexType::Hash).unwrap();
        mgr.create_index("idx2", "users", vec!["email".into()], IndexType::BTree).unwrap();
        mgr.create_index("idx3", "orders", vec!["id".into()], IndexType::Hash).unwrap();

        let user_indexes = mgr.list_indexes("users").unwrap();
        assert_eq!(user_indexes.len(), 2);

        let order_indexes = mgr.list_indexes("orders").unwrap();
        assert_eq!(order_indexes.len(), 1);
    }
}
