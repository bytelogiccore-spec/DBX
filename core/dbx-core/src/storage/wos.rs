//! WOS (Write-Optimized Store) — Tier 3: sled-backed durable row storage.
//!
//! Provides persistent key-value storage with B+Tree indexing (~10μs latency).
//! Each table maps to a separate sled `Tree`.

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use std::ops::RangeBounds;
use std::path::Path;

/// Tier 3: sled-backed persistent storage with B+Tree indexing.
pub struct WosBackend {
    db: sled::Db,
}

impl WosBackend {
    /// Open WOS at the given directory path.
    pub fn open(path: &Path) -> DbxResult<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Open a temporary WOS (for testing). Data is deleted on drop.
    pub fn open_temporary() -> DbxResult<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    /// Get or create a sled Tree for the given table name.
    fn tree(&self, table: &str) -> DbxResult<sled::Tree> {
        Ok(self.db.open_tree(table)?)
    }
}

impl StorageBackend for WosBackend {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        let tree = self.tree(table)?;
        tree.insert(key, value)?;
        Ok(())
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        let tree = self.tree(table)?;
        Ok(tree.get(key)?.map(|ivec| ivec.to_vec()))
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        let tree = self.tree(table)?;
        Ok(tree.remove(key)?.is_some())
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let tree = self.tree(table)?;

        // Convert RangeBounds<Vec<u8>> to sled-compatible range
        let iter = match (range.start_bound(), range.end_bound()) {
            (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded) => tree.iter(),
            (std::ops::Bound::Included(start), std::ops::Bound::Unbounded) => {
                tree.range(start.as_slice()..)
            }
            (std::ops::Bound::Included(start), std::ops::Bound::Excluded(end)) => {
                tree.range(start.as_slice()..end.as_slice())
            }
            (std::ops::Bound::Included(start), std::ops::Bound::Included(end)) => {
                tree.range(start.as_slice()..=end.as_slice())
            }
            (std::ops::Bound::Unbounded, std::ops::Bound::Excluded(end)) => {
                tree.range(..end.as_slice())
            }
            (std::ops::Bound::Unbounded, std::ops::Bound::Included(end)) => {
                tree.range(..=end.as_slice())
            }
            (std::ops::Bound::Excluded(_), _) => {
                // sled doesn't directly support excluded start bounds,
                // use full iteration with manual filter
                tree.iter()
            }
        };

        let mut result = Vec::new();
        for item in iter {
            let (k, v) = item?;
            let key_vec = k.to_vec();
            // For excluded start bound, manually filter
            if let std::ops::Bound::Excluded(start) = range.start_bound()
                && key_vec <= *start
            {
                continue;
            }
            result.push((key_vec, v.to_vec()));
        }
        Ok(result)
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        let tree = self.tree(table)?;

        // Convert RangeBounds<Vec<u8>> to sled-compatible range
        let mut iter = match (range.start_bound(), range.end_bound()) {
            (std::ops::Bound::Unbounded, std::ops::Bound::Unbounded) => tree.iter(),
            (std::ops::Bound::Included(start), std::ops::Bound::Unbounded) => {
                tree.range(start.as_slice()..)
            }
            (std::ops::Bound::Included(start), std::ops::Bound::Excluded(end)) => {
                tree.range(start.as_slice()..end.as_slice())
            }
            (std::ops::Bound::Included(start), std::ops::Bound::Included(end)) => {
                tree.range(start.as_slice()..=end.as_slice())
            }
            (std::ops::Bound::Unbounded, std::ops::Bound::Excluded(end)) => {
                tree.range(..end.as_slice())
            }
            (std::ops::Bound::Unbounded, std::ops::Bound::Included(end)) => {
                tree.range(..=end.as_slice())
            }
            (std::ops::Bound::Excluded(_), _) => tree.iter(),
        };

