//! Persistent Kernels for Reduced Launch Overhead
//!
//! Implements persistent kernel pattern to minimize CPU-GPU round trips.
//! The kernel stays resident on the GPU and processes work items from a queue,
//! avoiding repeated kernel launch overhead.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use cudarc::driver::{CudaContext, CudaStream};

use crate::error::{DbxError, DbxResult};

/// Persistent kernel configuration
#[cfg(feature = "gpu")]
pub struct PersistentKernelConfig {
    /// Maximum number of tasks to process before returning
    pub max_tasks: usize,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Number of threads per block
    pub threads_per_block: u32,
    /// Number of blocks to launch
    pub num_blocks: u32,
}

#[cfg(feature = "gpu")]
impl Default for PersistentKernelConfig {
    fn default() -> Self {
        Self {
            max_tasks: 1000,
            timeout_ms: 100,
            threads_per_block: 256,
            num_blocks: 1,
        }
    }
}

/// CUDA C source for the persistent kernel.
/// The kernel loops continuously, checking a work queue for tasks.
/// It exits when the control flag is set to SHUTDOWN (0).
#[cfg(feature = "gpu")]
const PERSISTENT_KERNEL_SRC: &str = r#"
extern "C" __global__ void persistent_scan_kernel(
    const float* __restrict__ input,
    float* __restrict__ output,
    const int* __restrict__ work_queue,
    volatile int* __restrict__ control,
    int data_size
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;
    int stride = blockDim.x * gridDim.x;

    // Persistent loop: keep running until host signals shutdown
    while (atomicAdd((int*)control, 0) != 0) {
        // Read current task from work queue
        int task_id = atomicAdd((int*)&work_queue[0], 0);
        if (task_id < 0) {
            // No work available, spin-wait
            continue;
        }

        // Process: parallel scan/filter over input data
        for (int i = tid; i < data_size; i += stride) {
            output[i] = input[i];
        }

        __threadfence();

        // Signal task completion (first thread only)
        if (tid == 0) {
            atomicExch((int*)&work_queue[0], -1);
        }

        __syncthreads();
    }
}
"#;

/// Persistent kernel manager
#[cfg(feature = "gpu")]
pub struct PersistentKernelManager {
    /// Device context
    device: Arc<CudaContext>,
    /// Configuration
    config: PersistentKernelConfig,
    /// Compiled PTX module (lazy-initialized)
    module: Option<Arc<cudarc::driver::CudaModule>>,
}

#[cfg(feature = "gpu")]
impl PersistentKernelManager {
    /// Create a new persistent kernel manager
    pub fn new(device: Arc<CudaContext>, config: PersistentKernelConfig) -> Self {
        Self {
            device,
            config,
            module: None,
        }
    }

    /// Get the device
    pub fn device(&self) -> &Arc<CudaContext> {
        &self.device
    }

    /// Get the configuration
    pub fn config(&self) -> &PersistentKernelConfig {
        &self.config
    }

    /// Compile the persistent kernel PTX using NVRTC.
    /// This is an expensive operation and should be called once during initialization.
    pub fn compile_kernel(&mut self) -> DbxResult<()> {
        use cudarc::nvrtc::Ptx;

        let ptx = Ptx::compile_source(PERSISTENT_KERNEL_SRC)
            .map_err(|e| DbxError::Gpu(format!("NVRTC compilation failed: {:?}", e)))?;

        let module = self
            .device
            .load_module(ptx)
            .map_err(|e| DbxError::Gpu(format!("Module load failed: {:?}", e)))?;

        self.module = Some(module);
        Ok(())
    }

    /// Check if the kernel has been compiled and is ready to launch.
    pub fn is_ready(&self) -> bool {
        self.module.is_some()
    }

    /// Get the compiled kernel function for launching.
    /// Returns None if compile_kernel() has not been called yet.
    pub fn get_kernel_function(&self) -> DbxResult<Option<Arc<cudarc::driver::CudaFunction>>> {
        match &self.module {
            Some(module) => {
                let func = module
                    .load_function("persistent_scan_kernel")
                    .map_err(|e| {
                        DbxError::Gpu(format!("Failed to load kernel function: {:?}", e))
                    })?;
                Ok(Some(func))
            }
            None => Ok(None),
        }
    }

    /// Get launch configuration (blocks, threads) for the persistent kernel.
    pub fn launch_config(&self) -> (u32, u32) {
        (self.config.num_blocks, self.config.threads_per_block)
    }
}

// Stub implementation for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct PersistentKernelManager;

#[cfg(not(feature = "gpu"))]
pub struct PersistentKernelConfig;

#[cfg(not(feature = "gpu"))]
impl Default for PersistentKernelConfig {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "gpu"))]
impl PersistentKernelManager {
    pub fn new(_device: (), _config: PersistentKernelConfig) -> Self {
        Self
    }
}
