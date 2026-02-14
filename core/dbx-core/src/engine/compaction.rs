//! Compaction module — background data movement between storage tiers.
//!
//! Implements strategies for flushing and compacting data, including
//! the WOS bypass strategy for Columnar Delta.

use crate::engine::{Database, DeltaVariant};
use crate::error::DbxResult;
use crate::storage::parquet_io::ParquetWriter;
use std::path::Path;

pub struct Compactor;

impl Compactor {
    /// Flush ColumnarDelta directly to Parquet (Tier 5), bypassing WOS (Tier 3).
    ///
    /// This implementation:
    /// 1. Drains batches from ColumnarDelta for the given table.
    /// 2. Merges them into a single RecordBatch.
    /// 3. Writes the batch to a new Parquet file in the ROS directory.
    pub fn bypass_flush(db: &Database, table: &str) -> DbxResult<()> {
        if let DeltaVariant::Columnar(delta) = &db.delta {
            // 1. Drain batches
            let versioned_batches = delta.drain_table(table);
            if versioned_batches.is_empty() {
                return Ok(());
            }

            // 2. Merge batches
            use crate::storage::kv_adapter::merge_batches;
            let batch_refs: Vec<_> = versioned_batches
                .iter()
                .map(|vb| std::sync::Arc::clone(&vb.data))
                .collect();
            let merged_batch = merge_batches(batch_refs)?;

            // 3. Generate path and write Parquet
            // We use a timestamp-based filename for uniqueness.
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();

            // For now, use a fixed data/ros directory relative to current working dir.
            // In a production implementation, this path would be part of Database config.
            let ros_dir = Path::new("data").join("ros").join(table);
            if let Err(e) = std::fs::create_dir_all(&ros_dir) {
                return Err(crate::error::DbxError::Storage(format!(
                    "Failed to create ROS directory: {}",
                    e
                )));
            }

            let file_path = ros_dir.join(format!("{}.parquet", timestamp));

            ParquetWriter::write(&file_path, &merged_batch)?;
        }
        Ok(())
    }
}
