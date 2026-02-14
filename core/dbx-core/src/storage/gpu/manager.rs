//! GPU Manager core - initialization, data upload, and cache management.

#[cfg(feature = "gpu")]
use arrow::array::{Array, Float64Array, Int32Array, Int64Array};
#[cfg(feature = "gpu")]
use cudarc::driver::{CudaContext, CudaModule, PushKernelArg};
#[cfg(feature = "gpu")]
use cudarc::nvrtc::compile_ptx;
#[cfg(feature = "gpu")]
use dashmap::DashMap;

use arrow::record_batch::RecordBatch;

#[cfg(feature = "gpu")]
use super::data::GpuData;
#[cfg(feature = "gpu")]
use super::memory_pool::GpuMemoryPool;
use super::strategy::{GpuHashStrategy, GpuReductionStrategy};
use crate::error::{DbxError, DbxResult};

#[cfg(feature = "gpu")]
const KERNELS_SRC: &str = include_str!("../kernels.cu");

/// Manager for GPU-accelerated operations.
pub struct GpuManager {
    /// CUDA device context (pub(super) for impl blocks in other files)
    #[cfg(feature = "gpu")]
    pub(super) device: Arc<CudaContext>,

    /// Compiled CUDA module (pub(super) for impl blocks in other files)
    #[cfg(feature = "gpu")]
    pub(super) module: Arc<CudaModule>,

    /// Buffer cache: table_name -> column_name -> GpuData
    /// This avoids re-uploading data that hasn't changed.
    #[cfg(feature = "gpu")]
    pub(super) buffer_cache: DashMap<String, DashMap<String, Arc<GpuData>>>,

    /// Hash strategy for GROUP BY operations (runtime configurable)
    pub(super) hash_strategy: GpuHashStrategy,

    /// Reduction strategy for SUM operations (runtime configurable)
    pub(super) reduction_strategy: GpuReductionStrategy,

    /// Memory pool for efficient GPU memory allocation
    #[cfg(feature = "gpu")]
    pub(super) memory_pool: Arc<GpuMemoryPool>,
}

impl GpuManager {
    /// Create a new GpuManager. Returns None if GPU acceleration is disabled
    /// or if no compatible device is found.
    pub fn try_new() -> Option<Self> {
        #[cfg(feature = "gpu")]
        {
            let device = match CudaContext::new(0) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!(
                        "⚠️  GPU Manager: Failed to initialize CUDA device 0: {:?}",
                        e
                    );
                    return None;
                }
            };

