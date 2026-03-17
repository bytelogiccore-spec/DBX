//! TableStore — WAL + 다중 4KB 페이지 + 스파스 인덱스 + LRU 캐시
//!
//! ## 개요
//! ```text
//! 쓰기: insert() → dirty (메모리)
//!       flush()  → WAL 파일에 append (빠름, O(dirty))
//!       compact()→ WAL + SSTable 병합 → 새 SSTable (느림, 드물게 발생)
//!
//! 읽기: get/scan → dirty → wal_entries → SSTable (LRU cache)
//! ```
//!
//! ## 파일 구조
//! ```
//! bench.wos  — SSTable (compact된 페이지 + 스파스 인덱스 + footer)
//! bench.wal  — WAL log (순차 append, compact 시 truncate)
//! ```
//!
//! ## WAL compact 조건
//! `wal_entries.len() >= WAL_COMPACT_THRESHOLD` (기본 5000)

use crate::error::{DbxError, DbxResult};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use super::page::{PageEntry, WosPage};
use super::wal::{WalRecord, replay_wal};

// ──────────────────────────────────────────
// 상수
// ──────────────────────────────────────────

/// SSTable 페이지 목표 크기 (bytes)
const PAGE_TARGET_BYTES: usize = 4096;

/// WAL footer magic
const FOOTER_MAGIC: u32 = 0x574F_5353;
/// Footer 고정 크기: index_offset(8) + page_count(4) + magic(4)
const FOOTER_SIZE: u64 = 16;

/// WAL 항목이 이 수를 넘으면 compact() 자동 호출
const WAL_COMPACT_THRESHOLD: usize = 5_000;

// ──────────────────────────────────────────
// LRU Page Cache
// ──────────────────────────────────────────

struct PageCache {
    capacity: usize,
    map: HashMap<usize, Vec<PageEntry>>,
    order: VecDeque<usize>,
}

