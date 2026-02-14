//! GPU GROUP BY operations - SUM, COUNT, MIN/MAX.

#[cfg(feature = "gpu")]
use cudarc::driver::{LaunchConfig, PushKernelArg};

#[cfg(feature = "gpu")]
use super::data::GpuData;
use super::manager::GpuManager;
use crate::error::{DbxError, DbxResult};

/// GROUP BY operations impl block
impl GpuManager {
    /// GROUP BY with SUM aggregation on GPU.
    /// Returns Vec<(group_key, sum_value, count)>
    pub fn group_by_sum(
        &self,
        table: &str,
        group_column: &str,
        value_column: &str,
    ) -> DbxResult<Vec<(i32, i64, i32)>> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, group_column, value_column);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let keys_data = self.get_gpu_data(table, group_column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, group_column
                ))
            })?;
            let values_data = self.get_gpu_data(table, value_column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, value_column
                ))
            })?;

            let (keys_slice, n) = match &*keys_data {
                GpuData::Int32(slice) => (slice, slice.len()),
                _ => {
                    return Err(DbxError::NotImplemented(
                        "GROUP BY keys must be Int32".to_string(),
                    ));
                }
            };

            let values_slice = match &*values_data {
                GpuData::Int64(slice) => slice,
                _ => {
                    return Err(DbxError::NotImplemented(
                        "GROUP BY values must be Int64 for SUM".to_string(),
                    ));
                }
            };

            // Hash table size: ~2x input size for good performance
            let table_size = (n * 2).next_power_of_two();
            let stream = self.device.default_stream();

            // Allocate hash table (initialized to -1 for keys, 0 for sums/counts)
            let mut hash_keys = vec![-1i32; table_size];
            let mut hash_sums = vec![0i64; table_size];
            let mut hash_counts = vec![0i32; table_size];

            let mut hash_keys_dev = stream
                .clone_htod(&hash_keys)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash keys: {:?}", e)))?;
            let mut hash_sums_dev = stream
                .clone_htod(&hash_sums)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash sums: {:?}", e)))?;
            let mut hash_counts_dev = stream
                .clone_htod(&hash_counts)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash counts: {:?}", e)))?;

            // Launch kernel based on selected strategy
            let kernel_name = match self.hash_strategy {
                crate::storage::gpu::GpuHashStrategy::Linear => "group_by_sum_i32",
                crate::storage::gpu::GpuHashStrategy::Cuckoo => "group_by_sum_cuckoo_i32",
                crate::storage::gpu::GpuHashStrategy::RobinHood => "group_by_sum_robin_hood_i32",
            };

            let func = self
                .module
                .load_function(kernel_name)
                .map_err(|_| DbxError::Gpu(format!("Kernel {} not found", kernel_name)))?;

            let cfg = LaunchConfig::for_num_elems(n as u32);
            let n_i32 = n as i32;
            let table_size_i32 = table_size as i32;

            let mut builder = stream.launch_builder(&func);
            builder.arg(keys_slice);
            builder.arg(values_slice);
            builder.arg(&mut hash_keys_dev);
            builder.arg(&mut hash_sums_dev);
            builder.arg(&mut hash_counts_dev);
            builder.arg(&n_i32);
            builder.arg(&table_size_i32);
            unsafe { builder.launch(cfg) }
                .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;

            // Copy results back
            hash_keys = stream
                .clone_dtoh(&hash_keys_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash keys: {:?}", e)))?;
            hash_sums = stream
                .clone_dtoh(&hash_sums_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash sums: {:?}", e)))?;
            hash_counts = stream
                .clone_dtoh(&hash_counts_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash counts: {:?}", e)))?;

            // Extract non-empty groups
            let mut results = Vec::new();
            for i in 0..table_size {
                if hash_keys[i] != -1 {
                    results.push((hash_keys[i], hash_sums[i], hash_counts[i]));
                }
            }

            Ok(results)
        }
    }

    /// GROUP BY with COUNT aggregation on GPU.
    /// Returns Vec<(group_key, count)>
    pub fn group_by_count(&self, table: &str, group_column: &str) -> DbxResult<Vec<(i32, i32)>> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, group_column);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let keys_data = self.get_gpu_data(table, group_column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, group_column
                ))
            })?;

            let (keys_slice, n) = match &*keys_data {
                GpuData::Int32(slice) => (slice, slice.len()),
                _ => {
                    return Err(DbxError::NotImplemented(
                        "GROUP BY keys must be Int32".to_string(),
                    ));
                }
            };

            let table_size = (n * 2).next_power_of_two();
            let stream = self.device.default_stream();

            let mut hash_keys = vec![-1i32; table_size];
            let mut hash_counts = vec![0i32; table_size];

            let mut hash_keys_dev = stream
                .clone_htod(&hash_keys)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash keys: {:?}", e)))?;
            let mut hash_counts_dev = stream
                .clone_htod(&hash_counts)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash counts: {:?}", e)))?;

            let func = self
                .module
                .load_function("group_by_count_i32")
                .map_err(|_| DbxError::Gpu("Kernel group_by_count_i32 not found".to_string()))?;

            let cfg = LaunchConfig::for_num_elems(n as u32);
            let n_i32 = n as i32;
            let table_size_i32 = table_size as i32;

            let mut builder = stream.launch_builder(&func);
            builder.arg(keys_slice);
            builder.arg(&mut hash_keys_dev);
            builder.arg(&mut hash_counts_dev);
            builder.arg(&n_i32);
            builder.arg(&table_size_i32);
            unsafe { builder.launch(cfg) }
                .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;

            hash_keys = stream
                .clone_dtoh(&hash_keys_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash keys: {:?}", e)))?;
            hash_counts = stream
                .clone_dtoh(&hash_counts_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash counts: {:?}", e)))?;

            let mut results = Vec::new();
            for i in 0..table_size {
                if hash_keys[i] != -1 {
                    results.push((hash_keys[i], hash_counts[i]));
                }
            }

            Ok(results)
        }
    }

    /// GROUP BY with MIN/MAX aggregation on GPU.
    /// Returns Vec<(group_key, min_or_max_value, count)>
    pub fn group_by_min_max(
        &self,
        table: &str,
        group_column: &str,
        value_column: &str,
        find_max: bool,
    ) -> DbxResult<Vec<(i32, i32, i32)>> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, group_column, value_column, find_max);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let keys_data = self.get_gpu_data(table, group_column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, group_column
                ))
            })?;
            let values_data = self.get_gpu_data(table, value_column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, value_column
                ))
            })?;

            let (keys_slice, n) = match &*keys_data {
                GpuData::Int32(slice) => (slice, slice.len()),
                _ => {
                    return Err(DbxError::NotImplemented(
                        "GROUP BY keys must be Int32".to_string(),
                    ));
                }
            };

            let values_slice = match &*values_data {
                GpuData::Int32(slice) => slice,
                _ => {
                    return Err(DbxError::NotImplemented(
                        "GROUP BY values must be Int32 for MIN/MAX".to_string(),
                    ));
                }
            };

            let table_size = (n * 2).next_power_of_two();
            let stream = self.device.default_stream();

            let initial_val = if find_max { i32::MIN } else { i32::MAX };
            let mut hash_keys = vec![-1i32; table_size];
            let mut hash_values = vec![initial_val; table_size];
            let mut hash_counts = vec![0i32; table_size];

            let mut hash_keys_dev = stream
                .clone_htod(&hash_keys)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash keys: {:?}", e)))?;
            let mut hash_values_dev = stream
                .clone_htod(&hash_values)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash values: {:?}", e)))?;
            let mut hash_counts_dev = stream
                .clone_htod(&hash_counts)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash counts: {:?}", e)))?;

            let kernel_name = if find_max {
                "group_by_max_i32"
            } else {
                "group_by_min_i32"
            };
            let func = self
                .module
                .load_function(kernel_name)
                .map_err(|_| DbxError::Gpu(format!("Kernel {} not found", kernel_name)))?;

            let cfg = LaunchConfig::for_num_elems(n as u32);
            let n_i32 = n as i32;
            let table_size_i32 = table_size as i32;

            let mut builder = stream.launch_builder(&func);
            builder.arg(keys_slice);
            builder.arg(values_slice);
            builder.arg(&mut hash_keys_dev);
            builder.arg(&mut hash_values_dev);
            builder.arg(&mut hash_counts_dev);
            builder.arg(&n_i32);
            builder.arg(&table_size_i32);
            unsafe { builder.launch(cfg) }
                .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;

            hash_keys = stream
                .clone_dtoh(&hash_keys_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash keys: {:?}", e)))?;
            hash_values = stream
                .clone_dtoh(&hash_values_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash values: {:?}", e)))?;
            hash_counts = stream
                .clone_dtoh(&hash_counts_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy hash counts: {:?}", e)))?;

            let mut results = Vec::new();
            for i in 0..table_size {
                if hash_keys[i] != -1 {
                    results.push((hash_keys[i], hash_values[i], hash_counts[i]));
                }
            }

            Ok(results)
        }
    }
}