            // Compile and Load kernels
            let ptx = match compile_ptx(KERNELS_SRC) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("⚠️  GPU Manager: Failed to compile PTX kernels: {:?}", e);
                    return None;
                }
            };

            let module = match device.load_module(ptx) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("⚠️  GPU Manager: Failed to load CUDA module: {:?}", e);
                    return None;
                }
            };

            let memory_pool = Arc::new(GpuMemoryPool::new(
                device.clone(),
                256, // 256MB default cache
            ));

            eprintln!("✅ GPU Manager initialized successfully");
            Some(Self {
                device,
                module,
                buffer_cache: DashMap::new(),
                hash_strategy: GpuHashStrategy::default(), // Linear by default
                reduction_strategy: GpuReductionStrategy::default(), // Auto by default
                memory_pool,
            })
        }
        #[cfg(not(feature = "gpu"))]
        {
            #[allow(unreachable_code)]
            {
                None
            }
        }
    }

    /// Set GPU hash strategy for GROUP BY operations
    pub fn set_hash_strategy(&mut self, strategy: GpuHashStrategy) {
        self.hash_strategy = strategy;
    }

    /// Get current GPU hash strategy
    pub fn hash_strategy(&self) -> GpuHashStrategy {
        self.hash_strategy
    }

    /// Set GPU reduction strategy for SUM operations
    pub fn set_reduction_strategy(&mut self, strategy: GpuReductionStrategy) {
        self.reduction_strategy = strategy;
    }

    /// Get current GPU reduction strategy
    pub fn reduction_strategy(&self) -> GpuReductionStrategy {
        self.reduction_strategy
    }

    /// Upload a RecordBatch to GPU memory and cache it.
    pub fn upload_batch(&self, table: &str, batch: &RecordBatch) -> DbxResult<()> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, batch);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            tracing::debug!(target: "gpu", table = %table, rows = batch.num_rows(), "GPU upload_batch start");
            let start = std::time::Instant::now();

            let table_cache = self
                .buffer_cache
                .entry(table.to_string())
                .or_insert_with(DashMap::new);
            let schema = batch.schema();

            for (i, column) in batch.columns().iter().enumerate() {
                let column_name = schema.field(i).name();
                if table_cache.contains_key(column_name) {
                    continue;
                }

                let gpu_data = self.convert_and_upload(column)?;
                table_cache.insert(column_name.clone(), Arc::new(gpu_data));
            }

            tracing::debug!(target: "gpu", table = %table, elapsed_us = start.elapsed().as_micros(), "GPU upload_batch complete");
            Ok(())
        }
    }

    #[cfg(feature = "gpu")]
    fn convert_and_upload(&self, array: &Arc<dyn Array>) -> DbxResult<GpuData> {
        match array.data_type() {
            arrow::datatypes::DataType::Int32 => {
                let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
                let stream = self.device.default_stream();
                // Zero-copy: Access the underlying slice directly
                let slice = stream
                    .clone_htod(&arr.values()[..])
                    .map_err(|e| DbxError::Gpu(format!("CUDA HTOD copy (i32) failed: {:?}", e)))?;
                Ok(GpuData::Int32(slice))
            }
            arrow::datatypes::DataType::Int64 => {
                let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                let stream = self.device.default_stream();
                let slice = stream
                    .clone_htod(&arr.values()[..])
                    .map_err(|e| DbxError::Gpu(format!("CUDA HTOD copy (i64) failed: {:?}", e)))?;
                Ok(GpuData::Int64(slice))
            }
            arrow::datatypes::DataType::Float64 => {
                let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
                let stream = self.device.default_stream();
                let slice = stream
                    .clone_htod(&arr.values()[..])
                    .map_err(|e| DbxError::Gpu(format!("CUDA HTOD copy (f64) failed: {:?}", e)))?;
                Ok(GpuData::Float64(slice))
            }
            _ => Err(DbxError::NotImplemented(format!(
                "GPU upload for type {:?} not supported yet",
                array.data_type()
            ))),
        }
    }

    /// Upload a RecordBatch to GPU memory using Pinned Memory for faster DMA transfer.
    pub fn upload_batch_pinned(&self, table: &str, batch: &RecordBatch) -> DbxResult<()> {
        #[cfg(not(feature = "gpu"))]
        {
            let _ = (table, batch);
            Err(DbxError::NotImplemented(
                "GPU acceleration is not enabled".to_string(),
            ))
        }

        #[cfg(feature = "gpu")]
        {
            let table_cache = self
                .buffer_cache
                .entry(table.to_string())
                .or_insert_with(DashMap::new);
            let schema = batch.schema();

            for (i, column) in batch.columns().iter().enumerate() {
                let column_name = schema.field(i).name();
                if table_cache.contains_key(column_name) {
                    continue;
                }

                // For Int32, use pinned memory
                if column.data_type() == &arrow::datatypes::DataType::Int32 {
                    let arr = column.as_any().downcast_ref::<Int32Array>().unwrap();
                    let values = &arr.values()[..];

                    let mut pinned = unsafe { self.device.alloc_pinned::<i32>(values.len()) }
                        .map_err(|e| {
                            DbxError::Gpu(format!("Failed to alloc pinned memory: {:?}", e))
                        })?;
                    // Use unsafe pointer copy as a fallback
                    unsafe {
                        let ptr = pinned.as_mut_ptr().map_err(|e| {
                            DbxError::Gpu(format!("Failed to get pinned memory pointer: {:?}", e))
                        })?;
                        std::ptr::copy_nonoverlapping(values.as_ptr(), ptr, values.len());
                    }

                    let stream = self.device.default_stream();
                    let slice = stream.clone_htod(&pinned).map_err(|e| {
                        DbxError::Gpu(format!("CUDA pinned HTOD copy failed: {:?}", e))
                    })?;

                    table_cache.insert(column_name.clone(), Arc::new(GpuData::Int32(slice)));
                } else {
                    let gpu_data = self.convert_and_upload(column)?;
                    table_cache.insert(column_name.clone(), Arc::new(gpu_data));
                }
            }
            Ok(())
        }
    }

    /// Retrieve cached GPU data for a specific column.
    #[cfg(feature = "gpu")]
    pub(super) fn get_gpu_data(&self, table: &str, column: &str) -> Option<Arc<GpuData>> {
        self.buffer_cache
            .get(table)?
            .get(column)
            .map(|v| Arc::clone(&v))
    }

    pub fn clear_table_cache(&self, table: &str) {
        #[cfg(feature = "gpu")]
        {
            self.buffer_cache.remove(table);
        }
        #[cfg(not(feature = "gpu"))]
        {
            let _ = table;
        }
    }

    pub fn clear_all_cache(&self) {
        #[cfg(feature = "gpu")]
        {
            self.buffer_cache.clear();
        }
    }
}
