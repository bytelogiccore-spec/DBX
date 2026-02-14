//! Storage module — 5-Tier Hybrid Storage architecture.
//!
//! All storage engines implement the [`StorageBackend`] trait.
//! The SQL layer depends only on this trait (Dependency Inversion Principle).

pub mod cache;
pub mod columnar;
pub mod columnar_cache;
pub mod columnar_delta;
pub mod compression;
pub mod delta_store;
pub mod encrypted_parquet;
pub mod encrypted_wos;
pub mod encryption;
pub mod gpu;

pub mod kv_adapter;
pub mod memory_wos;
pub mod opfs;
pub mod parquet_io;
pub mod versioned_batch;
pub mod wos;
pub mod arrow_ipc;

use crate::error::DbxResult;
use std::ops::RangeBounds;

/// Core storage interface — all tiers implement this trait.
///
/// # Design Principles
///
/// - **DIP**: SQL layer depends on this trait, never on concrete types.
/// - **Strategy**: New storage tiers are added by implementing this trait.
/// - **Thread Safety**: `Send + Sync` required for concurrent access.
///
/// # Contract
///
/// - `insert`: Upsert semantics — overwrites existing key.
/// - `get`: Returns `None` for non-existent keys, never errors.
/// - `delete`: Returns `true` if key existed, `false` otherwise.
/// - `scan`: Returns key-value pairs in key order within range.
/// - `flush`: Persists buffered data to durable storage.
/// - `count`: Returns the number of keys in a table.
/// - `table_names`: Returns all table names.
pub trait StorageBackend: Send + Sync {
    /// Insert a key-value pair.
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()>;

    /// Insert multiple key-value pairs in a batch (optimized).
    ///
    /// Default implementation calls insert() sequentially.
    /// Implementations should override this for better performance.
    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        for (key, value) in rows {
            self.insert(table, &key, &value)?;
        }
        Ok(())
    }

    /// Get a value by key.
    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>>;

    /// Delete a key-value pair.
    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool>;

    /// Scan a range of keys.
    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Scan a single key-value pair in a range (optimized).
    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>>;

    /// Flush any buffered data to durable storage.
    fn flush(&self) -> DbxResult<()>;

    /// Return the number of keys in the given table.
    fn count(&self, table: &str) -> DbxResult<usize>;

    /// Return all table names managed by this backend.
    fn table_names(&self) -> DbxResult<Vec<String>>;
}