impl PageCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    fn get(&mut self, page_idx: usize) -> Option<&Vec<PageEntry>> {
        if !self.map.contains_key(&page_idx) {
            return None;
        }
        if let Some(pos) = self.order.iter().position(|&x| x == page_idx) {
            self.order.remove(pos);
        }
        self.order.push_front(page_idx);
        self.map.get(&page_idx)
    }

    fn insert(&mut self, page_idx: usize, entries: Vec<PageEntry>) {
        if self.map.contains_key(&page_idx) {
            if let Some(pos) = self.order.iter().position(|&x| x == page_idx) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.capacity {
            if let Some(lru) = self.order.pop_back() {
                self.map.remove(&lru);
            }
        }
        self.map.insert(page_idx, entries);
        self.order.push_front(page_idx);
    }

    fn invalidate(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

// ──────────────────────────────────────────
// SSTable Sparse Index
// ──────────────────────────────────────────

#[derive(Debug)]
struct IndexEntry {
    first_key: Vec<u8>,
    file_offset: u64,
}

// ──────────────────────────────────────────
// Dirty State
// ──────────────────────────────────────────

enum DirtyState {
    Put(Vec<u8>),
    Delete,
}

// ──────────────────────────────────────────
// TableStore
// ──────────────────────────────────────────

/// 단일 테이블의 WAL + SSTable 저장소.
///
/// - `dirty`: flush 전 in-memory 변경분 (crash 시 소실)
/// - `wal_entries`: WAL에 기록됐지만 아직 SSTable에 compact 안 된 항목
/// - `wal_file`: `.wal` 파일 핸들 (append-only)
/// - `page_index`: SSTable 스파스 인덱스
/// - `page_cache`: LRU hot-page cache
/// - `file`: `.wos` SSTable 파일 핸들
pub struct TableStore {
    path: PathBuf,           // .wos 경로
    wal_path: PathBuf,       // .wal 경로

    dirty: BTreeMap<Vec<u8>, DirtyState>,       // 아직 WAL에도 안 씀
    wal_entries: BTreeMap<Vec<u8>, DirtyState>, // WAL에 씀, SSTable엔 없음

    wal_file: File,
    page_index: Vec<IndexEntry>,
    page_cache: PageCache,
    file: File,
    has_flushed_data: bool,   // SSTable에 데이터가 있는지
}

impl TableStore {
    pub fn open(path: impl AsRef<Path>) -> DbxResult<Self> {
        let path = path.as_ref().to_path_buf();
        let wal_path = path.with_extension("wal");

        let file = OpenOptions::new()
            .read(true).write(true).create(true)
            .open(&path)?;

        let wal_file = OpenOptions::new()
            .read(true).write(true).create(true).append(true)
            .open(&wal_path)?;

        let mut store = Self {
            path,
            wal_path,
            dirty: BTreeMap::new(),
            wal_entries: BTreeMap::new(),
            wal_file,
            page_index: Vec::new(),
            page_cache: PageCache::new(64),
            file,
            has_flushed_data: false,
        };
        store.load_index()?;
        store.replay_wal_file()?;
        Ok(store)
    }

    // ──────────────────────────────────────────
    // SSTable Index Load
    // ──────────────────────────────────────────

    fn load_index(&mut self) -> DbxResult<()> {
        let file_len = self.file.seek(SeekFrom::End(0))?;
        if file_len < FOOTER_SIZE {
            return Ok(());
        }
        self.file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let mut footer = [0u8; 16];
        self.file.read_exact(&mut footer)?;

        let index_offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let page_count = u32::from_le_bytes(footer[8..12].try_into().unwrap()) as usize;
        let magic = u32::from_le_bytes(footer[12..16].try_into().unwrap());
        if magic != FOOTER_MAGIC {
            return Err(DbxError::Storage("TableStore: invalid footer magic".into()));
        }
        self.file.seek(SeekFrom::Start(index_offset))?;
        let index_size = (file_len - FOOTER_SIZE - index_offset) as usize;
        let mut index_buf = vec![0u8; index_size];
        self.file.read_exact(&mut index_buf)?;

        let mut cur = 0;
        let mut entries = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            let klen = u32::from_le_bytes(index_buf[cur..cur+4].try_into().unwrap()) as usize;
            cur += 4;
            let first_key = index_buf[cur..cur+klen].to_vec();
            cur += klen;
            let file_offset = u64::from_le_bytes(index_buf[cur..cur+8].try_into().unwrap());
            cur += 8;
            entries.push(IndexEntry { first_key, file_offset });
        }
        self.page_index = entries;
        self.has_flushed_data = !self.page_index.is_empty();
        Ok(())
    }

    // ──────────────────────────────────────────
    // WAL Replay (open 시)
    // ──────────────────────────────────────────

    fn replay_wal_file(&mut self) -> DbxResult<()> {
        self.wal_file.seek(SeekFrom::Start(0))?;
        let mut reader = BufReader::new(&self.wal_file);
        let records = replay_wal(&mut reader);
        for r in records {
            let state = if r.deleted {
                DirtyState::Delete
            } else {
                DirtyState::Put(r.value)
            };
            self.wal_entries.insert(r.key, state);
        }
        // wal_file을 append 위치로 복귀
        self.wal_file.seek(SeekFrom::End(0))?;
        Ok(())
    }

    // ──────────────────────────────────────────
    // SSTable Page Read (LRU 캐시)
    // ──────────────────────────────────────────

    fn read_index_offset(&mut self) -> DbxResult<u64> {
        self.file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let mut footer = [0u8; 16];
        self.file.read_exact(&mut footer)?;
        Ok(u64::from_le_bytes(footer[0..8].try_into().unwrap()))
    }

    fn read_page_at(&mut self, page_idx: usize) -> DbxResult<WosPage> {
        if page_idx >= self.page_index.len() {
            return Err(DbxError::Storage("page index out of range".into()));
        }
        // LRU 캐시 조회
        if self.page_cache.get(page_idx).is_some() {
            let entries = self.page_cache.map.get(&page_idx).unwrap().clone();
            return Ok(WosPage { entries });
        }
        let start = self.page_index[page_idx].file_offset;
        let end = if page_idx + 1 < self.page_index.len() {
            self.page_index[page_idx + 1].file_offset
        } else {
            self.read_index_offset()?
        };
        let size = (end - start) as usize;
        let mut buf = vec![0u8; size];
        self.file.seek(SeekFrom::Start(start))?;
        self.file.read_exact(&mut buf)?;
        let page = WosPage::deserialize(&buf)?;
        self.page_cache.insert(page_idx, page.entries.clone());
        Ok(page)
    }

    fn find_page_for_key(&self, key: &[u8]) -> Option<usize> {
        if self.page_index.is_empty() { return None; }
        let mut lo = 0usize;
        let mut hi = self.page_index.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.page_index[mid].first_key.as_slice() <= key {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 { None } else { Some(lo - 1) }
    }

    // ──────────────────────────────────────────
    // WAL Compact (SSTable + WAL → 새 SSTable)
    // ──────────────────────────────────────────

    fn compact(&mut self) -> DbxResult<()> {
        // 0. dirty를 먼저 wal_entries에 병합 (flush 없이 compact 호출 시 데이터 손실 방지)
        for (k, state) in std::mem::take(&mut self.dirty) {
            self.wal_entries.insert(k, state);
        }

        // 1. SSTable 전체 로드
        let mut all: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        if self.has_flushed_data {
            for pi in 0..self.page_index.len() {
                let page = self.read_page_at(pi)?;
                for entry in page.entries {
                    if !entry.deleted {
                        all.insert(entry.key, entry.value);
                    }
                }
            }
        }
        // 2. WAL entries overlay (dirty 포함)
        for (k, state) in &self.wal_entries {
            match state {
                DirtyState::Put(v) => { all.insert(k.clone(), v.clone()); }
                DirtyState::Delete => { all.remove(k); }
            }
        }
        // 3. 4KB 페이지로 분할 후 재작성
        self.write_sstable(all)?;
        // 4. WAL 클리어 — 파일을 truncate mode로 재오픈 (Windows 호환)
        drop(std::mem::replace(
            &mut self.wal_file,
            OpenOptions::new()
                .read(true).write(true).create(true).truncate(true)
                .open(&self.wal_path)?,
        ));
        // 이후 append를 위해 append 모드로 재오픈
        self.wal_file = OpenOptions::new()
            .read(true).write(true).create(true).append(true)
            .open(&self.wal_path)?;
        self.wal_entries.clear();
        self.page_cache.invalidate();
        Ok(())
    }

    fn write_sstable(&mut self, all: BTreeMap<Vec<u8>, Vec<u8>>) -> DbxResult<()> {
        let mut pages: Vec<Vec<PageEntry>> = Vec::new();
        let mut current_page: Vec<PageEntry> = Vec::new();
        let mut current_size: usize = 0;
        for (k, v) in &all {
            let entry_size = 4 + k.len() + 4 + v.len() + 1;
            if current_size > 0 && current_size + entry_size > PAGE_TARGET_BYTES {
                pages.push(std::mem::take(&mut current_page));
                current_size = 0;
            }
            current_size += entry_size;
            current_page.push(PageEntry { key: k.clone(), value: v.clone(), deleted: false });
        }
        if !current_page.is_empty() { pages.push(current_page); }

        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;

        let mut page_index: Vec<IndexEntry> = Vec::with_capacity(pages.len());
        let mut offset: u64 = 0;
        for page_entries in pages {
            let first_key = page_entries[0].key.clone();
            let bytes = WosPage::from_entries(page_entries).serialize()?;
            page_index.push(IndexEntry { first_key, file_offset: offset });
            self.file.write_all(&bytes)?;
            offset += bytes.len() as u64;
        }
        // Sparse Index
        let index_offset = offset;
        let mut index_buf: Vec<u8> = Vec::new();
        for entry in &page_index {
            index_buf.extend_from_slice(&(entry.first_key.len() as u32).to_le_bytes());
            index_buf.extend_from_slice(&entry.first_key);
            index_buf.extend_from_slice(&entry.file_offset.to_le_bytes());
        }
        self.file.write_all(&index_buf)?;
        // Footer
        let mut footer = [0u8; 16];
        footer[0..8].copy_from_slice(&index_offset.to_le_bytes());
        footer[8..12].copy_from_slice(&(page_index.len() as u32).to_le_bytes());
        footer[12..16].copy_from_slice(&FOOTER_MAGIC.to_le_bytes());
        self.file.write_all(&footer)?;
        self.file.sync_all()?;

        self.page_index = page_index;
        self.has_flushed_data = !self.page_index.is_empty();
        Ok(())
    }

    // ──────────────────────────────────────────
    // Public API
    // ──────────────────────────────────────────

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> DbxResult<()> {
        self.dirty.insert(key.to_vec(), DirtyState::Put(value.to_vec()));
        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // 1. dirty (가장 최신)
        if let Some(state) = self.dirty.get(key) {
            return match state {
                DirtyState::Put(v) => Ok(Some(v.clone())),
                DirtyState::Delete => Ok(None),
            };
        }
        // 2. wal_entries (flush됐지만 compact 안됨)
        if let Some(state) = self.wal_entries.get(key) {
            return match state {
                DirtyState::Put(v) => Ok(Some(v.clone())),
                DirtyState::Delete => Ok(None),
            };
        }
        // 3. SSTable (compact된 것, LRU 캐시)
        if let Some(page_idx) = self.find_page_for_key(key) {
            let page = self.read_page_at(page_idx)?;
            for entry in &page.entries {
                if entry.key == key {
                    return if entry.deleted { Ok(None) } else { Ok(Some(entry.value.clone())) };
                }
            }
        }
        Ok(None)
    }

    pub fn delete(&mut self, key: &[u8]) -> DbxResult<bool> {
        let existed = self.get(key)?.is_some();
        self.dirty.insert(key.to_vec(), DirtyState::Delete);
        Ok(existed)
    }

    pub fn scan<R: RangeBounds<Vec<u8>>>(&mut self, range: R) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Fast-path: SSTable도 WAL도 없음 → dirty만
        if !self.has_flushed_data && self.wal_entries.is_empty() {
            return Ok(self.dirty.range(range)
                .filter_map(|(k, s)| match s {
                    DirtyState::Put(v) => Some((k.clone(), v.clone())),
                    DirtyState::Delete => None,
                })
                .collect());
        }

        // Slow-path: SSTable sequential read → wal_entries overlay → dirty overlay
        let mut merged: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        if self.has_flushed_data {
            let start_page = match range.start_bound() {
                std::ops::Bound::Included(k) | std::ops::Bound::Excluded(k) => {
                    self.find_page_for_key(k).unwrap_or(0)
                }
                std::ops::Bound::Unbounded => 0,
            };
            let index_offset = self.read_index_offset()?;
            let start_offset = self.page_index[start_page].file_offset;
            self.file.seek(SeekFrom::Start(start_offset))?;
            let mut reader = BufReader::with_capacity(
                64 * 1024,
                (&self.file).take(index_offset - start_offset),
            );
            let mut reached_end = false;
            let mut page_i = start_page;
            while !reached_end && page_i < self.page_index.len() {
                let page_end = if page_i + 1 < self.page_index.len() {
                    self.page_index[page_i + 1].file_offset
                } else {
                    index_offset
                };
                let page_size = (page_end - self.page_index[page_i].file_offset) as usize;
                let mut buf = vec![0u8; page_size];
                reader.read_exact(&mut buf)?;
                let page = WosPage::deserialize(&buf)?;
                for entry in page.entries {
                    if entry.deleted { continue; }
                    let k = entry.key;
                    let past_end = match range.end_bound() {
                        std::ops::Bound::Included(end) => k.as_slice() > end.as_slice(),
                        std::ops::Bound::Excluded(end) => k.as_slice() >= end.as_slice(),
                        std::ops::Bound::Unbounded => false,
                    };
                    if past_end { reached_end = true; break; }
                    if range.contains(&k) { merged.insert(k, entry.value); }
                }
                page_i += 1;
            }
        }

        // wal_entries overlay — BTreeMap range로 순회 (range 조건 수동 체크)
        for (k, state) in &self.wal_entries {
            if !range.contains(k) { continue; }
            match state {
                DirtyState::Put(v) => { merged.insert(k.clone(), v.clone()); }
                DirtyState::Delete => { merged.remove(k); }
            }
        }

        // dirty overlay
        for (k, state) in self.dirty.range(range) {
            match state {
                DirtyState::Put(v) => { merged.insert(k.clone(), v.clone()); }
                DirtyState::Delete => { merged.remove(k); }
            }
        }

        Ok(merged.into_iter().collect())
    }

    pub fn scan_one<R: RangeBounds<Vec<u8>>>(&mut self, range: R) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        Ok(self.scan(range)?.into_iter().next())
    }

    pub fn count(&mut self) -> DbxResult<usize> {
        Ok(self.scan(..)?.len())
    }

    /// WAL에 dirty entries를 sequential append (빠름 — read 없음)
    /// WAL이 threshold를 넘으면 자동으로 compact() 호출
    pub fn flush(&mut self) -> DbxResult<()> {
        if self.dirty.is_empty() {
            return Ok(());
        }
        // WAL append (sequential write only)
        for (k, state) in &self.dirty {
            let (val, deleted) = match state {
                DirtyState::Put(v) => (v.as_slice(), false),
                DirtyState::Delete => (b"".as_ref(), true),
            };
            let record = WalRecord { key: k.clone(), value: val.to_vec(), deleted };
            self.wal_file.write_all(&record.encode())?;
        }
        self.wal_file.sync_all()?;

        // dirty → wal_entries 이동
        for (k, state) in std::mem::take(&mut self.dirty) {
            self.wal_entries.insert(k, state);
        }

        // WAL이 너무 크면 compact
        if self.wal_entries.len() >= WAL_COMPACT_THRESHOLD {
            self.compact()?;
        }
        Ok(())
    }
}

