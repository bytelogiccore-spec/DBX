//! GPU Hash Join operations.

#[cfg(feature = "gpu")]
use cudarc::driver::{LaunchConfig, PushKernelArg};

#[cfg(feature = "gpu")]
use super::data::GpuData;
use super::manager::GpuManager;
use crate::error::{DbxError, DbxResult};

/// Hash Join operations impl block
impl GpuManager {
    /// Hash Join on GPU: Build + Probe phases.
    /// Returns Vec<(probe_row_id, build_row_id)> for matched rows.
    pub fn hash_join(
        &self,
        build_table: &str,
        build_key_column: &str,
        probe_table: &str,
        probe_key_column: &str,
    ) -> DbxResult<Vec<(i32, i32)>> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (build_table, build_key_column, probe_table, probe_key_column);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            // Get build side keys
            let build_keys_data = self
                .get_gpu_data(build_table, build_key_column)
                .ok_or_else(|| {
                    DbxError::Gpu(format!(
                        "Column {}.{} not found in GPU cache",
                        build_table, build_key_column
                    ))
                })?;
            let (build_keys_slice, build_n) = match &*build_keys_data {
                GpuData::Int32(slice) => (slice, slice.len()),
                _ => {
                    return Err(DbxError::NotImplemented(
                        "Hash join keys must be Int32".to_string(),
                    ));
                }
            };

            // Get probe side keys
            let probe_keys_data = self
                .get_gpu_data(probe_table, probe_key_column)
                .ok_or_else(|| {
                    DbxError::Gpu(format!(
                        "Column {}.{} not found in GPU cache",
                        probe_table, probe_key_column
                    ))
                })?;
            let (probe_keys_slice, probe_n) = match &*probe_keys_data {
                GpuData::Int32(slice) => (slice, slice.len()),
                _ => {
                    return Err(DbxError::NotImplemented(
                        "Hash join keys must be Int32".to_string(),
                    ));
                }
            };

            let stream = self.device.default_stream();

            // Phase 1: Build hash table
            let table_size = (build_n * 2).next_power_of_two();
            let mut hash_table_keys = vec![-1i32; table_size];
            let mut hash_table_row_ids = vec![-1i32; table_size];

            let mut hash_table_keys_dev = stream
                .clone_htod(&hash_table_keys)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc hash table keys: {:?}", e)))?;
            let mut hash_table_row_ids_dev =
                stream.clone_htod(&hash_table_row_ids).map_err(|e| {
                    DbxError::Gpu(format!("Failed to alloc hash table row IDs: {:?}", e))
                })?;

            // Create build row IDs (0, 1, 2, ...)
            let build_row_ids: Vec<i32> = (0..build_n as i32).collect();
            let build_row_ids_dev = stream
                .clone_htod(&build_row_ids)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc build row IDs: {:?}", e)))?;

            let build_func = self
                .module
                .load_function("hash_join_build_i32")
                .map_err(|_| DbxError::Gpu("Kernel hash_join_build_i32 not found".to_string()))?;

            let build_cfg = LaunchConfig::for_num_elems(build_n as u32);
            let build_n_i32 = build_n as i32;
            let table_size_i32 = table_size as i32;

            let mut builder = stream.launch_builder(&build_func);
            builder.arg(build_keys_slice);
            builder.arg(&build_row_ids_dev);
            builder.arg(&mut hash_table_keys_dev);
            builder.arg(&mut hash_table_row_ids_dev);
            builder.arg(&build_n_i32);
            builder.arg(&table_size_i32);
            unsafe { builder.launch(build_cfg) }
                .map_err(|e| DbxError::Gpu(format!("Build kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Build stream sync failed: {:?}", e)))?;

            // Phase 2: Probe hash table
            let max_output_size = probe_n * 2; // Conservative estimate
            let mut output_probe_ids = vec![0i32; max_output_size];
            let mut output_build_ids = vec![0i32; max_output_size];
            let mut match_count = vec![0i32; 1];

            let mut output_probe_ids_dev = stream
                .clone_htod(&output_probe_ids)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc output probe IDs: {:?}", e)))?;
            let mut output_build_ids_dev = stream
                .clone_htod(&output_build_ids)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc output build IDs: {:?}", e)))?;
            let mut match_count_dev = stream
                .clone_htod(&match_count)
                .map_err(|e| DbxError::Gpu(format!("Failed to alloc match count: {:?}", e)))?;

            let probe_func = self
                .module
                .load_function("hash_join_probe_i32")
                .map_err(|_| DbxError::Gpu("Kernel hash_join_probe_i32 not found".to_string()))?;

            let probe_cfg = LaunchConfig::for_num_elems(probe_n as u32);
            let probe_n_i32 = probe_n as i32;
            let max_output_size_i32 = max_output_size as i32;

            let mut builder = stream.launch_builder(&probe_func);
            builder.arg(probe_keys_slice);
            builder.arg(&hash_table_keys_dev);
            builder.arg(&hash_table_row_ids_dev);
            builder.arg(&mut output_probe_ids_dev);
            builder.arg(&mut output_build_ids_dev);
            builder.arg(&mut match_count_dev);
            builder.arg(&probe_n_i32);
            builder.arg(&table_size_i32);
            builder.arg(&max_output_size_i32);
            unsafe { builder.launch(probe_cfg) }
                .map_err(|e| DbxError::Gpu(format!("Probe kernel launch failed: {:?}", e)))?;

            stream
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Probe stream sync failed: {:?}", e)))?;

            // Copy results back
            match_count = stream
                .clone_dtoh(&match_count_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy match count: {:?}", e)))?;
            let actual_matches = match_count[0] as usize;

            output_probe_ids = stream
                .clone_dtoh(&output_probe_ids_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy output probe IDs: {:?}", e)))?;
            output_build_ids = stream
                .clone_dtoh(&output_build_ids_dev)
                .map_err(|e| DbxError::Gpu(format!("Failed to copy output build IDs: {:?}", e)))?;

            // Extract matched pairs
            let mut results = Vec::new();
            for i in 0..actual_matches.min(max_output_size) {
                results.push((output_probe_ids[i], output_build_ids[i]));
            }

            Ok(results)
        }
    }
}
