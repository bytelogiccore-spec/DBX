//! GPU Data Sharding for Multi-GPU Distribution
//!
//! Distributes data across multiple GPUs for parallel processing.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use arrow::record_batch::RecordBatch;
#[cfg(feature = "gpu")]
use cudarc::driver::CudaContext;

use crate::error::{DbxError, DbxResult};

/// Sharding strategy for data distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShardingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Hash-based distribution
    Hash,
    /// Range-based distribution
    Range,
}

/// Data shard for a specific GPU
#[cfg(feature = "gpu")]
pub struct DataShard {
    /// Target device ID
    pub device_id: usize,
    /// Shard data (subset of original batch)
    pub batch: RecordBatch,
    /// Shard index
    pub shard_index: usize,
}

/// Shard Manager - distributes data across GPUs
#[cfg(feature = "gpu")]
pub struct ShardManager {
    /// Number of devices
    device_count: usize,
    /// Sharding strategy
    strategy: ShardingStrategy,
}

#[cfg(feature = "gpu")]
impl ShardManager {
    /// Create a new shard manager
    pub fn new(device_count: usize, strategy: ShardingStrategy) -> Self {
        Self {
            device_count,
            strategy,
        }
    }

    /// Shard a RecordBatch across devices
    pub fn shard_batch(&self, batch: &RecordBatch) -> DbxResult<Vec<DataShard>> {
        if self.device_count == 0 {
            return Err(DbxError::Gpu(
                "No devices available for sharding".to_string(),
            ));
        }

        if self.device_count == 1 {
            // Single device - no sharding needed
            return Ok(vec![DataShard {
                device_id: 0,
                batch: batch.clone(),
                shard_index: 0,
            }]);
        }

        match self.strategy {
            ShardingStrategy::RoundRobin => self.shard_round_robin(batch),
            ShardingStrategy::Hash => self.shard_hash(batch),
            ShardingStrategy::Range => self.shard_range(batch),
        }
    }

    /// Round-robin sharding - distribute rows evenly
    fn shard_round_robin(&self, batch: &RecordBatch) -> DbxResult<Vec<DataShard>> {
        let total_rows = batch.num_rows();
        let rows_per_shard = (total_rows + self.device_count - 1) / self.device_count;

        let mut shards = Vec::new();

        for device_id in 0..self.device_count {
            let start_row = device_id * rows_per_shard;
            if start_row >= total_rows {
                break;
            }

            let end_row = std::cmp::min(start_row + rows_per_shard, total_rows);
            let shard_batch = batch.slice(start_row, end_row - start_row);

            shards.push(DataShard {
                device_id,
                batch: shard_batch,
                shard_index: device_id,
            });
        }

        Ok(shards)
    }

    /// Hash-based sharding - distribute rows by hash of first column
    fn shard_hash(&self, batch: &RecordBatch) -> DbxResult<Vec<DataShard>> {
        use ahash::AHasher;
        use std::hash::{Hash, Hasher};

        let total_rows = batch.num_rows();
        // Assign each row to a device based on hash of the first column value
        let mut row_assignments: Vec<Vec<usize>> = vec![Vec::new(); self.device_count];

        let col = batch.column(0);
        for row_idx in 0..total_rows {
            let mut hasher = AHasher::default();
            // Hash the row index combined with column data for distribution
            format!("{:?}:{}", col.data_type(), row_idx).hash(&mut hasher);
            if let Some(arr) = col.as_any().downcast_ref::<arrow::array::Int32Array>() {
                arr.value(row_idx).hash(&mut hasher);
            } else if let Some(arr) = col.as_any().downcast_ref::<arrow::array::Int64Array>() {
                arr.value(row_idx).hash(&mut hasher);
            } else if let Some(arr) = col.as_any().downcast_ref::<arrow::array::StringArray>() {
                arr.value(row_idx).hash(&mut hasher);
            } else {
                row_idx.hash(&mut hasher);
            }
            let device_id = (hasher.finish() as usize) % self.device_count;
            row_assignments[device_id].push(row_idx);
        }

        let mut shards = Vec::new();
        for (device_id, indices) in row_assignments.into_iter().enumerate() {
            if indices.is_empty() {
                continue;
            }
            let idx_array = arrow::array::UInt32Array::from(
                indices.iter().map(|&i| i as u32).collect::<Vec<_>>(),
            );
            let columns: Vec<arrow::array::ArrayRef> = batch
                .columns()
                .iter()
                .map(|col| arrow::compute::take(col.as_ref(), &idx_array, None))
                .collect::<Result<Vec<_>, _>>()?;
            let shard_batch = RecordBatch::try_new(batch.schema(), columns)?;
            shards.push(DataShard {
                device_id,
                batch: shard_batch,
                shard_index: device_id,
            });
        }

        Ok(shards)
    }

    /// Range-based sharding - distribute contiguous row ranges to devices
    fn shard_range(&self, batch: &RecordBatch) -> DbxResult<Vec<DataShard>> {
        // Range sharding assigns contiguous blocks to each device
        // This is optimal when data is pre-sorted by the sharding key
        let total_rows = batch.num_rows();
        let rows_per_shard = (total_rows + self.device_count - 1) / self.device_count;

        let mut shards = Vec::new();

        for device_id in 0..self.device_count {
            let start_row = device_id * rows_per_shard;
            if start_row >= total_rows {
                break;
            }

            let length = std::cmp::min(rows_per_shard, total_rows - start_row);
            let shard_batch = batch.slice(start_row, length);

            shards.push(DataShard {
                device_id,
                batch: shard_batch,
                shard_index: device_id,
            });
        }

        Ok(shards)
    }

    /// Get the number of devices
    pub fn device_count(&self) -> usize {
        self.device_count
    }
}

// Stub implementation for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct ShardManager;

#[cfg(not(feature = "gpu"))]
impl ShardManager {
    pub fn new(_device_count: usize, _strategy: ()) -> Self {
        Self
    }
}
