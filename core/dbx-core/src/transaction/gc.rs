//! Garbage Collection for MVCC version cleanup.
//!
//! The GarbageCollector is responsible for cleaning up old versions of data
//! that are no longer visible to any active transaction. This prevents
//! unbounded memory growth in the Delta Store.
//!
//! # Algorithm
//!
//! 1. Track the minimum active read timestamp (`min_active_ts`)
//! 2. Any version with `commit_ts < min_active_ts` is safe to delete
//! 3. Keep at least one version per key (the latest committed version)
//!
//! # Example
//!
//! ```rust
//! use dbx_core::transaction::gc::GarbageCollector;
//! use dbx_core::Database;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! let db = Database::open_in_memory()?;
//! let gc = GarbageCollector::new();
//!
//! // Collect garbage for versions older than timestamp 100
//! let deleted = gc.collect(&db, 100)?;
//! # Ok(())
//! # }
//! ```

use crate::Database;
use crate::error::DbxResult;
use crate::storage::StorageBackend;
use crate::transaction::version::VersionedKey;
use std::collections::HashMap;

/// Garbage collector for cleaning up old MVCC versions.
#[derive(Debug)]
pub struct GarbageCollector {
    /// Minimum number of versions to keep per key (default: 1)
    min_versions_per_key: usize,
}

impl GarbageCollector {
    /// Create a new garbage collector.
    pub fn new() -> Self {
        Self {
            min_versions_per_key: 1,
        }
    }

    /// Create a garbage collector with custom settings.
    pub fn with_min_versions(min_versions: usize) -> Self {
        Self {
            min_versions_per_key: min_versions.max(1), // At least 1
        }
    }

