//! GPU aggregation operations - SUM, COUNT, MIN/MAX, FILTER.

#[cfg(feature = "gpu")]
use cudarc::driver::{LaunchConfig, PushKernelArg};

#[cfg(feature = "gpu")]
use super::data::GpuData;
use super::manager::GpuManager;
use crate::error::{DbxError, DbxResult};

/// Aggregation operations impl block
impl GpuManager {
    /// SUM aggregation on GPU with configurable reduction strategy.
    pub fn sum(&self, table: &str, column: &str) -> DbxResult<i64> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, column);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            tracing::debug!(target: "gpu", table = %table, column = %column, "GPU sum start");
            let start = std::time::Instant::now();

            let data = self.get_gpu_data(table, column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, column
                ))
            })?;

            match &*data {
                GpuData::Int32(slice) => {
                    let n = slice.len() as i32;
                    let stream = self.device.default_stream();

                    // Choose reduction strategy
                    let strategy = self.reduction_strategy.choose_for_sum(slice.len());

                    match strategy {
                        GpuReductionStrategy::Histogram => {
                            // Histogram-based aggregation for low cardinality data
                            // Step 1: Copy data to CPU to detect cardinality (find min/max)
                            let slice_host = stream.clone_dtoh(slice).map_err(|e| {
                                DbxError::Gpu(format!("Failed to copy slice to host: {:?}", e))
                            })?;

                            let min_val = slice_host.iter().min().copied().unwrap_or(0);
                            let max_val = slice_host.iter().max().copied().unwrap_or(0);
                            let num_bins = (max_val - min_val + 1).min(1000) as usize;

                            // Only use histogram if cardinality is reasonable
                            if num_bins > 1000 || num_bins == 0 {
                                // Fall back to SinglePass for high cardinality
                                return Err(DbxError::Gpu(
                                    "Cardinality too high for histogram, use SinglePass"
                                        .to_string(),
                                ));
                            }

                            // Allocate histogram buffer
                            let mut histogram_dev =
                                stream.alloc_zeros::<i64>(num_bins).map_err(|e| {
                                    DbxError::Gpu(format!("Failed to alloc histogram: {:?}", e))
                                })?;

                            // Load histogram kernel
                            let func =
                                self.module
                                    .load_function("histogram_sum_i32")
                                    .map_err(|_| {
                                        DbxError::Gpu(
                                            "Kernel histogram_sum_i32 not found".to_string(),
                                        )
                                    })?;

                            // Launch histogram kernel
                            let cfg = LaunchConfig::for_num_elems(n as u32);
                            let shared_mem_bytes = (num_bins * std::mem::size_of::<i64>()) as u32;
                            let cfg_with_shared = LaunchConfig {
                                shared_mem_bytes,
                                ..cfg
                            };

                            let num_bins_i32 = num_bins as i32;
                            let mut builder = stream.launch_builder(&func);
                            builder.arg(slice);
                            builder.arg(slice); // keys = values for simple SUM
                            builder.arg(&mut histogram_dev);
                            builder.arg(&n);
                            builder.arg(&num_bins_i32);
                            unsafe { builder.launch(cfg_with_shared) }.map_err(|e| {
                                DbxError::Gpu(format!("Histogram kernel launch failed: {:?}", e))
                            })?;

                            // Synchronize and sum histogram
                            stream.synchronize().map_err(|e| {
                                DbxError::Gpu(format!("Stream sync failed: {:?}", e))
                            })?;

                            let histogram_host =
                                stream.clone_dtoh(&histogram_dev).map_err(|e| {
                                    DbxError::Gpu(format!("Failed to copy histogram: {:?}", e))
                                })?;

                            let result = histogram_host.iter().sum();
                            tracing::debug!(target: "gpu", table = %table, column = %column, strategy = "Histogram", elapsed_us = start.elapsed().as_micros(), "GPU sum complete");
                            Ok(result)
                        }
                        GpuReductionStrategy::SinglePass => {
                            // Single-pass with atomic operations
                            let mut result_dev = stream.alloc_zeros::<i64>(1).map_err(|e| {
                                DbxError::Gpu(format!("Failed to alloc result: {:?}", e))
                            })?;

                            let func = self.module.load_function("sum_i32").map_err(|_| {
                                DbxError::Gpu("Kernel sum_i32 not found".to_string())
                            })?;

                            let cfg = LaunchConfig::for_num_elems(n as u32);
                            let mut builder = stream.launch_builder(&func);
                            builder.arg(slice);
                            builder.arg(&mut result_dev);
                            builder.arg(&n);
                            unsafe { builder.launch(cfg) }.map_err(|e| {
                                DbxError::Gpu(format!("Kernel launch failed: {:?}", e))
                            })?;

                            stream.synchronize().map_err(|e| {
                                DbxError::Gpu(format!("Stream sync failed: {:?}", e))
                            })?;
                            let result_host = stream.clone_dtoh(&result_dev).map_err(|e| {
                                DbxError::Gpu(format!("Failed to copy result: {:?}", e))
                            })?;

                            tracing::debug!(target: "gpu", table = %table, column = %column, strategy = "SinglePass", elapsed_us = start.elapsed().as_micros(), "GPU sum complete");
                            Ok(result_host[0])
                        }
                        GpuReductionStrategy::MultiPass => {
                            // Multi-pass reduction (eliminates atomic contention)
                            let cfg = LaunchConfig::for_num_elems(n as u32);
                            let num_blocks = cfg.grid_dim.0 as usize;

                            // Allocate intermediate buffer for block partial sums
                            let mut block_sums_dev =
                                stream.alloc_zeros::<i64>(num_blocks).map_err(|e| {
                                    DbxError::Gpu(format!("Failed to alloc block_sums: {:?}", e))
                                })?;

                            // Pass 1: Compute block partial sums
                            let func_pass1 =
                                self.module.load_function("sum_i32_pass1").map_err(|_| {
                                    DbxError::Gpu("Kernel sum_i32_pass1 not found".to_string())
                                })?;

                            let mut builder = stream.launch_builder(&func_pass1);
                            builder.arg(slice);
                            builder.arg(&mut block_sums_dev);
                            builder.arg(&n);
                            unsafe { builder.launch(cfg) }.map_err(|e| {
                                DbxError::Gpu(format!("Pass1 kernel launch failed: {:?}", e))
                            })?;

                            // Pass 2: Final reduction of block sums
                            let mut result_dev = stream.alloc_zeros::<i64>(1).map_err(|e| {
                                DbxError::Gpu(format!("Failed to alloc result: {:?}", e))
                            })?;

                            let func_pass2 =
                                self.module.load_function("sum_i32_pass2").map_err(|_| {
                                    DbxError::Gpu("Kernel sum_i32_pass2 not found".to_string())
                                })?;

                            // Use single block for pass2 (num_blocks is usually small)
                            let cfg_pass2 = LaunchConfig {
                                grid_dim: (1, 1, 1),
                                block_dim: (256, 1, 1),
                                shared_mem_bytes: 0,
                            };

                            let mut builder2 = stream.launch_builder(&func_pass2);
                            builder2.arg(&block_sums_dev);
                            builder2.arg(&mut result_dev);
                            let num_blocks_i32 = num_blocks as i32;
                            builder2.arg(&num_blocks_i32);
                            unsafe { builder2.launch(cfg_pass2) }.map_err(|e| {
                                DbxError::Gpu(format!("Pass2 kernel launch failed: {:?}", e))
                            })?;

                            stream.synchronize().map_err(|e| {
                                DbxError::Gpu(format!("Stream sync failed: {:?}", e))
                            })?;
                            let result_host = stream.clone_dtoh(&result_dev).map_err(|e| {
                                DbxError::Gpu(format!("Failed to copy result: {:?}", e))
                            })?;

                            Ok(result_host[0])
                        }
                        GpuReductionStrategy::Auto => {
                            unreachable!("Auto should be resolved by choose_for_sum")
                        }
                    }
                }
                GpuData::PinnedInt32(_) => {
                    return Err(DbxError::NotImplemented(
                        "SUM for PinnedInt32 not implemented yet".to_string(),
                    ));
                }
                _ => Err(DbxError::NotImplemented(
                    "GPU SUM only supported for Int32 for now".to_string(),
                )),
            }
        }
    }

    /// COUNT aggregation on GPU (single-pass for simplicity).
    pub fn count(&self, table: &str, column: &str) -> DbxResult<u64> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, column);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let data = self.get_gpu_data(table, column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, column
                ))
            })?;

            let n = data.len() as i32;
            let stream = self.device.default_stream();
            let mut result_dev = stream
                .alloc_zeros::<i64>(1)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc result: {:?}", e)))?;

            let func = self
                .module
                .load_function("count_all")
                .map_err(|_| DbxError::Gpu("Kernel count_all not found".to_string()))?;

            let cfg = LaunchConfig::for_num_elems(n as u32);

            let mut builder = stream.launch_builder(&func);
            match &*data {
                GpuData::Int32(s) => {
                    builder.arg(s);
                    builder.arg(&mut result_dev);
                    builder.arg(&n);
                }
                GpuData::Int64(s) => {
                    builder.arg(s);
                    builder.arg(&mut result_dev);
                    builder.arg(&n);
                }
                GpuData::Float64(s) => {
                    builder.arg(s);
                    builder.arg(&mut result_dev);
                    builder.arg(&n);
                }
                GpuData::Raw(s) => {
                    builder.arg(s);
                    builder.arg(&mut result_dev);
                    builder.arg(&n);
                }
                GpuData::PinnedInt32(_) => {
                    return Err(DbxError::NotImplemented(
                        "COUNT for PinnedInt32 not implemented yet".to_string(),
                    ));
                }
            }
            unsafe { builder.launch(cfg) }
                .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;
            let result_host = stream
                .clone_dtoh(&result_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy result: {:?}", e)))?;

            Ok(result_host[0] as u64)
        }
    }

    /// MIN/MAX aggregation on GPU.
    pub fn min_max(&self, table: &str, column: &str, find_max: bool) -> DbxResult<i32> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, column, find_max);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let data = self.get_gpu_data(table, column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, column
                ))
            })?;

            match &*data {
                GpuData::Int32(slice) => {
                    let n = slice.len() as i32;
                    let initial_val = if find_max { i32::MIN } else { i32::MAX };
                    let stream = self.device.default_stream();
                    let mut result_dev = stream
                        .clone_htod(&[initial_val])
                        .map_err(|e| DbxError::Gpu(format!("Failed to alloc result: {:?}", e)))?;

                    let kernel_name = if find_max { "max_i32" } else { "min_i32" };
                    let func = self
                        .module
                        .load_function(kernel_name)
                        .map_err(|_| DbxError::Gpu(format!("Kernel {} not found", kernel_name)))?;

                    let cfg = LaunchConfig::for_num_elems(n as u32);
                    let mut builder = stream.launch_builder(&func);
                    builder.arg(slice);
                    builder.arg(&mut result_dev);
                    builder.arg(&n);
                    unsafe { builder.launch(cfg) }
                        .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

                    stream
                        .synchronize()
                        .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;
                    let result_host = stream
                        .clone_dtoh(&result_dev)
                        .map_err(|e| DbxError::Gpu(format!("Failed to copy result: {:?}", e)))?;

                    Ok(result_host[0])
                }
                GpuData::PinnedInt32(_) => {
                    return Err(DbxError::NotImplemented(
                        "MIN/MAX for PinnedInt32 not implemented yet".to_string(),
                    ));
                }
                _ => Err(DbxError::NotImplemented(
                    "GPU MIN/MAX only supported for Int32 for now".to_string(),
                )),
            }
        }
    }

    /// Filter GT on GPU. Returns a bitmask (Vec<u8> where 1 means true).
    pub fn filter_gt(&self, table: &str, column: &str, threshold: i32) -> DbxResult<Vec<u8>> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, column, threshold);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let data = self.get_gpu_data(table, column).ok_or_else(|| {
                DbxError::Gpu(format!(
                    "Column {}.{} not found in GPU cache",
                    table, column
                ))
            })?;

            match &*data {
                GpuData::Int32(slice) => {
                    let n = slice.len() as i32;
                    let stream = self.device.default_stream();
                    let mut mask_dev = stream
                        .alloc_zeros::<u8>(n as usize)
                        .map_err(|e| DbxError::Gpu(format!("Failed to alloc mask: {:?}", e)))?;

                    let func = self
                        .module
                        .load_function("filter_gt_i32")
                        .map_err(|_| DbxError::Gpu("Kernel filter_gt_i32 not found".to_string()))?;

                    let cfg = LaunchConfig::for_num_elems(n as u32);
                    let mut builder = stream.launch_builder(&func);
                    builder.arg(slice);
                    builder.arg(&threshold);
                    builder.arg(&mut mask_dev);
                    builder.arg(&n);
                    unsafe { builder.launch(cfg) }
                        .map_err(|e| DbxError::Gpu(format!("Kernel launch failed: {:?}", e)))?;

                    stream
                        .synchronize()
                        .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;
                    let mask_host = stream
                        .clone_dtoh(&mask_dev)
                        .map_err(|e| DbxError::Gpu(format!("Failed to copy mask: {:?}", e)))?;

                    Ok(mask_host)
                }
                GpuData::PinnedInt32(_) => {
                    return Err(DbxError::NotImplemented(
                        "FILTER for PinnedInt32 not implemented yet".to_string(),
                    ));
                }
                _ => Err(DbxError::NotImplemented(
                    "GPU FILTER only supported for Int32 for now".to_string(),
                )),
            }
        }
    }

    #[cfg(feature = "gpu")]
    /// SUM aggregation across two tiers (e.g. Delta and ROS) using a single merge kernel.
    pub fn merge_sum(
        &self,
        _table: &str,
        _column: &str,
        delta_data: &super::data::GpuData,
        ros_data: &super::data::GpuData,
    ) -> DbxResult<i64> {
        match (delta_data, ros_data) {
            (GpuData::Int32(delta_slice), GpuData::Int32(ros_slice)) => {
                let delta_n = delta_slice.len() as i32;
                let ros_n = ros_slice.len() as i32;
                let stream = self.device.default_stream();

                let mut result_dev = stream
                    .alloc_zeros::<i64>(1)
                    .map_err(|e| DbxError::Gpu(format!("Failed to alloc result: {:?}", e)))?;

                let func = self
                    .module
                    .load_function("merge_sum_i32")
                    .map_err(|_| DbxError::Gpu("Kernel merge_sum_i32 not found".to_string()))?;

                // Configure based on the larger dataset
                let cfg = LaunchConfig::for_num_elems(std::cmp::max(delta_n, ros_n) as u32);

                let mut builder = stream.launch_builder(&func);
                builder.arg(delta_slice);
                builder.arg(&delta_n);
                builder.arg(ros_slice);
                builder.arg(&ros_n);
                builder.arg(&mut result_dev);

                unsafe { builder.launch(cfg) }
                    .map_err(|e| DbxError::Gpu(format!("Merge kernel launch failed: {:?}", e)))?;

                stream
                    .synchronize()
                    .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))?;
                let result_host = stream
                    .clone_dtoh(&result_dev)
                    .map_err(|e| DbxError::Gpu(format!("Failed to copy result: {:?}", e)))?;

                Ok(result_host[0])
            }
            _ => Err(DbxError::NotImplemented(
                "GPU Merge SUM only supported for Int32".to_string(),
            )),
        }
    }
}
