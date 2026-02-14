//! Columnar Delta Store — MVCC-aware in-memory columnar buffer.
//!
//! Replaces the row-based Delta Store with a columnar implementation
//! using Arrow RecordBatch and VersionedBatch for MVCC Snapshot Isolation.

use crate::error::{DbxError, DbxResult};
use crate::storage::StorageBackend;
use crate::storage::kv_adapter::{batch_to_kv, kv_to_batch, merge_batches};
use crate::storage::versioned_batch::VersionedBatch;
use arrow::array::{Array, BinaryArray, BooleanArray};
use arrow::compute::{filter, sort_to_indices, take};
use arrow::record_batch::RecordBatch;
use dashmap::DashMap;
use std::ops::RangeBounds;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Columnar Delta Store with MVCC support.
///
/// Stores versioned RecordBatches for each table, enabling:
/// - Snapshot Isolation via VersionedBatch
/// - Efficient columnar scans via Arrow
/// - Memory sharing via Arc<RecordBatch>
pub struct ColumnarDelta {
    /// Table name → list of versioned batches
    tables: DashMap<String, Vec<VersionedBatch>>,

    /// Global sequence counter for ordering batches
    sequence: AtomicU64,

    /// Flush threshold (number of rows across all tables)
    flush_threshold: usize,

    /// Current total row count across all tables
    row_count: AtomicU64,
}

impl ColumnarDelta {
    /// Create a new ColumnarDelta with the given flush threshold.
    pub fn new(flush_threshold: usize) -> Self {
        Self {
            tables: DashMap::new(),
            sequence: AtomicU64::new(0),
            flush_threshold,
            row_count: AtomicU64::new(0),
        }
    }

    /// Insert a versioned batch for a table.
    ///
    /// The batch will be assigned a sequence number and begin_ts.
    pub fn insert_versioned_batch(
        &self,
        table: &str,
        batch: RecordBatch,
        begin_ts: u64,
    ) -> DbxResult<()> {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        let versioned = VersionedBatch::new(Arc::new(batch.clone()), begin_ts, sequence);

        let row_count = batch.num_rows();
        self.row_count.fetch_add(row_count as u64, Ordering::SeqCst);

        self.tables
            .entry(table.to_string())
            .or_default()
            .push(versioned);

        Ok(())
    }

    /// Get all batches visible to a snapshot at the given read_ts.
    pub fn get_visible_batches(&self, table: &str, read_ts: u64) -> Vec<Arc<RecordBatch>> {
        if let Some(batches) = self.tables.get(table) {
            batches
                .iter()
                .filter(|b| b.is_visible(read_ts))
                .map(|b| Arc::clone(&b.data))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if flush is needed based on row count threshold.
    pub fn should_flush(&self) -> bool {
        self.row_count.load(Ordering::SeqCst) as usize >= self.flush_threshold
    }

    /// Get the current row count across all tables.
    pub fn row_count(&self) -> usize {
        self.row_count.load(Ordering::SeqCst) as usize
    }

    /// Drain all batches from a table (for flushing to WOS/Parquet).
    ///
    /// Returns all batches and clears the table.
    pub fn drain_table(&self, table: &str) -> Vec<VersionedBatch> {
        if let Some((_, batches)) = self.tables.remove(table) {
            let row_count: usize = batches.iter().map(|b| b.num_rows()).sum();
            self.row_count.fetch_sub(row_count as u64, Ordering::SeqCst);
            batches
        } else {
            Vec::new()
        }
    }

    /// Get all table names.
    pub fn table_names(&self) -> Vec<String> {
        self.tables
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Clear all data (for testing).
    #[cfg(test)]
    pub fn clear(&self) {
        self.tables.clear();
        self.row_count.store(0, Ordering::SeqCst);
    }
}

// ============================================================================
// Arrow Compute Kernel Helpers
// ============================================================================

/// Find a key in a RecordBatch using direct Binary array access.
///
/// This is much faster than converting to key-value pairs and searching.
/// Uses Arrow's zero-copy Binary array access for optimal performance.
fn find_key_in_batch(batch: &RecordBatch, target_key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
    if batch.num_rows() == 0 {
        return Ok(None);
    }

    // Extract key column (column 0)
    let key_array = batch
        .column(0)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| DbxError::Storage("Key column is not BinaryArray".into()))?;

    // Search for matching key
    for i in 0..key_array.len() {
        if !key_array.is_null(i) && key_array.value(i) == target_key {
            // Found! Extract value from column 1
            let value_array = batch
                .column(1)
                .as_any()
                .downcast_ref::<BinaryArray>()
                .ok_or_else(|| DbxError::Storage("Value column is not BinaryArray".into()))?;

            if !value_array.is_null(i) {
                return Ok(Some(value_array.value(i).to_vec()));
            }
        }
    }

    Ok(None)
}

/// Apply range filter to RecordBatch using Arrow compute.
fn apply_range_filter<R: RangeBounds<Vec<u8>>>(
    batch: &RecordBatch,
    range: R,
) -> DbxResult<RecordBatch> {
    if batch.num_rows() == 0 {
        return Ok(batch.clone());
    }

    let key_array = batch
        .column(0)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| DbxError::Storage("Key column is not BinaryArray".into()))?;

    // Build filter mask
    let mut mask = vec![true; batch.num_rows()];

    for (i, mask_val) in mask.iter_mut().enumerate().take(key_array.len()) {
        if !key_array.is_null(i) {
            let key = key_array.value(i).to_vec();
            *mask_val = range.contains(&key);
        } else {
            *mask_val = false;
        }
    }

    // Apply filter using Arrow compute
    let mask_array = BooleanArray::from(mask);

    // Filter each column
    let filtered_columns: Vec<Arc<dyn Array>> = batch
        .columns()
        .iter()
        .map(|col| filter(col.as_ref(), &mask_array))
        .collect::<Result<Vec<_>, _>>()?;

    // Create new batch with filtered columns
    let filtered = RecordBatch::try_new(batch.schema(), filtered_columns)?;

    Ok(filtered)
}

/// Sort RecordBatch by key column using Arrow compute.
fn sort_batch_by_key(batch: &RecordBatch) -> DbxResult<RecordBatch> {
    if batch.num_rows() == 0 {
        return Ok(batch.clone());
    }

    // Get sort indices for key column (column 0)
    let indices = sort_to_indices(batch.column(0), None, None)?;

    // Apply indices to all columns
    let sorted_columns: Vec<Arc<dyn Array>> = batch
        .columns()
        .iter()
        .map(|col| take(col.as_ref(), &indices, None))
        .collect::<Result<Vec<_>, _>>()?;

    // Create new batch with sorted columns
    let sorted_batch = RecordBatch::try_new(batch.schema(), sorted_columns)?;

    Ok(sorted_batch)
}

impl StorageBackend for ColumnarDelta {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        // Convert single key-value pair to RecordBatch
        let batch = kv_to_batch(vec![(key.to_vec(), value.to_vec())])?;

