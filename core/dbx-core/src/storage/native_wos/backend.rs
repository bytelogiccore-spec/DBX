//! NativeWosBackend — StorageBackend 구현 (stub, Task 3에서 완성)

use super::table_store::TableStore;
use crate::error::DbxResult;
use crate::storage::StorageBackend;
use dashmap::DashMap;
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Tier 3: 자체 구현 WOS — sled 의존 없음.
/// DashMap으로 테이블별 독립 락 제공.
pub struct NativeWosBackend {
    base_path: PathBuf,
    tables: DashMap<String, Mutex<TableStore>>,
    /// 임시 디렉토리 소유권 보관 (open_temporary 시 drop 방지)
    _temp_dir: Option<tempfile::TempDir>,
}

impl NativeWosBackend {
    pub fn open(base_path: &Path) -> DbxResult<Self> {
        std::fs::create_dir_all(base_path)?;
        Ok(Self {
            base_path: base_path.to_path_buf(),
            tables: DashMap::new(),
            _temp_dir: None,
        })
    }

    /// 임시 백엔드 (테스트 및 in-memory 모드용)
    pub fn open_temporary() -> DbxResult<Self> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().to_path_buf();
        Ok(Self {
            base_path: path,
            tables: DashMap::new(),
            _temp_dir: Some(dir), // TempDir 소유권 유지 → drop 방지
        })
    }

    fn get_or_open(&self, table: &str) -> DbxResult<()> {
        if !self.tables.contains_key(table) {
            // 테이블 이름의 파일시스템 불가 문자를 '_'로 치환
            let safe_name = table.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
            let path = self.base_path.join(format!("{safe_name}.wos"));
            // 중간 디렉토리 생성 (혹시 남은 경우를 대비)
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let store = TableStore::open(&path)?;
            self.tables.insert(table.to_string(), Mutex::new(store));
        }
        Ok(())
    }
}

impl StorageBackend for NativeWosBackend {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        self.get_or_open(table)?;
        self.tables
            .get(table)
            .unwrap()
            .lock()
            .unwrap()
            .insert(key, value)
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        self.get_or_open(table)?;
        self.tables.get(table).unwrap().lock().unwrap().get(key)
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        self.get_or_open(table)?;
        self.tables.get(table).unwrap().lock().unwrap().delete(key)
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        self.get_or_open(table)?;
        self.tables.get(table).unwrap().lock().unwrap().scan(range)
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        self.get_or_open(table)?;
        self.tables
            .get(table)
            .unwrap()
            .lock()
            .unwrap()
            .scan_one(range)
    }

    fn flush(&self) -> DbxResult<()> {
        for entry in self.tables.iter() {
            entry.value().lock().unwrap().flush()?;
        }
        Ok(())
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        self.get_or_open(table)?;
        self.tables.get(table).unwrap().lock().unwrap().count()
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        Ok(self.tables.iter().map(|e| e.key().clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_backend() -> NativeWosBackend {
        NativeWosBackend::open_temporary().unwrap()
    }

    #[test]
    fn insert_and_get() {
        let b = temp_backend();
        b.insert("users", b"key1", b"value1").unwrap();
        assert_eq!(b.get("users", b"key1").unwrap(), Some(b"value1".to_vec()));
    }

    #[test]
    fn get_nonexistent() {
        let b = temp_backend();
        assert_eq!(b.get("users", b"missing").unwrap(), None);
    }

    #[test]
    fn delete_existing() {
        let b = temp_backend();
        b.insert("users", b"key1", b"value1").unwrap();
        assert!(b.delete("users", b"key1").unwrap());
        assert_eq!(b.get("users", b"key1").unwrap(), None);
    }

    #[test]
    fn delete_nonexistent() {
        let b = temp_backend();
        assert!(!b.delete("users", b"missing").unwrap());
    }

    #[test]
    fn upsert_overwrites() {
        let b = temp_backend();
        b.insert("t", b"k", b"v1").unwrap();
        b.insert("t", b"k", b"v2").unwrap();
        assert_eq!(b.get("t", b"k").unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn scan_all() {
        let b = temp_backend();
        b.insert("t", b"a", b"1").unwrap();
        b.insert("t", b"b", b"2").unwrap();
        b.insert("t", b"c", b"3").unwrap();
        let all = b.scan("t", ..).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].0, b"a");
        assert_eq!(all[2].0, b"c");
    }

    #[test]
    fn scan_range() {
        let b = temp_backend();
        b.insert("t", b"a", b"1").unwrap();
        b.insert("t", b"b", b"2").unwrap();
        b.insert("t", b"c", b"3").unwrap();
        b.insert("t", b"d", b"4").unwrap();
        let res = b.scan("t", b"b".to_vec()..b"d".to_vec()).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, b"b");
        assert_eq!(res[1].0, b"c");
    }

    #[test]
    fn count() {
        let b = temp_backend();
        assert_eq!(b.count("t").unwrap(), 0);
        b.insert("t", b"a", b"1").unwrap();
        b.insert("t", b"b", b"2").unwrap();
        assert_eq!(b.count("t").unwrap(), 2);
    }

    #[test]
    fn table_names() {
        let b = temp_backend();
        b.insert("users", b"a", b"1").unwrap();
        b.insert("orders", b"b", b"2").unwrap();
        let mut names = b.table_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["orders".to_string(), "users".to_string()]);
    }

    #[test]
    fn multiple_tables_isolation() {
        let b = temp_backend();
        b.insert("t1", b"k", b"v1").unwrap();
        b.insert("t2", b"k", b"v2").unwrap();
        assert_eq!(b.get("t1", b"k").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(b.get("t2", b"k").unwrap(), Some(b"v2".to_vec()));
    }
}
