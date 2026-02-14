//! In-memory WOS backend using BTreeMap
//!
//! Provides fast O(log n) operations with proven stability

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use std::collections::{BTreeMap, HashMap};
use std::ops::RangeBounds;
use std::sync::RwLock;

/// In-memory WOS backend using BTreeMap
pub struct InMemoryWosBackend {
    tables: RwLock<HashMap<String, BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl InMemoryWosBackend {
    /// Create a new in-memory WOS backend
    pub fn new() -> Self {
        Self {
            tables: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryWosBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for InMemoryWosBackend {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        let mut tables = self.tables.write().unwrap();
        tables
            .entry(table.to_string())
            .or_insert_with(BTreeMap::new)
            .insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        let tables = self.tables.read().unwrap();
        Ok(tables.get(table).and_then(|map| map.get(key).cloned()))
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        let mut tables = self.tables.write().unwrap();
        if let Some(map) = tables.get_mut(table) {
            Ok(map.remove(key).is_some())
        } else {
            Ok(false)
        }
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let tables = self.tables.read().unwrap();
        if let Some(map) = tables.get(table) {
            Ok(map
                .range(range)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        let tables = self.tables.read().unwrap();
        if let Some(map) = tables.get(table) {
            Ok(map
                .range(range)
                .map(|(k, v)| (k.clone(), v.clone()))
                .next())
        } else {
            Ok(None)
        }
    }

    fn flush(&self) -> DbxResult<()> {
        // No-op for in-memory backend
        Ok(())
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        let tables = self.tables.read().unwrap();
        Ok(tables.get(table).map(|m| m.len()).unwrap_or(0))
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        let tables = self.tables.read().unwrap();
        Ok(tables.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let backend = InMemoryWosBackend::new();
        backend
            .insert("test", b"key1", b"value1")
            .unwrap();

        let result = backend.get("test", b"key1").unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_delete() {
        let backend = InMemoryWosBackend::new();
        backend
            .insert("test", b"key1", b"value1")
            .unwrap();

        assert!(backend.delete("test", b"key1").unwrap());
        assert_eq!(backend.get("test", b"key1").unwrap(), None);
    }

    #[test]
    fn test_scan() {
        let backend = InMemoryWosBackend::new();
        backend
            .insert("test", b"key1", b"value1")
            .unwrap();
        backend
            .insert("test", b"key2", b"value2")
            .unwrap();
        backend
            .insert("test", b"key3", b"value3")
            .unwrap();

        let results = backend
            .scan("test", b"key1".to_vec()..b"key3".to_vec())
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_count() {
        let backend = InMemoryWosBackend::new();
        backend
            .insert("test", b"key1", b"value1")
            .unwrap();
        backend
            .insert("test", b"key2", b"value2")
            .unwrap();

        assert_eq!(backend.count("test").unwrap(), 2);
    }
}
