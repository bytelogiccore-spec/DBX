//! Compaction module - background data movement between storage tiers.
//!
//! Implements strategies for flushing and compacting data, including
//! the WOS bypass strategy for Columnar Delta.

use crate::engine::{Database, DeltaVariant};
use crate::error::DbxResult;
use crate::storage::parquet_io::ParquetWriter;
use rayon::prelude::*;
use std::path::Path;

pub struct Compactor;

impl Compactor {
    /// Flush ColumnarDelta directly to Parquet (Tier 5), bypassing WOS (Tier 3).
    ///
    /// P9: `batch_refs` 수집을 `par_iter()`로 병렬화합니다.
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

            // 2. P9: batch Arc::clone을 par_iter()로 병렬 수집
            use crate::storage::kv_adapter::merge_batches;
            let batch_refs: Vec<_> = versioned_batches
                .par_iter()
                .map(|vb| std::sync::Arc::clone(&vb.data))
                .collect();
            let merged_batch = merge_batches(batch_refs)?;

            // 3. Generate path and write Parquet
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();

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

    /// 여러 테이블을 병렬로 bypass_flush합니다. (P9 확장)
    ///
    /// 각 테이블이 독립적이므로 Rayon work-stealing으로 동시에 처리합니다.
    pub fn bypass_flush_tables(db: &Database, tables: &[&str]) -> Vec<DbxResult<()>> {
        tables
            .par_iter()
            .map(|&table| Self::bypass_flush(db, table))
            .collect()
    }
}