// ──────────────────────────────────────────
// 스캔 헬퍼: Bound cloning workaround
// ──────────────────────────────────────────

trait BoundClone {
    fn cloned(self) -> std::ops::Bound<Vec<u8>>;
}
impl BoundClone for std::ops::Bound<&Vec<u8>> {
    fn cloned(self) -> std::ops::Bound<Vec<u8>> {
        match self {
            std::ops::Bound::Included(k) => std::ops::Bound::Included(k.clone()),
            std::ops::Bound::Excluded(k) => std::ops::Bound::Excluded(k.clone()),
            std::ops::Bound::Unbounded => std::ops::Bound::Unbounded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn tmp_store() -> (TableStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = TableStore::open(dir.path().join("t.wos")).unwrap();
        (store, dir)
    }

    #[test]
    fn insert_and_get() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"k1", b"v1").unwrap();
        assert_eq!(s.get(b"k1").unwrap(), Some(b"v1".to_vec()));
    }

    #[test]
    fn get_nonexistent() {
        let (mut s, _dir) = tmp_store();
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
        assert_eq!(s.count().unwrap(), 1);
    }

    #[test]
    fn wal_flush_and_reload() {
        // flush 후 재오픈 시 WAL이 replay 되는지 확인
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"key", b"val").unwrap();
            s.flush().unwrap(); // WAL append
            // compact 안 함 → .wos 파일에는 아무것도 없음
        }
        {
            let mut s = TableStore::open(&path).unwrap(); // WAL replay
            assert_eq!(s.get(b"key").unwrap(), Some(b"val".to_vec()));
        }
    }

    #[test]
    fn wal_delete_replayed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"k", b"v").unwrap();
            s.flush().unwrap();
            s.delete(b"k").unwrap();
            s.flush().unwrap();
        }
        {
            let mut s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(b"k").unwrap(), None);
        }
    }

    #[test]
    fn compact_merges_wal_and_sstable() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            // SSTable에 compact
            for i in 0u8..50 {
                s.insert(&[i], &[i]).unwrap();
            }
            s.compact().unwrap();
            // WAL에 추가
            for i in 50u8..100 {
                s.insert(&[i], &[i]).unwrap();
            }
            s.flush().unwrap();
        }
        {
            let mut s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(&[0u8]).unwrap(), Some(vec![0u8]));
            assert_eq!(s.get(&[99u8]).unwrap(), Some(vec![99u8]));
            assert_eq!(s.count().unwrap(), 100);
        }
    }

    #[test]
    fn persist_and_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"key", b"val").unwrap();
            s.compact().unwrap(); // SSTable에 쓰기
        }
        {
            let mut s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(b"key").unwrap(), Some(b"val".to_vec()));
        }
    }

    #[test]
    fn multi_page_scan() {
        let (mut s, _dir) = tmp_store();
        for i in 0..200u32 {
            let key = format!("key{:05}", i).into_bytes();
            // 30 bytes value → 200 × (8+4+30+4+1) ≈ 9400 bytes > 4KB, 2+ pages
            let val = format!("{:030}", i).into_bytes();
            s.insert(&key, &val).unwrap();
        }
        s.compact().unwrap();
        assert!(s.page_index.len() > 1, "expected >1 pages, got {}", s.page_index.len());
        assert_eq!(s.scan(..).unwrap().len(), 200);
    }

    #[test]
    fn cross_page_range_scan() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            for i in 0..200u32 {
                // 30 bytes value → 2+ pages
                s.insert(format!("key{:05}", i).as_bytes(),
                          format!("{:030}", i).as_bytes()).unwrap();
            }
            s.compact().unwrap();
        }
        {
            let mut s = TableStore::open(&path).unwrap();
            let res = s.scan(b"key00050".to_vec()..b"key00100".to_vec()).unwrap();
            assert_eq!(res.len(), 50);
        }
    }

    #[test]
    fn dirty_overlay_after_wal_flush() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"a", b"old").unwrap();
        s.flush().unwrap(); // WAL에 기록
        s.insert(b"a", b"new").unwrap(); // dirty에 덮어씀
        assert_eq!(s.get(b"a").unwrap(), Some(b"new".to_vec()));
    }
}
