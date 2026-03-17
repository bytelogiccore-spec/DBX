//! TableStore — 다중 4KB 페이지 + 스파스 인덱스 + LRU 페이지 캐시 기반 SSTable
//!
//! ## 파일 포맷
//! ```text
//! [Page 0][Page 1]...[Page N] | [SparseIndex] | [Footer: index_offset(8) + page_count(4) + MAGIC(4)]
//! ```
//!
//! - **Page**: 정렬된 key-value 엔트리 묶음, 최대 `PAGE_TARGET_BYTES` bytes
//! - **SparseIndex**: 각 페이지의 첫 키(key_len:4 + key) + page 파일 오프셋(8) 목록
//! - **Footer**: index 시작 오프셋(u64) + 페이지 수(u32) + 매직(u32)
//!
//! ## Scan 최적화
//! - 스파스 인덱스로 시작 페이지를 binary search
//! - `BufReader`로 연속 페이지를 sequential read
//! - `PageCache` (LRU): hot page를 메모리에 보관, disk I/O 제거
//! - 메모리에는 dirty buffer(flush 전 변경분)만 BTreeMap으로 유지

use crate::error::{DbxError, DbxResult};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::ops::RangeBounds;
use std::path::{Path, PathBuf};
use super::page::{PageEntry, WosPage};

/// 페이지 목표 크기 (bytes). 엔트리가 이 크기를 넘으면 새 페이지 시작.
const PAGE_TARGET_BYTES: usize = 4096;

/// 파일 Footer의 매직 (WOSS = WOS SSTable)
const FOOTER_MAGIC: u32 = 0x574F_5353;
/// Footer 고정 크기: index_offset(8) + page_count(4) + magic(4)
const FOOTER_SIZE: u64 = 16;

/// 스파스 인덱스 항목: 각 페이지의 첫 키와 파일 오프셋
#[derive(Debug)]
struct IndexEntry {
    first_key: Vec<u8>,
    file_offset: u64,
}

// ──────────────────────────────────────────
// LRU Page Cache
// ──────────────────────────────────────────

/// LRU 페이지 캐시 — page_index(usize) → Vec<PageEntry>
///
/// - `capacity`: 최대 보관 페이지 수 (기본 64 = ~256KB)
/// - `map`: page_idx → entries (O(1) 접근)
/// - `order`: MRU 순서 VecDeque (front = 최근, back = 가장 오래됨)
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

    /// 캐시에서 page_idx 조회. 존재하면 MRU로 이동 후 반환.
    fn get(&mut self, page_idx: usize) -> Option<&Vec<PageEntry>> {
        if !self.map.contains_key(&page_idx) {
            return None;
        }
        // order에서 해당 항목을 front로 이동
        if let Some(pos) = self.order.iter().position(|&x| x == page_idx) {
            self.order.remove(pos);
        }
        self.order.push_front(page_idx);
        self.map.get(&page_idx)
    }

    /// 페이지를 캐시에 삽입. capacity 초과 시 LRU (back) 제거.
    fn insert(&mut self, page_idx: usize, entries: Vec<PageEntry>) {
        if self.map.contains_key(&page_idx) {
            if let Some(pos) = self.order.iter().position(|&x| x == page_idx) {
                self.order.remove(pos);
            }
        } else if self.map.len() >= self.capacity {
            // LRU 제거
            if let Some(lru) = self.order.pop_back() {
                self.map.remove(&lru);
            }
        }
        self.map.insert(page_idx, entries);
        self.order.push_front(page_idx);
    }

    /// flush 후 캐시 전체 무효화
    fn invalidate(&mut self) {
        self.map.clear();
        self.order.clear();
    }

    fn len(&self) -> usize { self.map.len() }
}

enum DirtyState {
    Put(Vec<u8>),
    Delete,
}

