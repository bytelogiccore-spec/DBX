//! Delta Store — Tier 1: In-memory write buffer backed by SkipList and DashMap.
//!
//! Provides concurrent insert/get/delete with O(log N) latency and
//! O(log N + K) range scans without sorting overhead.
//! Now MVCC-aware using VersionedKey and DashMap for table management.

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use crate::transaction::version::VersionedKey;
use crossbeam_skiplist::SkipMap;
use dashmap::DashMap;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;

/// Default flush threshold: flush to WOS when entry count exceeds this.
const DEFAULT_FLUSH_THRESHOLD: usize = 10_000;

/// Tier 1: Concurrent in-memory store with ordered versioned keys.
///
/// Uses `DashMap` for O(1) table lookups and `SkipMap` for O(log N) ordered storage.
/// Each table is a separate `SkipMap` instance.
pub struct DeltaStore {
    /// Table name → SkipMap mapping
    /// Using DashMap for O(1) table access
    #[allow(clippy::type_complexity)]
    tables: DashMap<String, Arc<SkipMap<VersionedKey, Arc<Vec<u8>>>>>,
    /// Threshold to trigger flush
    flush_threshold: usize,
}

impl DeltaStore {
    /// Create a new Delta Store with the default flush threshold (10,000).
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_FLUSH_THRESHOLD)
    }

    /// Create a new Delta Store with a custom flush threshold.
    pub fn with_threshold(threshold: usize) -> Self {
        Self {
            tables: DashMap::new(),
            flush_threshold: threshold,
        }
    }

    /// Check if the store should be flushed to a lower tier.
    pub fn should_flush(&self) -> bool {
        self.entry_count() >= self.flush_threshold
    }

    /// Get the current entry count across all tables.
    pub fn entry_count(&self) -> usize {
        self.tables.iter().map(|e| e.value().len()).sum()
    }

    /// Drain all data from the store, returning table→entries mapping.
    /// Used during flush to move data to WOS.
    ///
    /// Note: Returns encoded keys for backward compatibility with WOS.
    #[allow(clippy::type_complexity)]
    pub fn drain_all(&self) -> Vec<(String, Vec<(Vec<u8>, Vec<u8>)>)> {
        let mut result = Vec::new();

        // Collect all table names
        let table_names: Vec<String> = self.tables.iter().map(|e| e.key().clone()).collect();

        for table_name in table_names {
            if let Some((_, table_map)) = self.tables.remove(&table_name) {
                let entries: Vec<(Vec<u8>, Vec<u8>)> = table_map
                    .iter()
                    .map(|e| (e.key().encode(), (**e.value()).clone()))
                    .collect();

                result.push((table_name, entries));
            }
        }

        result
    }

    /// Get or create the SkipMap for a table.
    fn get_or_create_table(&self, table: &str) -> Arc<SkipMap<VersionedKey, Arc<Vec<u8>>>> {
        self.tables
            .entry(table.to_string())
            .or_insert_with(|| Arc::new(SkipMap::new()))
            .value()
            .clone()
    }

    /// Helper to convert raw bytes to VersionedKey.
    fn to_versioned_key(key: &[u8], default_ts: u64) -> VersionedKey {
        // If it looks like a versioned key (length > 8), try to decode it.
        // Versioned keys are [user_key] + [8 bytes timestamp].
        if key.len() > 8
            && let Ok(vk) = VersionedKey::decode(key)
        {
            return vk;
        }
        VersionedKey::new(key.to_vec(), default_ts)
    }

    /// Helper to convert Bound<&Vec<u8>> to Bound<VersionedKey>.
    fn convert_start_bound(bound: Bound<&Vec<u8>>) -> Bound<VersionedKey> {
        match bound {
            Bound::Included(v) => {
                if v.is_empty() {
                    Bound::Included(VersionedKey::new(vec![], u64::MAX))
                } else {
                    Bound::Included(Self::to_versioned_key(v, u64::MAX))
                }
            }
            Bound::Excluded(v) => {
                if v.is_empty() {
                    Bound::Excluded(VersionedKey::new(vec![], u64::MAX))
                } else {
                    Bound::Excluded(Self::to_versioned_key(v, u64::MAX))
                }
            }
            Bound::Unbounded => Bound::Unbounded,
        }
    }

    fn convert_end_bound(bound: Bound<&Vec<u8>>) -> Bound<VersionedKey> {
        match bound {
            Bound::Included(v) => {
                if v.is_empty() {
                    Bound::Included(VersionedKey::new(vec![], 0))
                } else {
                    Bound::Included(Self::to_versioned_key(v, 0))
                }
            }
            Bound::Excluded(v) => {
                if v.is_empty() {
                    Bound::Excluded(VersionedKey::new(vec![], 0))
                } else {
                    Bound::Excluded(Self::to_versioned_key(v, 0))
                }
            }
            Bound::Unbounded => Bound::Unbounded,
        }
    }
}