        // Use current timestamp (0 for now, will be set by Database)
        self.insert_versioned_batch(table, batch, 0)?;
        Ok(())
    }

    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        if rows.is_empty() {
            return Ok(());
        }

        // Convert key-value pairs to RecordBatch
        let batch = kv_to_batch(rows)?;

        // Insert with timestamp 0 (will be overridden by Database)
        self.insert_versioned_batch(table, batch, 0)?;
        Ok(())
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // Get all visible batches (using max timestamp for now)
        let batches = self.get_visible_batches(table, u64::MAX);

        // Search through batches using Arrow operations (no conversion needed)
        for batch in batches {
            if let Some(value) = find_key_in_batch(&batch, key)? {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    fn delete(&self, _table: &str, _key: &[u8]) -> DbxResult<bool> {
        // TODO: Implement tombstone support
        // For now, deletion is not supported in ColumnarDelta
        Ok(false)
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Get all visible batches
        let batches = self.get_visible_batches(table, u64::MAX);

        if batches.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Merge all batches
        let merged = merge_batches(batches)?;

        // 2. Apply range filter using Arrow compute
        let filtered = apply_range_filter(&merged, range)?;

        // 3. Sort by key using Arrow compute
        let sorted = sort_batch_by_key(&filtered)?;

        // 4. Convert only the filtered/sorted results
        batch_to_kv(&sorted)
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        let results = self.scan(table, range)?;
        Ok(results.into_iter().next())
    }

    fn flush(&self) -> DbxResult<()> {
        // Flushing is handled by Database, not by ColumnarDelta itself
        Ok(())
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        let batches = self.get_visible_batches(table, u64::MAX);
        let total: usize = batches.iter().map(|b| b.num_rows()).sum();
        Ok(total)
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        Ok(ColumnarDelta::table_names(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    fn create_test_batch(ids: Vec<i32>, names: Vec<&str>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let id_array = Int32Array::from(ids);
        let name_array = StringArray::from(names);

        RecordBatch::try_new(schema, vec![Arc::new(id_array), Arc::new(name_array)]).unwrap()
    }

    #[test]
    fn test_insert_and_retrieve() {
        let delta = ColumnarDelta::new(1000);

        let batch1 = create_test_batch(vec![1, 2], vec!["Alice", "Bob"]);
        delta.insert_versioned_batch("users", batch1, 10).unwrap();

        let visible = delta.get_visible_batches("users", 15);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].num_rows(), 2);
    }

    #[test]
    fn test_snapshot_isolation() {
        let delta = ColumnarDelta::new(1000);

        // Insert batch at ts=10
        let batch1 = create_test_batch(vec![1], vec!["Alice"]);
        delta.insert_versioned_batch("users", batch1, 10).unwrap();

        // Insert batch at ts=20
        let batch2 = create_test_batch(vec![2], vec!["Bob"]);
        delta.insert_versioned_batch("users", batch2, 20).unwrap();

        // Snapshot at ts=15 should only see batch1
        let visible = delta.get_visible_batches("users", 15);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].num_rows(), 1);

        // Snapshot at ts=25 should see both batches
        let visible = delta.get_visible_batches("users", 25);
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_flush_threshold() {
        let delta = ColumnarDelta::new(5);

        let batch1 = create_test_batch(vec![1, 2, 3], vec!["A", "B", "C"]);
        delta.insert_versioned_batch("users", batch1, 10).unwrap();

        assert!(!delta.should_flush()); // 3 rows < 5

        let batch2 = create_test_batch(vec![4, 5], vec!["D", "E"]);
        delta.insert_versioned_batch("users", batch2, 20).unwrap();

        assert!(delta.should_flush()); // 5 rows >= 5
    }

    #[test]
    fn test_drain_table() {
        let delta = ColumnarDelta::new(1000);

        let batch1 = create_test_batch(vec![1, 2], vec!["Alice", "Bob"]);
        delta.insert_versioned_batch("users", batch1, 10).unwrap();

        assert_eq!(delta.row_count(), 2);

        let drained = delta.drain_table("users");
        assert_eq!(drained.len(), 1);
        assert_eq!(delta.row_count(), 0);

        // Table should be empty now
        let visible = delta.get_visible_batches("users", 15);
        assert_eq!(visible.len(), 0);
    }

    #[test]
    fn test_multiple_tables() {
        let delta = ColumnarDelta::new(1000);

        let batch1 = create_test_batch(vec![1], vec!["Alice"]);
        delta.insert_versioned_batch("users", batch1, 10).unwrap();

        let batch2 = create_test_batch(vec![100], vec!["Order1"]);
        delta.insert_versioned_batch("orders", batch2, 10).unwrap();

        let tables = delta.table_names();
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }

    #[test]
    fn test_arc_sharing() {
        let delta = ColumnarDelta::new(1000);

        let batch = create_test_batch(vec![1, 2], vec!["Alice", "Bob"]);
        delta.insert_versioned_batch("users", batch, 10).unwrap();

        // Get visible batches multiple times
        let visible1 = delta.get_visible_batches("users", 15);
        let visible2 = delta.get_visible_batches("users", 15);

        // Both should share the same Arc
        assert!(Arc::ptr_eq(&visible1[0], &visible2[0]));
    }

    // StorageBackend trait tests

    #[test]
    fn test_storage_backend_insert_get() {
        use crate::storage::StorageBackend;

        let delta = ColumnarDelta::new(1000);

        delta.insert("users", b"key1", b"value1").unwrap();
        delta.insert("users", b"key2", b"value2").unwrap();

        assert_eq!(
            delta.get("users", b"key1").unwrap(),
            Some(b"value1".to_vec())
        );
        assert_eq!(
            delta.get("users", b"key2").unwrap(),
            Some(b"value2".to_vec())
        );
        assert_eq!(delta.get("users", b"key3").unwrap(), None);
    }

    #[test]
    fn test_storage_backend_batch_insert() {
        use crate::storage::StorageBackend;

        let delta = ColumnarDelta::new(1000);

        let rows = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        StorageBackend::insert_batch(&delta, "users", rows).unwrap();

        assert_eq!(delta.count("users").unwrap(), 3);
        assert_eq!(
            delta.get("users", b"key2").unwrap(),
            Some(b"value2".to_vec())
        );
    }

    #[test]
    fn test_storage_backend_scan() {
        use crate::storage::StorageBackend;

        let delta = ColumnarDelta::new(1000);

        delta.insert("users", b"key1", b"value1").unwrap();
        delta.insert("users", b"key2", b"value2").unwrap();
        delta.insert("users", b"key3", b"value3").unwrap();

        let results = delta.scan("users", Vec::<u8>::new()..).unwrap();
        assert_eq!(results.len(), 3);

        // Results should be sorted by key
        assert_eq!(results[0].0, b"key1");
        assert_eq!(results[1].0, b"key2");
        assert_eq!(results[2].0, b"key3");
    }

    #[test]
    fn test_storage_backend_count() {
        use crate::storage::StorageBackend;

        let delta = ColumnarDelta::new(1000);

        assert_eq!(delta.count("users").unwrap(), 0);

        delta.insert("users", b"key1", b"value1").unwrap();
        assert_eq!(delta.count("users").unwrap(), 1);

        delta.insert("users", b"key2", b"value2").unwrap();
        assert_eq!(delta.count("users").unwrap(), 2);
    }

    #[test]
    fn test_storage_backend_table_names() {
        use crate::storage::StorageBackend;

        let delta = ColumnarDelta::new(1000);

        delta.insert("users", b"key1", b"value1").unwrap();
        delta.insert("orders", b"key2", b"value2").unwrap();

        let tables = ColumnarDelta::table_names(&delta);
        assert_eq!(tables.len(), 2);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }
}
