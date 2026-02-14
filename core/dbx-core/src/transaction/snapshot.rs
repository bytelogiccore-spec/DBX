//! Snapshot — MVCC Snapshot Isolation implementation.
//!
//! A Snapshot represents a consistent view of the database at a specific point in time.
//! All reads within a snapshot see the same data, regardless of concurrent writes.

use crate::Database;
use crate::error::DbxResult;
use dashmap::DashMap;
use std::sync::Arc;

/// A snapshot of the database at a specific read timestamp.
///
/// Provides Snapshot Isolation: all reads see a consistent view of the database
/// as it existed at `read_ts`, unaffected by concurrent writes.
#[derive(Clone)]
pub struct Snapshot {
    /// The timestamp at which this snapshot was taken
    pub read_ts: u64,

    /// Reference to the database for accessing versioned data
    db: Arc<Database>,

    /// Cache of visible versions for this snapshot
    /// (table, key) -> value
    /// This cache is populated lazily as keys are accessed
    #[allow(clippy::type_complexity)]
    visible_cache: Arc<DashMap<(String, Vec<u8>), Option<Vec<u8>>>>,
}

impl Snapshot {
    /// Create a new snapshot at the given read timestamp.
    pub fn new(db: Arc<Database>, read_ts: u64) -> Self {
        Self {
            read_ts,
            db,
            visible_cache: Arc::new(DashMap::new()),
        }
    }

    /// Get a value from the snapshot.
    ///
    /// Returns the latest version of the key that is visible to this snapshot
    /// (i.e., committed before or at `read_ts`).
    pub fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        let cache_key = (table.to_string(), key.to_vec());

        // Check cache first
        if let Some(entry) = self.visible_cache.get(&cache_key) {
            return Ok(entry.value().clone());
        }

        // Not in cache - query versioned storage
        let result = match self.db.get_snapshot(table, key, self.read_ts)? {
            Some(Some(value)) => Some(value), // Found value
            Some(None) => None,               // Found tombstone (deleted)
            None => None,                     // Not found in versioned storage
        };

        // Cache the result
        self.visible_cache.insert(cache_key, result.clone());

        Ok(result)
    }

    /// Scan all keys in a table that are visible to this snapshot.
    ///
    /// Returns all key-value pairs where the latest version visible to this snapshot
    /// is not a tombstone.
    pub fn scan(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Get all versioned keys from Delta Store and WOS
        let delta_entries = self.db.scan_delta_versioned(table)?;
        let wos_entries = self.db.scan_wos_versioned(table)?;

        // Merge and filter by visibility
        // Store (value, commit_ts) to track the latest version for each key
        let visible_keys: DashMap<Vec<u8>, (Vec<u8>, u64)> = DashMap::new();

        // Process WOS entries first (lower priority)
        for (encoded_key, encoded_value) in wos_entries {
            if let Ok(vk) = crate::transaction::version::VersionedKey::decode(&encoded_key)
                && vk.commit_ts <= self.read_ts {
                    // Decode value
                    let value = if encoded_value.is_empty() {
                        Vec::new()
                    } else if encoded_value[0] == b'v' {
                        encoded_value[1..].to_vec()
                    } else if encoded_value[0] == b'd' {
                        Vec::new()
                    } else {
                        encoded_value.clone() // Legacy
                    };

                    // Insert or update if this version is newer
                    visible_keys
                        .entry(vk.user_key.clone())
                        .and_modify(|(existing_val, existing_ts)| {
                            if vk.commit_ts > *existing_ts {
                                *existing_val = value.clone();
                                *existing_ts = vk.commit_ts;
                            }
                        })
                        .or_insert((value, vk.commit_ts));
                }
        }

        // Process Delta entries (higher priority - overrides WOS)
        for (encoded_key, encoded_value) in delta_entries {
            if let Ok(vk) = crate::transaction::version::VersionedKey::decode(&encoded_key)
                && vk.commit_ts <= self.read_ts {
                    // Decode value - handle legacy (no prefix) and versioned (v/d prefix)
                    let value = if encoded_value.is_empty() {
                        Vec::new() // Should not happen but handle gracefully
                    } else if encoded_value[0] == b'v' {
                        encoded_value[1..].to_vec()
                    } else if encoded_value[0] == b'd' {
                        Vec::new() // Tombstone
                    } else {
                        // Legacy value (no prefix)
                        encoded_value.clone()
                    };

                    // Insert or update if this version is newer
                    visible_keys
                        .entry(vk.user_key.clone())
                        .and_modify(|(existing_val, existing_ts)| {
                            if vk.commit_ts > *existing_ts {
                                *existing_val = value.clone();
                                *existing_ts = vk.commit_ts;
                            }
                        })
                        .or_insert((value, vk.commit_ts));
                }
        }

        // Filter out tombstones and convert to Vec
        let result: Vec<(Vec<u8>, Vec<u8>)> = visible_keys
            .into_iter()
            .filter(|(_, (v, _))| !v.is_empty())
            .map(|(k, (v, _))| (k, v))
            .collect();

        Ok(result)
    }

    /// Get the read timestamp of this snapshot.
    pub fn read_ts(&self) -> u64 {
        self.read_ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;

    #[test]
    fn test_snapshot_isolation() -> DbxResult<()> {
        let db = Arc::new(Database::open_in_memory()?);

        // Insert initial data
        db.insert_versioned("test", b"key1", Some(b"v1"), 10)?;
        db.insert_versioned("test", b"key2", Some(b"v2"), 20)?;

        // Create snapshot at ts=15
        let snapshot = Snapshot::new(Arc::clone(&db), 15);

        // Snapshot should see key1 (ts=10) but not key2 (ts=20)
        assert_eq!(snapshot.get("test", b"key1")?, Some(b"v1".to_vec()));
        assert_eq!(snapshot.get("test", b"key2")?, None);

        // Insert new version after snapshot
        db.insert_versioned("test", b"key1", Some(b"v1_new"), 30)?;

        // Snapshot should still see old version
        assert_eq!(snapshot.get("test", b"key1")?, Some(b"v1".to_vec()));

        Ok(())
    }

    #[test]
    fn test_snapshot_tombstone() -> DbxResult<()> {
        let db = Arc::new(Database::open_in_memory()?);

        // Insert and delete
        db.insert_versioned("test", b"key1", Some(b"value"), 10)?;
        db.insert_versioned("test", b"key1", None, 20)?; // Delete

        // Snapshot before delete
        let snapshot1 = Snapshot::new(Arc::clone(&db), 15);
        assert_eq!(snapshot1.get("test", b"key1")?, Some(b"value".to_vec()));

        // Snapshot after delete
        let snapshot2 = Snapshot::new(Arc::clone(&db), 25);
        assert_eq!(snapshot2.get("test", b"key1")?, None);

        Ok(())
    }

    #[test]
    fn test_snapshot_cache() -> DbxResult<()> {
        let db = Arc::new(Database::open_in_memory()?);

        db.insert_versioned("test", b"key1", Some(b"value"), 10)?;

        let snapshot = Snapshot::new(Arc::clone(&db), 15);

        // First access - should query and cache
        let val1 = snapshot.get("test", b"key1")?;
        assert_eq!(val1, Some(b"value".to_vec()));

        // Second access - should hit cache
        let val2 = snapshot.get("test", b"key1")?;
        assert_eq!(val2, Some(b"value".to_vec()));

        // Cache should have 1 entry
        assert_eq!(snapshot.visible_cache.len(), 1);

        Ok(())
    }
}
