//! TableStore — 테이블별 in-memory BTreeMap 인덱스 + SSTable 파일 flush
//!
//! 각 테이블은 독립된 .wos 파일로 저장된다.

use crate::error::DbxResult;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use super::page::{PageEntry, WosPage};

enum EntryState {
    Cached(Vec<u8>),
    Deleted,
}

/// 단일 테이블의 영구 저장소
pub struct TableStore {
    path: PathBuf,
    index: BTreeMap<Vec<u8>, EntryState>,
    file: File,
    dirty: bool,
}

impl TableStore {
    pub fn open(path: impl AsRef<Path>) -> DbxResult<Self> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;
        let mut store = Self {
            path,
            index: BTreeMap::new(),
            file,
            dirty: false,
        };
        store.load_from_disk()?;
        Ok(store)
    }

    fn load_from_disk(&mut self) -> DbxResult<()> {
        let mut buf = Vec::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_end(&mut buf)?;
        if buf.is_empty() {
            return Ok(());
        }
        let page = WosPage::deserialize(&buf)?;
        for entry in page.entries {
            if entry.deleted {
                self.index.insert(entry.key, EntryState::Deleted);
            } else {
                self.index.insert(entry.key, EntryState::Cached(entry.value));
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> DbxResult<()> {
        self.index.insert(key.to_vec(), EntryState::Cached(value.to_vec()));
        self.dirty = true;
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        match self.index.get(key) {
            Some(EntryState::Cached(v)) => Ok(Some(v.clone())),
            _ => Ok(None),
        }
    }

    pub fn delete(&mut self, key: &[u8]) -> DbxResult<bool> {
        let existed = matches!(self.index.get(key), Some(EntryState::Cached(_)));
        self.index.insert(key.to_vec(), EntryState::Deleted);
        self.dirty = true;
        Ok(existed)
    }

    pub fn scan<R: RangeBounds<Vec<u8>>>(&self, range: R) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut result = Vec::new();
        for (k, v) in self.index.range(range) {
            if let EntryState::Cached(val) = v {
                result.push((k.clone(), val.clone()));
            }
        }
        Ok(result)
    }

    pub fn scan_one<R: RangeBounds<Vec<u8>>>(&self, range: R) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        for (k, v) in self.index.range(range) {
            if let EntryState::Cached(val) = v {
                return Ok(Some((k.clone(), val.clone())));
            }
        }
        Ok(None)
    }

    pub fn count(&self) -> usize {
        self.index
            .values()
            .filter(|v| matches!(v, EntryState::Cached(_)))
            .count()
    }

    pub fn flush(&mut self) -> DbxResult<()> {
        if !self.dirty {
            return Ok(());
        }
        let entries: Vec<PageEntry> = self
            .index
            .iter()
            .map(|(k, v)| PageEntry {
                key: k.clone(),
                value: if let EntryState::Cached(val) = v { val.clone() } else { vec![] },
                deleted: matches!(v, EntryState::Deleted),
            })
            .collect();
        let page = WosPage::from_entries(entries);
        let bytes = page.serialize()?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.write_all(&bytes)?;
        self.file.sync_all()?;
        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn tmp_store() -> (TableStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = TableStore::open(dir.path().join("t.wos")).unwrap();
        (store, dir) // dir은 TempDir을 살려두기 위해 함께 반환
    }

    #[test]
    fn insert_and_get() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"k1", b"v1").unwrap();
        assert_eq!(s.get(b"k1").unwrap(), Some(b"v1".to_vec()));
    }

    #[test]
    fn get_nonexistent() {
        let (s, _dir) = tmp_store();
        assert_eq!(s.get(b"missing").unwrap(), None);
    }

    #[test]
    fn delete_returns_true() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"k1", b"v1").unwrap();
        assert!(s.delete(b"k1").unwrap());
        assert_eq!(s.get(b"k1").unwrap(), None);
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let (mut s, _dir) = tmp_store();
        assert!(!s.delete(b"missing").unwrap());
    }

    #[test]
    fn scan_range_ordered() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"c", b"3").unwrap();
        s.insert(b"a", b"1").unwrap();
        s.insert(b"b", b"2").unwrap();
        let res = s.scan(b"a".to_vec()..b"c".to_vec()).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, b"a");
        assert_eq!(res[1].0, b"b");
    }

    #[test]
    fn count_excludes_deleted() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"a", b"1").unwrap();
        s.insert(b"b", b"2").unwrap();
        s.delete(b"a").unwrap();
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn persist_and_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"key", b"val").unwrap();
            s.flush().unwrap();
        }
        {
            let s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(b"key").unwrap(), Some(b"val".to_vec()));
        }
    }

    #[test]
    fn deleted_not_reloaded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"k", b"v").unwrap();
            s.delete(b"k").unwrap();
            s.flush().unwrap();
        }
        {
            let s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(b"k").unwrap(), None);
        }
    }
}
