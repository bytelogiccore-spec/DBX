//! GPU-accelerated radix sort implementation
//!
//! High-performance radix sort for GPU-based sorting operations

#[cfg(feature = "gpu")]
use cudarc::driver::{CudaSlice, LaunchConfig, PushKernelArg};

#[cfg(feature = "gpu")]
use super::data::GpuData;
use super::manager::GpuManager;

/// Radix Sort operations impl block
impl GpuManager {
    /// Perform radix sort on GPU data (keys and values)
    /// Returns sorted (keys, values) as GPU slices
    #[cfg(feature = "gpu")]
    pub(super) fn radix_sort_i32_i64(
        &self,
        keys: &CudaSlice<i32>,
        values: &CudaSlice<i64>,
    ) -> DbxResult<(CudaSlice<i32>, CudaSlice<i64>)> {
        let n = keys.len();
        let stream = self.device.default_stream();

        // Allocate temporary buffers
        let mut keys_buf1 = keys.clone();
        let mut keys_buf2 = stream
            .alloc_zeros::<i32>(n)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc keys buffer: {:?}", e)))?;

        let mut values_buf1 = values.clone();
        let mut values_buf2 = stream
            .alloc_zeros::<i64>(n)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc values buffer: {:?}", e)))?;

        // Radix sort: 4 passes for 32-bit integers (8 bits per pass)
        for pass in 0..4 {
            let bit_shift = pass * 8;

            // Determine input/output buffers (ping-pong)
            let (in_keys, out_keys) = if pass % 2 == 0 {
                (&keys_buf1, &mut keys_buf2)
            } else {
                (&keys_buf2, &mut keys_buf1)
            };

            let (in_values, out_values) = if pass % 2 == 0 {
                (&values_buf1, &mut values_buf2)
            } else {
                (&values_buf2, &mut values_buf1)
            };

            // Allocate temporary histogram and prefix sum buffers
            let mut temp_histogram = stream
                .alloc_zeros::<i32>(256)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc histogram: {:?}", e)))?;
            let mut temp_prefix = stream
                .alloc_zeros::<i32>(256)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc prefix: {:?}", e)))?;

            // Launch radix sort pass kernel
            let func = self
                .module
                .load_function("radix_sort_pass_i32")
                .map_err(|_| DbxError::Gpu("Kernel radix_sort_pass_i32 not found".to_string()))?;

            let cfg = LaunchConfig::for_num_elems(n as u32);
            let n_i32 = n as i32;
            let bit_shift_i32 = bit_shift as i32;

            let mut builder = stream.launch_builder(&func);
            builder.arg(in_keys);
            builder.arg(out_keys);
            builder.arg(in_values);
            builder.arg(out_values);
            builder.arg(&mut temp_histogram);
            builder.arg(&mut temp_prefix);
            builder.arg(&n_i32);
            builder.arg(&bit_shift_i32);

            unsafe { builder.launch(cfg) }
                .map_err(|e| DbxError::Gpu(format!("Radix sort pass {} failed: {:?}", pass, e)))?;
        }

        stream
            .synchronize()
            .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;

        // Return final sorted buffers (after 4 passes, data is in buf1)
        Ok((keys_buf1, values_buf1))
    }

    /// GROUP BY with SUM using radix sort (for high cardinality)
    /// This is more efficient than hash-based GROUP BY when there are many unique groups
    #[cfg(feature = "gpu")]
    pub(super) fn group_by_sum_sorted(
        &self,
        table: &str,
        group_column: &str,
        value_column: &str,
    ) -> DbxResult<Vec<(i32, i64, i32)>> {
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

        // Step 1: Radix sort by keys
        let (sorted_keys, sorted_values) = self.radix_sort_i32_i64(keys_slice, values_slice)?;

        // Step 2: Aggregate sorted data
        let stream = self.device.default_stream();

        // Allocate output buffers (max possible groups = n)
        let mut out_keys = stream
            .alloc_zeros::<i32>(n)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc out_keys: {:?}", e)))?;
        let mut out_sums = stream
            .alloc_zeros::<i64>(n)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc out_sums: {:?}", e)))?;
        let mut out_counts = stream
            .alloc_zeros::<i32>(n)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc out_counts: {:?}", e)))?;
        let mut num_groups_dev = stream
            .alloc_zeros::<i32>(1)
            .map_err(|e| DbxError::Gpu(format!("Failed to alloc num_groups: {:?}", e)))?;

        // Launch sorted aggregation kernel
        let func = self
            .module
            .load_function("sorted_group_by_sum_i32")
            .map_err(|_| DbxError::Gpu("Kernel sorted_group_by_sum_i32 not found".to_string()))?;

        let cfg = LaunchConfig::for_num_elems(n as u32);
        let n_i32 = n as i32;

        let mut builder = stream.launch_builder(&func);
        builder.arg(&sorted_keys);
        builder.arg(&sorted_values);
        builder.arg(&mut out_keys);
        builder.arg(&mut out_sums);
        builder.arg(&mut out_counts);
        builder.arg(&mut num_groups_dev);
        builder.arg(&n_i32);

        unsafe { builder.launch(cfg) }
            .map_err(|e| DbxError::Gpu(format!("Sorted aggregation failed: {:?}", e)))?;

        stream
            .synchronize()
            .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;

        // Copy results back
        let num_groups_vec = stream
            .clone_dtoh(&num_groups_dev)
            .map_err(|e| DbxError::Gpu(format!("Failed to copy num_groups: {:?}", e)))?;
        let num_groups = num_groups_vec[0] as usize;

        let keys_vec = stream
            .clone_dtoh(&out_keys)
            .map_err(|e| DbxError::Gpu(format!("Failed to copy keys: {:?}", e)))?;
        let sums_vec = stream
            .clone_dtoh(&out_sums)
            .map_err(|e| DbxError::Gpu(format!("Failed to copy sums: {:?}", e)))?;
        let counts_vec = stream
            .clone_dtoh(&out_counts)
            .map_err(|e| DbxError::Gpu(format!("Failed to copy counts: {:?}", e)))?;

        // Extract actual groups
        let mut results = Vec::with_capacity(num_groups);
        for i in 0..num_groups {
            results.push((keys_vec[i], sums_vec[i], counts_vec[i]));
        }

        Ok(results)
    }
}