        if let Some(item) = iter.next() {
            let (k, v) = item?;
            let key_vec = k.to_vec();
            // For excluded start bound, manually filter
            if let std::ops::Bound::Excluded(start) = range.start_bound()
                && key_vec <= *start
            {
                // Fallback to full iteration if the first item doesn't match
                // (This is inefficient but consistent with scan's current logic)
                for next_item in iter {
                    let (nk, nv) = next_item?;
                    let nkey_vec = nk.to_vec();
                    if nkey_vec > *start {
                        return Ok(Some((nkey_vec, nv.to_vec())));
                    }
                }
                return Ok(None);
            }
            return Ok(Some((key_vec, v.to_vec())));
        }
        Ok(None)
    }

    fn flush(&self) -> DbxResult<()> {
        self.db.flush()?;
        Ok(())
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        let tree = self.tree(table)?;
        Ok(tree.len())
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        Ok(self
            .db
            .tree_names()
            .into_iter()
            .filter_map(|name| {
                let s = String::from_utf8(name.to_vec()).ok()?;
                // sled has a default tree named "__sled__default"
                if s == "__sled__default" {
                    None
                } else {
                    Some(s)
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_wos() -> WosBackend {
        WosBackend::open_temporary().unwrap()
    }

    #[test]
    fn insert_and_get() {
        let wos = temp_wos();
        wos.insert("users", b"key1", b"value1").unwrap();
        let result = wos.get("users", b"key1").unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
    }

    #[test]
    fn get_nonexistent() {
        let wos = temp_wos();
        let result = wos.get("users", b"missing").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn delete_existing() {
        let wos = temp_wos();
        wos.insert("users", b"key1", b"value1").unwrap();
        assert!(wos.delete("users", b"key1").unwrap());
        assert_eq!(wos.get("users", b"key1").unwrap(), None);
    }

    #[test]
    fn delete_nonexistent() {
        let wos = temp_wos();
        assert!(!wos.delete("users", b"missing").unwrap());
    }

    #[test]
    fn upsert_overwrites() {
        let wos = temp_wos();
        wos.insert("t", b"k", b"v1").unwrap();
        wos.insert("t", b"k", b"v2").unwrap();
        assert_eq!(wos.get("t", b"k").unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn scan_all() {
        let wos = temp_wos();
        wos.insert("t", b"a", b"1").unwrap();
        wos.insert("t", b"b", b"2").unwrap();
        wos.insert("t", b"c", b"3").unwrap();

        let all: Vec<(Vec<u8>, Vec<u8>)> = wos.scan("t", ..).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].0, b"a");
        assert_eq!(all[2].0, b"c");
    }

    #[test]
    fn scan_range() {
        let wos = temp_wos();
        wos.insert("t", b"a", b"1").unwrap();
        wos.insert("t", b"b", b"2").unwrap();
        wos.insert("t", b"c", b"3").unwrap();
        wos.insert("t", b"d", b"4").unwrap();

        let range_result = wos.scan("t", b"b".to_vec()..b"d".to_vec()).unwrap();
        assert_eq!(range_result.len(), 2);
        assert_eq!(range_result[0].0, b"b");
        assert_eq!(range_result[1].0, b"c");
    }

    #[test]
    fn count() {
        let wos = temp_wos();
        assert_eq!(wos.count("t").unwrap(), 0);
        wos.insert("t", b"a", b"1").unwrap();
        wos.insert("t", b"b", b"2").unwrap();
        assert_eq!(wos.count("t").unwrap(), 2);
    }

    #[test]
    fn table_names() {
        let wos = temp_wos();
        wos.insert("users", b"a", b"1").unwrap();
        wos.insert("orders", b"b", b"2").unwrap();
        let mut names = wos.table_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["orders".to_string(), "users".to_string()]);
    }

    #[test]
    fn flush_persists() {
        let wos = temp_wos();
        wos.insert("t", b"key", b"val").unwrap();
        wos.flush().unwrap();
        // After flush, data should still be readable
        assert_eq!(wos.get("t", b"key").unwrap(), Some(b"val".to_vec()));
    }

    #[test]
    fn multiple_tables_isolation() {
        let wos = temp_wos();
        wos.insert("t1", b"k", b"v1").unwrap();
        wos.insert("t2", b"k", b"v2").unwrap();
        assert_eq!(wos.get("t1", b"k").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(wos.get("t2", b"k").unwrap(), Some(b"v2".to_vec()));
    }
}
