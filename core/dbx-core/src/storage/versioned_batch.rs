//! Versioned RecordBatch for MVCC Columnar Storage.
//!
//! Combines Arrow RecordBatch with MVCC version metadata for Snapshot Isolation.

use arrow::record_batch::RecordBatch;
use std::sync::Arc;

/// A versioned RecordBatch with MVCC metadata.
///
/// Stores an immutable Arrow RecordBatch along with version information
/// for Snapshot Isolation. Multiple versions can share the same RecordBatch
/// via Arc, reducing memory overhead.
#[derive(Debug, Clone)]
pub struct VersionedBatch {
    /// The actual columnar data (immutable, shared via Arc)
    pub data: Arc<RecordBatch>,

    /// Transaction timestamp when this batch became visible
    pub begin_ts: u64,

    /// Transaction timestamp when this batch became obsolete (None = still active)
    pub end_ts: Option<u64>,

    /// Batch sequence number for ordering within the same timestamp
    pub sequence: u64,
}

impl VersionedBatch {
    /// Create a new versioned batch.
    pub fn new(data: Arc<RecordBatch>, begin_ts: u64, sequence: u64) -> Self {
        Self {
            data,
            begin_ts,
            end_ts: None,
            sequence,
        }
    }

    /// Mark this batch as obsolete at the given timestamp.
    pub fn mark_obsolete(&mut self, end_ts: u64) {
        self.end_ts = Some(end_ts);
    }

    /// Check if this batch is visible to a snapshot at the given read timestamp.
    ///
    /// A batch is visible if:
    /// - begin_ts <= read_ts
    /// - end_ts is None OR end_ts > read_ts
    pub fn is_visible(&self, read_ts: u64) -> bool {
        self.begin_ts <= read_ts && self.end_ts.is_none_or(|end| end > read_ts)
    }

    /// Check if this batch is obsolete (has an end_ts).
    pub fn is_obsolete(&self) -> bool {
        self.end_ts.is_some()
    }

    /// Get the number of rows in this batch.
    pub fn num_rows(&self) -> usize {
        self.data.num_rows()
    }

    /// Get the number of columns in this batch.
    pub fn num_columns(&self) -> usize {
        self.data.num_columns()
    }
}

/// Metadata for tracking versions of a specific key or row.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    /// The key or row identifier
    pub key: Vec<u8>,

    /// List of batch sequences that contain versions of this key
    /// Sorted by (begin_ts, sequence) in descending order (newest first)
    pub batch_sequences: Vec<u64>,
}

impl VersionInfo {
    /// Create new version info for a key.
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            key,
            batch_sequences: Vec::new(),
        }
    }

    /// Add a new version (batch sequence) for this key.
    ///
    /// Maintains descending order by timestamp.
    pub fn add_version(&mut self, sequence: u64) {
        // Insert in descending order
        match self
            .batch_sequences
            .binary_search_by(|s| s.cmp(&sequence).reverse())
        {
            Ok(_) => {} // Already exists
            Err(pos) => self.batch_sequences.insert(pos, sequence),
        }
    }

    /// Get the latest version visible to the given read timestamp.
    pub fn get_visible_version(&self, batches: &[VersionedBatch], read_ts: u64) -> Option<u64> {
        for &seq in &self.batch_sequences {
            if let Some(batch) = batches.iter().find(|b| b.sequence == seq)
                && batch.is_visible(read_ts)
            {
                return Some(seq);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    fn create_test_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let id_array = Int32Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["Alice", "Bob", "Charlie"]);

        RecordBatch::try_new(schema, vec![Arc::new(id_array), Arc::new(name_array)]).unwrap()
    }

    #[test]
    fn test_versioned_batch_creation() {
        let batch = create_test_batch();
        let versioned = VersionedBatch::new(Arc::new(batch), 10, 1);

        assert_eq!(versioned.begin_ts, 10);
        assert_eq!(versioned.end_ts, None);
        assert_eq!(versioned.sequence, 1);
        assert_eq!(versioned.num_rows(), 3);
        assert!(!versioned.is_obsolete());
    }

    #[test]
    fn test_visibility() {
        let batch = create_test_batch();
        let mut versioned = VersionedBatch::new(Arc::new(batch), 10, 1);

        // Visible to snapshots at ts >= 10
        assert!(!versioned.is_visible(5));
        assert!(versioned.is_visible(10));
        assert!(versioned.is_visible(15));

        // Mark obsolete at ts=20
        versioned.mark_obsolete(20);

        // Now only visible to snapshots in [10, 20)
        assert!(!versioned.is_visible(5));
        assert!(versioned.is_visible(10));
        assert!(versioned.is_visible(15));
        assert!(!versioned.is_visible(20));
        assert!(!versioned.is_visible(25));
        assert!(versioned.is_obsolete());
    }

    #[test]
    fn test_version_info() {
        let mut info = VersionInfo::new(b"key1".to_vec());

        info.add_version(1);
        info.add_version(3);
        info.add_version(2);

        // Should be sorted in descending order
        assert_eq!(info.batch_sequences, vec![3, 2, 1]);
    }

    #[test]
    fn test_get_visible_version() {
        let batch1 = create_test_batch();
        let batch2 = create_test_batch();
        let batch3 = create_test_batch();

        let mut v1 = VersionedBatch::new(Arc::new(batch1), 10, 1);
        let mut v2 = VersionedBatch::new(Arc::new(batch2), 20, 2);
        let v3 = VersionedBatch::new(Arc::new(batch3), 30, 3);

        v1.mark_obsolete(20); // Obsolete at ts=20
        v2.mark_obsolete(30); // Obsolete at ts=30

        let batches = vec![v1, v2, v3];

        let mut info = VersionInfo::new(b"key1".to_vec());
        info.add_version(1);
        info.add_version(2);
        info.add_version(3);

        // At ts=15, should see version 1
        assert_eq!(info.get_visible_version(&batches, 15), Some(1));

        // At ts=25, should see version 2
        assert_eq!(info.get_visible_version(&batches, 25), Some(2));

        // At ts=35, should see version 3
        assert_eq!(info.get_visible_version(&batches, 35), Some(3));

        // At ts=5, nothing visible
        assert_eq!(info.get_visible_version(&batches, 5), None);
    }

    #[test]
    fn test_arc_sharing() {
        let batch = Arc::new(create_test_batch());

        let v1 = VersionedBatch::new(Arc::clone(&batch), 10, 1);
        let v2 = VersionedBatch::new(Arc::clone(&batch), 20, 2);

        // Both versions share the same RecordBatch
        assert_eq!(Arc::strong_count(&batch), 3); // original + v1 + v2
        assert_eq!(v1.num_rows(), v2.num_rows());
    }
}