impl Default for DeltaStore {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for DeltaStore {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        let table_map = self.get_or_create_table(table);
        // For inserts, if it's a raw key, we use ts=0 (legacy).
        // If it's encoded, decode() will find the correct TS.
        let vk = Self::to_versioned_key(key, 0);
        table_map.insert(vk, Arc::new(value.to_vec()));

        Ok(())
    }

    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        let table_map = self.get_or_create_table(table);

        for (key, value) in rows {
            let vk = Self::to_versioned_key(&key, 0);
            table_map.insert(vk, Arc::new(value));
        }

        Ok(())
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        let Some(table_map) = self.tables.get(table) else {
            return Ok(None);
        };

        let vk = Self::to_versioned_key(key, 0);
        Ok(table_map.get(&vk).map(|e| (**e.value()).clone()))
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        let Some(table_map) = self.tables.get(table) else {
            return Ok(false);
        };

        let vk = Self::to_versioned_key(key, 0);
        Ok(table_map.remove(&vk).is_some())
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let Some(table_map) = self.tables.get(table) else {
            return Ok(Vec::new());
        };

        let start = Self::convert_start_bound(range.start_bound());
        let end = Self::convert_end_bound(range.end_bound());

        let entries: Vec<(Vec<u8>, Vec<u8>)> = table_map
            .range((start, end))
            .map(|e| (e.key().encode(), (**e.value()).clone()))
            .collect();

        Ok(entries)
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        let Some(table_map) = self.tables.get(table) else {
            return Ok(None);
        };

        let start = Self::convert_start_bound(range.start_bound());
        let end = Self::convert_end_bound(range.end_bound());

        Ok(table_map
            .range((start, end))
            .next()
            .map(|e| (e.key().encode(), (**e.value()).clone())))
    }

    fn flush(&self) -> DbxResult<()> {
        Ok(())
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        let Some(table_map) = self.tables.get(table) else {
            return Ok(0);
        };

        Ok(table_map.len())
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        Ok(self.tables.iter().map(|e| e.key().clone()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let store = DeltaStore::new();
        store.insert("users", b"key1", b"value1").unwrap();
        let result = store.get("users", b"key1").unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_versioned_storage() {
        let store = DeltaStore::new();
        let vk1 = VersionedKey::new(b"key1".to_vec(), 100);
        let vk2 = VersionedKey::new(b"key1".to_vec(), 200);

        store.insert("users", &vk1.encode(), b"v1").unwrap();
        store.insert("users", &vk2.encode(), b"v2").unwrap();

        // Should be able to get both versions if we use the exact versioned key
        assert_eq!(
            store.get("users", &vk1.encode()).unwrap(),
            Some(b"v1".to_vec())
        );
        assert_eq!(
            store.get("users", &vk2.encode()).unwrap(),
            Some(b"v2".to_vec())
        );

        // Scan should return them in correct order (latest first for same key)
        let results = store.scan("users", Vec::<u8>::new()..).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(VersionedKey::decode(&results[0].0).unwrap().commit_ts, 200);
        assert_eq!(VersionedKey::decode(&results[1].0).unwrap().commit_ts, 100);
    }

    #[test]
    fn delete_existing_key() {
        let store = DeltaStore::new();
        store.insert("users", b"key1", b"value1").unwrap();
        assert!(store.delete("users", b"key1").unwrap());
        assert_eq!(store.get("users", b"key1").unwrap(), None);
    }

    #[test]
    fn entry_count_tracking() {
        let store = DeltaStore::new();
        assert_eq!(store.entry_count(), 0);
        store.insert("t1", b"a", b"1").unwrap();
        store.insert("t1", b"b", b"2").unwrap();
        store.insert("t2", b"c", b"3").unwrap();
        assert_eq!(store.entry_count(), 3);
    }
}
