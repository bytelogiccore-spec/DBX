//! VersionManager — 범용 MVCC 버전 관리자
//!
//! `Versionable` 트레이트를 구현한 임의의 타입에 대해 버전 관리를 제공합니다.

use crate::error::{DbxError, DbxResult};
use crate::transaction::manager::TimestampOracle;
use crate::transaction::versionable::Versionable;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// 범용 버전 관리자
///
/// `Versionable` 트레이트를 구현한 타입 `T`에 대해 MVCC 버전 관리를 제공합니다.
///
/// # Type Parameters
///
/// * `T` - 버전 관리할 타입 (반드시 `Versionable` 트레이트 구현 필요)
///
/// # Example
///
/// ```rust
/// use dbx_core::transaction::version_manager::VersionManager;
/// use dbx_core::transaction::versionable::Versionable;
/// use dbx_core::transaction::manager::TimestampOracle;
/// use std::sync::Arc;
///
/// // String은 Versionable을 구현하므로 바로 사용 가능
/// let oracle = Arc::new(TimestampOracle::default());
/// let manager = VersionManager::<String>::new(Arc::clone(&oracle));
///
/// // 버전 추가
/// manager.add_version(b"user:1".to_vec(), "Alice".to_string(), 10).unwrap();
/// manager.add_version(b"user:1".to_vec(), "Alice Updated".to_string(), 20).unwrap();
///
/// // 스냅샷 조회
/// let value_at_15 = manager.get_at_snapshot(b"user:1", 15).unwrap();
/// assert_eq!(value_at_15, Some("Alice".to_string()));
/// ```
/// Type alias for version storage
type VersionStorage<T> = Arc<RwLock<BTreeMap<Vec<u8>, Vec<(u64, T)>>>>;

#[derive(Clone)]
pub struct VersionManager<T: Versionable> {
    /// 키별 버전 리스트: key -> [(commit_ts, value)]
    /// 각 키에 대해 타임스탬프 내림차순으로 정렬된 버전 리스트 유지
    versions: VersionStorage<T>,

    /// 타임스탬프 오라클 참조 (선택적)
    #[allow(dead_code)]
    oracle: Option<Arc<TimestampOracle>>,
}

impl<T: Versionable> VersionManager<T> {
    /// 새로운 VersionManager를 생성합니다.
    ///
    /// # Arguments
    ///
    /// * `oracle` - 타임스탬프 오라클 (선택적)
    pub fn new(oracle: Arc<TimestampOracle>) -> Self {
        Self {
            versions: Arc::new(RwLock::new(BTreeMap::new())),
            oracle: Some(oracle),
        }
    }

    /// 타임스탬프 오라클 없이 VersionManager를 생성합니다.
    ///
    /// 이 경우 타임스탬프는 외부에서 직접 관리해야 합니다.
    pub fn new_without_oracle() -> Self {
        Self {
            versions: Arc::new(RwLock::new(BTreeMap::new())),
            oracle: None,
        }
    }

    /// 새로운 버전을 추가합니다.
    ///
    /// # Arguments
    ///
    /// * `key` - 버전 키 (동일한 엔티티의 여러 버전을 식별)
    /// * `value` - 저장할 값
    /// * `commit_ts` - 커밋 타임스탬프
    ///
    /// # Returns
    ///
    /// 성공 시 `Ok(())`, 실패 시 에러
    pub fn add_version(&self, key: Vec<u8>, value: T, commit_ts: u64) -> DbxResult<()> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        let version_list = versions.entry(key).or_insert_with(Vec::new);

        // 타임스탬프 내림차순으로 삽입 (최신 버전이 앞에 오도록)
        let insert_pos = version_list
            .binary_search_by(|(ts, _)| commit_ts.cmp(ts))
            .unwrap_or_else(|pos| pos);

        version_list.insert(insert_pos, (commit_ts, value));