/// 단일 테이블의 SSTable 저장소.
///
/// - `dirty`: flush 전 in-memory 변경분
/// - `page_index`: flush 된 SSTable의 스파스 인덱스 (page 0..N의 첫 키)
/// - `page_cache`: LRU hot-page cache (디스크 읽기 횟수 감소)
/// - `file`: .wos 파일 핸들
pub struct TableStore {
    path: PathBuf,
    dirty: BTreeMap<Vec<u8>, DirtyState>,
    page_index: Vec<IndexEntry>,
    page_cache: PageCache,
    file: File,
    has_flushed_data: bool,
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
            dirty: BTreeMap::new(),
            page_index: Vec::new(),
            page_cache: PageCache::new(64), // LRU 64페이지 = ~256KB
            file,
            has_flushed_data: false,
        };
        store.load_index()?;
        Ok(store)
    }

    // ──────────────────────────────────────────
    // Index Load
    // ──────────────────────────────────────────

    fn load_index(&mut self) -> DbxResult<()> {
        let file_len = self.file.seek(SeekFrom::End(0))?;
        if file_len < FOOTER_SIZE {
            return Ok(());
        }

        // Read footer
        self.file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let mut footer = [0u8; 16];
        self.file.read_exact(&mut footer)?;

        let index_offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let page_count = u32::from_le_bytes(footer[8..12].try_into().unwrap()) as usize;
        let magic = u32::from_le_bytes(footer[12..16].try_into().unwrap());
        if magic != FOOTER_MAGIC {
            return Err(DbxError::Storage("TableStore: invalid footer magic".into()));
        }

        // Read sparse index
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
    // Page Read (from disk)
    // ──────────────────────────────────────────

    /// 인덱스 항목 i번째 페이지의 바이트 범위를 반환
    fn page_file_range(&self, i: usize) -> (u64, u64) {
        let start = self.page_index[i].file_offset;
        let end = if i + 1 < self.page_index.len() {
            self.page_index[i + 1].file_offset
        } else {
            // 마지막 페이지: index_offset = self.page_index ends before sparse index
            // index 시작 위치 = page_index.last().file_offset + page_size...
            // 실제로는 sparse index가 페이지 바로 다음에 위치하므로
            // file_len - FOOTER_SIZE - index_size
            // 간단하게: 다음 페이지가 없으면 sparse index 직전까지 = index_offset
            // 우리는 index_offset을 직접 저장하지 않았으므로 page_index[last]의 offset을
            // 이용해 파일에서 페이지 크기를 읽어야 한다.
            // 편의상: 다음 seek 전에 읽어야 할 바이트를 모른다 → full page read fallback
            0 // 특별 케이스: 아래에서 처리
        };
        (start, end)
    }

    fn read_page_at(&mut self, page_idx: usize) -> DbxResult<WosPage> {
        if page_idx >= self.page_index.len() {
            return Err(DbxError::Storage("page index out of range".into()));
        }

        // ★ LRU 캐시 조회 (cache hit 시 disk I/O 없음)
        if self.page_cache.get(page_idx).is_some() {
            let entries = self.page_cache.map.get(&page_idx).unwrap().clone();
            return Ok(WosPage { entries });
        }

        let start = self.page_index[page_idx].file_offset;

        // 끝 위치 결정
        let end = if page_idx + 1 < self.page_index.len() {
            self.page_index[page_idx + 1].file_offset
        } else {
            // 마지막 페이지: sparse index가 바로 다음에 옴
            self.file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
            let mut footer = [0u8; 16];
            self.file.read_exact(&mut footer)?;
            u64::from_le_bytes(footer[0..8].try_into().unwrap()) // index_offset
        };

        let size = (end - start) as usize;
        let mut buf = vec![0u8; size];
        self.file.seek(SeekFrom::Start(start))?;
        self.file.read_exact(&mut buf)?;
        let page = WosPage::deserialize(&buf)?;

        // ★ LRU 캐시에 저장 (cache miss 후 eviction 포함)
        self.page_cache.insert(page_idx, page.entries.clone());

        Ok(page)
    }

    /// 스파스 인덱스에서 `key`가 속할 가능성 있는 첫 페이지 인덱스를 반환.
    /// binary search: `first_key <= key`인 가장 큰 i.
    fn find_page_for_key(&self, key: &[u8]) -> Option<usize> {
        if self.page_index.is_empty() {
            return None;
        }
        // 마지막으로 first_key <= key인 페이지
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
    // Public API
    // ──────────────────────────────────────────

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> DbxResult<()> {
        self.dirty.insert(key.to_vec(), DirtyState::Put(value.to_vec()));
        Ok(())
    }

    pub fn get(&mut self, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // 1. dirty buffer 우선
        if let Some(state) = self.dirty.get(key) {
            return match state {
                DirtyState::Put(v) => Ok(Some(v.clone())),
                DirtyState::Delete => Ok(None),
            };
        }

        // 2. 디스크에서 해당 페이지만 읽기
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
        // ── Fast-path: flush 전 (in-memory only) ──────────────────────────
        // disk I/O 불필요. BTreeMap range 직접 사용 = O(n) 이전 동작과 동일.
        if !self.has_flushed_data {
            return Ok(self
                .dirty
                .range(range)
                .filter_map(|(k, s)| match s {
                    DirtyState::Put(v) => Some((k.clone(), v.clone())),
                    DirtyState::Delete => None,
                })
                .collect());
        }

        // ── Slow-path: flushed data + dirty 병합 ──────────────────────────
        let mut merged: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        // range 시작 페이지부터 BufReader sequential read
        let start_page = match range.start_bound() {
            std::ops::Bound::Included(k) | std::ops::Bound::Excluded(k) => {
                self.find_page_for_key(k).unwrap_or(0)
            }
            std::ops::Bound::Unbounded => 0,
        };

        // footer에서 index_offset 읽기
        self.file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let mut footer_buf = [0u8; 16];
        self.file.read_exact(&mut footer_buf)?;
        let index_offset = u64::from_le_bytes(footer_buf[0..8].try_into().unwrap());

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
                if range.contains(&k) {
                    merged.insert(k, entry.value);
                }
            }
            page_i += 1;
        }

        // dirty buffer overlay
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
        // 전체 scan으로 계산 (full-table count는 드문 연산)
        Ok(self.scan(..)?.len())
    }

    // ──────────────────────────────────────────
    // Flush — 4KB 페이지로 분할 후 스파스 인덱스 기록
    // ──────────────────────────────────────────

    pub fn flush(&mut self) -> DbxResult<()> {
        // 기존 데이터 + dirty 병합
        let mut all: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();

        // 이미 flush된 데이터 로드
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

        // dirty overlay
        for (k, state) in &self.dirty {
            match state {
                DirtyState::Put(v) => { all.insert(k.clone(), v.clone()); }
                DirtyState::Delete => { all.remove(k); }
            }
        }

        if self.dirty.is_empty() {
            return Ok(());
        }

        // 4KB 페이지로 분할
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
            current_page.push(PageEntry {
                key: k.clone(),
                value: v.clone(),
                deleted: false,
            });
        }
        if !current_page.is_empty() {
            pages.push(current_page);
        }

        // 파일 재작성
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;

        let mut page_index: Vec<IndexEntry> = Vec::with_capacity(pages.len());
        let mut offset: u64 = 0;

        for page_entries in pages {
            let first_key = page_entries[0].key.clone();
            let page = WosPage::from_entries(page_entries);
            let bytes = page.serialize()?;
            page_index.push(IndexEntry { first_key, file_offset: offset });
            self.file.write_all(&bytes)?;
            offset += bytes.len() as u64;
        }

        // Sparse Index 기록
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
        self.page_cache.invalidate(); // flush 후 캐시 무효화 (페이지 레이아웃 변경됨)
        self.dirty.clear();
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
    fn persist_and_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            s.insert(b"key", b"val").unwrap();
            s.flush().unwrap();
        }
        {
            let mut s = TableStore::open(&path).unwrap();
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
            let mut s = TableStore::open(&path).unwrap();
            assert_eq!(s.get(b"k").unwrap(), None);
        }
    }

    #[test]
    fn multi_page_scan() {
        let (mut s, _dir) = tmp_store();
        // 4KB를 넘겨서 다중 페이지를 강제
        for i in 0..200u32 {
            let key = format!("key{:05}", i).into_bytes();
            let val = format!("value{:05}", i).into_bytes();
            s.insert(&key, &val).unwrap();
        }
        s.flush().unwrap();
        assert!(s.page_index.len() > 1, "should have multiple pages");
        let all = s.scan(..).unwrap();
        assert_eq!(all.len(), 200);
        // 정렬 순서 확인
        for i in 1..all.len() {
            assert!(all[i-1].0 < all[i].0);
        }
    }

    #[test]
    fn cross_page_range_scan() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.wos");
        {
            let mut s = TableStore::open(&path).unwrap();
            for i in 0..200u32 {
                let key = format!("key{:05}", i).into_bytes();
                s.insert(&key, b"v").unwrap();
            }
            s.flush().unwrap();
        }
        {
            let mut s = TableStore::open(&path).unwrap();
            // key00050..key00100 범위 스캔
            let res = s.scan(b"key00050".to_vec()..b"key00100".to_vec()).unwrap();
            assert_eq!(res.len(), 50);
            assert_eq!(res[0].0, b"key00050");
            assert_eq!(res[49].0, b"key00099");
        }
    }

    #[test]
    fn dirty_overlay_after_flush() {
        let (mut s, _dir) = tmp_store();
        s.insert(b"a", b"old").unwrap();
        s.flush().unwrap();
        // flush 후 dirty 업데이트
        s.insert(b"a", b"new").unwrap();
        assert_eq!(s.get(b"a").unwrap(), Some(b"new".to_vec()));
    }
}