    /// Collect garbage for versions older than `min_active_ts`.
    ///
    /// This scans the Delta Store and removes versions that:
    /// 1. Have `commit_ts < min_active_ts`
    /// 2. Are not the latest version for their key
    ///
    /// # Arguments
    ///
    /// * `db` - The database to clean
    /// * `min_active_ts` - The minimum active read timestamp
    ///
    /// # Returns
    ///
    /// The number of versions deleted
    ///
    /// # Note
    ///
    /// This is a simplified implementation that scans all versioned keys.
    /// For production use, consider implementing a background GC thread
    /// that tracks version metadata separately.
    pub fn collect(&self, db: &Database, min_active_ts: u64) -> DbxResult<usize> {
        let mut deleted_count = 0;

        // Get all table names
        let tables = db.table_names()?;

        for table in tables {
            // Scan all entries in Delta Store for this table
            let all_entries = db.delta.scan(&table, vec![]..)?;

            // Group by user_key
            let mut key_versions: HashMap<Vec<u8>, Vec<(Vec<u8>, u64)>> = HashMap::new();

            for (encoded_key, _value) in all_entries {
                if let Ok(vk) = VersionedKey::decode(&encoded_key) {
                    key_versions
                        .entry(vk.user_key.clone())
                        .or_default()
                        .push((encoded_key, vk.commit_ts));
                }
            }

            // For each user_key, keep only recent versions
            for (_user_key, mut versions) in key_versions {
                // Sort by commit_ts descending (newest first)
                versions.sort_by(|a, b| b.1.cmp(&a.1));

                // Keep the first `min_versions_per_key` versions
                let to_keep = self.min_versions_per_key;

                // Delete old versions that are:
                // 1. Beyond the min_versions_per_key threshold
                // 2. Older than min_active_ts
                // 3. Have a visible newer version (visibility boundary preservation)
                for (i, (encoded_key, commit_ts)) in versions.iter().enumerate() {
                    // Always keep the first N versions
                    if i < to_keep {
                        continue;
                    }

                    // Check if there's ANY newer version that's visible at min_active_ts
                    // This ensures we don't delete a version that might be needed
                    // for transactions reading at min_active_ts
                    let has_visible_newer_version =
                        versions[..i].iter().any(|(_, ts)| *ts <= min_active_ts);

                    // Only delete if:
                    // - Current version is older than min_active_ts
                    // - There's a newer version that's visible at min_active_ts
                    if *commit_ts < min_active_ts && has_visible_newer_version {
                        // Delete from Delta Store directly
                        // Note: This accesses internal API - in production, consider
                        // adding a public delete_versioned() method to Database
                        db.delta.delete(&table, encoded_key)?;
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// Estimate the number of versions that would be deleted.
    ///
    /// This is useful for monitoring and deciding when to run GC.
    pub fn estimate_garbage(&self, db: &Database, min_active_ts: u64) -> DbxResult<usize> {
        let mut garbage_count = 0;

        let tables = db.table_names()?;

        for table in tables {
            let all_entries = db.delta.scan(&table, vec![]..)?;

            let mut key_versions: HashMap<Vec<u8>, Vec<u64>> = HashMap::new();

            for (encoded_key, _value) in all_entries {
                if let Ok(vk) = VersionedKey::decode(&encoded_key) {
                    key_versions
                        .entry(vk.user_key.clone())
                        .or_default()
                        .push(vk.commit_ts);
                }
            }

            for (_user_key, mut versions) in key_versions {
                versions.sort_by(|a, b| b.cmp(a));

                for (i, commit_ts) in versions.iter().enumerate() {
                    // Skip the first N versions
                    if i < self.min_versions_per_key {
                        continue;
                    }

                    // Check if there's ANY visible newer version
                    let has_visible_newer_version =
                        versions[..i].iter().any(|ts| *ts <= min_active_ts);

                    // Count as garbage if old and has visible newer version
                    if *commit_ts < min_active_ts && has_visible_newer_version {
                        garbage_count += 1;
                    }
                }
            }
        }

        Ok(garbage_count)
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_removes_old_versions() -> DbxResult<()> {
        let db = Database::open_in_memory()?;
        let gc = GarbageCollector::new();

        // Create multiple versions of the same key
        db.insert_versioned("test", b"key1", Some(b"v1"), 10)?;
        db.insert_versioned("test", b"key1", Some(b"v2"), 20)?;
        db.insert_versioned("test", b"key1", Some(b"v3"), 30)?;

        // Before GC: 3 versions
        let estimate_before = gc.estimate_garbage(&db, 25)?;
        assert_eq!(estimate_before, 1); // Version at ts=10 is garbage

        // Run GC with min_active_ts = 25
        let deleted = gc.collect(&db, 25)?;
        assert_eq!(deleted, 1);

        // After GC: should have 2 versions (ts=20 and ts=30)
        let estimate_after = gc.estimate_garbage(&db, 25)?;
        assert_eq!(estimate_after, 0);

        Ok(())
    }

    #[test]
    fn test_gc_keeps_minimum_versions() -> DbxResult<()> {
        let db = Database::open_in_memory()?;
        let gc = GarbageCollector::with_min_versions(2);

        // Create 4 versions
        db.insert_versioned("test", b"key1", Some(b"v1"), 10)?;
        db.insert_versioned("test", b"key1", Some(b"v2"), 20)?;
        db.insert_versioned("test", b"key1", Some(b"v3"), 30)?;
        db.insert_versioned("test", b"key1", Some(b"v4"), 40)?;

        // Run GC with min_active_ts = 100 (all versions are old)
        let deleted = gc.collect(&db, 100)?;

        // Should keep 2 versions (ts=40 and ts=30), delete 2 (ts=20 and ts=10)
        assert_eq!(deleted, 2);

        Ok(())
    }

    #[test]
    fn test_gc_multiple_keys() -> DbxResult<()> {
        let db = Database::open_in_memory()?;
        let gc = GarbageCollector::new();

        // Key1: 3 versions
        db.insert_versioned("test", b"key1", Some(b"v1"), 10)?;
        db.insert_versioned("test", b"key1", Some(b"v2"), 20)?;
        db.insert_versioned("test", b"key1", Some(b"v3"), 30)?;

        // Key2: 3 versions (added ts=20 to make it visible at min_active_ts=22)
        db.insert_versioned("test", b"key2", Some(b"v1"), 15)?;
        db.insert_versioned("test", b"key2", Some(b"v2"), 20)?;
        db.insert_versioned("test", b"key2", Some(b"v3"), 25)?;

        // Run GC with min_active_ts = 22
        let deleted = gc.collect(&db, 22)?;

        // Should delete: key1@10 (has visible ts=20), key2@15 (has visible ts=20) = 2 versions
        assert_eq!(deleted, 2);

        Ok(())
    }

    #[test]
    fn test_gc_estimate_accuracy() -> DbxResult<()> {
        let db = Database::open_in_memory()?;
        let gc = GarbageCollector::new();

        db.insert_versioned("test", b"key1", Some(b"v1"), 10)?;
        db.insert_versioned("test", b"key1", Some(b"v2"), 20)?;
        db.insert_versioned("test", b"key1", Some(b"v3"), 30)?;

        let estimate = gc.estimate_garbage(&db, 25)?;
        let actual = gc.collect(&db, 25)?;

        assert_eq!(estimate, actual);

        Ok(())
    }

    #[test]
    fn test_gc_empty_database() -> DbxResult<()> {
        let db = Database::open_in_memory()?;
        let gc = GarbageCollector::new();

        let deleted = gc.collect(&db, 100)?;
        assert_eq!(deleted, 0);

        Ok(())
    }
}