        Ok(())
    }

    /// 특정 스냅샷 시점의 버전을 조회합니다.
    ///
    /// # Arguments
    ///
    /// * `key` - 조회할 키
    /// * `read_ts` - 읽기 타임스탬프 (스냅샷 시점)
    ///
    /// # Returns
    ///
    /// - `Ok(Some(value))` - 해당 시점에 보이는 값이 존재
    /// - `Ok(None)` - 해당 시점에 보이는 값이 없음
    /// - `Err(_)` - 에러 발생
    pub fn get_at_snapshot(&self, key: &[u8], read_ts: u64) -> DbxResult<Option<T>> {
        let versions = self
            .versions
            .read()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        if let Some(version_list) = versions.get(key) {
            // 타임스탬프 내림차순으로 정렬되어 있으므로
            // read_ts 이하인 첫 번째 버전을 찾음
            for (commit_ts, value) in version_list {
                if *commit_ts <= read_ts {
                    return Ok(Some(value.clone()));
                }
            }
        }

        Ok(None)
    }

    /// 가비지 컬렉션을 수행합니다.
    ///
    /// `min_active_ts`보다 오래된 버전 중 최신 버전이 아닌 것들을 삭제합니다.
    /// 각 키당 최소 1개의 버전은 유지합니다.
    ///
    /// # Arguments
    ///
    /// * `min_active_ts` - 최소 활성 타임스탬프 (이보다 오래된 버전은 GC 대상)
    ///
    /// # Returns
    ///
    /// 삭제된 버전의 개수
    pub fn collect_garbage(&self, min_active_ts: u64) -> DbxResult<usize> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        let mut deleted_count = 0;

        for (_key, version_list) in versions.iter_mut() {
            if version_list.len() <= 1 {
                // 버전이 1개 이하면 GC 불필요
                continue;
            }

            // min_active_ts보다 오래된 버전 중 최신 버전이 아닌 것들을 찾음
            let mut to_remove = Vec::new();

            for (i, (commit_ts, _)) in version_list.iter().enumerate() {
                // 첫 번째 버전(최신)은 항상 유지
                if i == 0 {
                    continue;
                }

                // min_active_ts보다 오래된 버전만 삭제 대상
                if *commit_ts < min_active_ts {
                    to_remove.push(i);
                }
            }

            // 역순으로 삭제 (인덱스 변경 방지)
            for &idx in to_remove.iter().rev() {
                version_list.remove(idx);
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    /// 특정 키의 모든 버전 개수를 반환합니다.
    pub fn version_count(&self, key: &[u8]) -> DbxResult<usize> {
        let versions = self
            .versions
            .read()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        Ok(versions.get(key).map(|v| v.len()).unwrap_or(0))
    }

    /// 전체 키 개수를 반환합니다.
    pub fn key_count(&self) -> DbxResult<usize> {
        let versions = self
            .versions
            .read()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        Ok(versions.len())
    }

    /// 전체 버전 개수를 반환합니다.
    pub fn total_version_count(&self) -> DbxResult<usize> {
        let versions = self
            .versions
            .read()
            .map_err(|e| DbxError::Storage(format!("Lock error: {}", e)))?;

        Ok(versions.values().map(|v| v.len()).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_manager_add_and_get() -> DbxResult<()> {
        let oracle = Arc::new(TimestampOracle::default());
        let manager = VersionManager::<String>::new(Arc::clone(&oracle));

        // 버전 추가
        manager.add_version(b"user:1".to_vec(), "Alice v1".to_string(), 10)?;
        manager.add_version(b"user:1".to_vec(), "Alice v2".to_string(), 20)?;
        manager.add_version(b"user:1".to_vec(), "Alice v3".to_string(), 30)?;

        // 스냅샷 조회
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 5)?,
            None // ts=5에는 아무 버전도 보이지 않음
        );
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 15)?,
            Some("Alice v1".to_string()) // ts=15에는 v1이 보임
        );
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 25)?,
            Some("Alice v2".to_string()) // ts=25에는 v2가 보임
        );
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 35)?,
            Some("Alice v3".to_string()) // ts=35에는 v3가 보임
        );

        Ok(())
    }

    #[test]
    fn test_version_manager_snapshot_isolation() -> DbxResult<()> {
        let oracle = Arc::new(TimestampOracle::default());
        let manager = VersionManager::<Vec<u8>>::new(Arc::clone(&oracle));

        // 초기 데이터
        manager.add_version(b"key1".to_vec(), b"value1".to_vec(), 10)?;

        // 스냅샷 시점 ts=15
        let snapshot_ts = 15;
        let value_at_15 = manager.get_at_snapshot(b"key1", snapshot_ts)?;
        assert_eq!(value_at_15, Some(b"value1".to_vec()));

        // 스냅샷 이후 새로운 버전 추가
        manager.add_version(b"key1".to_vec(), b"value2".to_vec(), 20)?;

        // 스냅샷 시점에는 여전히 이전 값이 보여야 함
        let value_at_15_again = manager.get_at_snapshot(b"key1", snapshot_ts)?;
        assert_eq!(value_at_15_again, Some(b"value1".to_vec()));

        // 새로운 스냅샷 시점 ts=25에는 새 값이 보임
        let value_at_25 = manager.get_at_snapshot(b"key1", 25)?;
        assert_eq!(value_at_25, Some(b"value2".to_vec()));

        Ok(())
    }

    #[test]
    fn test_version_manager_garbage_collection() -> DbxResult<()> {
        let oracle = Arc::new(TimestampOracle::default());
        let manager = VersionManager::<String>::new(Arc::clone(&oracle));

        // 여러 버전 추가
        manager.add_version(b"key1".to_vec(), "v1".to_string(), 10)?;
        manager.add_version(b"key1".to_vec(), "v2".to_string(), 20)?;
        manager.add_version(b"key1".to_vec(), "v3".to_string(), 30)?;
        manager.add_version(b"key1".to_vec(), "v4".to_string(), 40)?;

        // GC 전: 4개 버전
        assert_eq!(manager.version_count(b"key1")?, 4);

        // min_active_ts=25로 GC 수행
        // ts=10, ts=20은 삭제 대상 (ts=30, ts=40은 유지)
        let deleted = manager.collect_garbage(25)?;
        assert_eq!(deleted, 2);

        // GC 후: 2개 버전 (ts=30, ts=40)
        assert_eq!(manager.version_count(b"key1")?, 2);

        // 스냅샷 조회로 검증
        assert_eq!(
            manager.get_at_snapshot(b"key1", 15)?,
            None // ts=10, ts=20이 삭제되어 보이지 않음
        );
        assert_eq!(
            manager.get_at_snapshot(b"key1", 35)?,
            Some("v3".to_string()) // ts=30은 유지됨
        );
        assert_eq!(
            manager.get_at_snapshot(b"key1", 45)?,
            Some("v4".to_string()) // ts=40은 유지됨
        );

        Ok(())
    }

    #[test]
    fn test_version_manager_multiple_keys() -> DbxResult<()> {
        let manager = VersionManager::<String>::new_without_oracle();

        // 여러 키에 대한 버전 추가
        manager.add_version(b"user:1".to_vec(), "Alice".to_string(), 10)?;
        manager.add_version(b"user:2".to_vec(), "Bob".to_string(), 15)?;
        manager.add_version(b"user:1".to_vec(), "Alice Updated".to_string(), 20)?;

        // 키 개수 확인
        assert_eq!(manager.key_count()?, 2);

        // 전체 버전 개수 확인
        assert_eq!(manager.total_version_count()?, 3);

        // 각 키별 조회
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 12)?,
            Some("Alice".to_string())
        );
        assert_eq!(
            manager.get_at_snapshot(b"user:2", 18)?,
            Some("Bob".to_string())
        );
        assert_eq!(
            manager.get_at_snapshot(b"user:1", 25)?,
            Some("Alice Updated".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_version_manager_gc_preserves_latest() -> DbxResult<()> {
        let manager = VersionManager::<Vec<u8>>::new_without_oracle();

        // 단일 버전만 있는 경우
        manager.add_version(b"key1".to_vec(), b"value1".to_vec(), 10)?;

        // GC 수행 (min_active_ts=100으로 모든 버전이 오래됨)
        let deleted = manager.collect_garbage(100)?;

        // 최신 버전은 항상 유지되므로 삭제되지 않음
        assert_eq!(deleted, 0);
        assert_eq!(manager.version_count(b"key1")?, 1);

        Ok(())
    }
}
