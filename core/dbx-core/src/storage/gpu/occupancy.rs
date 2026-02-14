//! GPU Occupancy Calculator
//!
//! Calculates optimal block size based on register usage and shared memory.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use cudarc::driver::CudaContext;

use crate::error::{DbxError, DbxResult};

/// Occupancy calculation parameters
#[derive(Debug, Clone)]
pub struct OccupancyParams {
    /// Registers per thread
    pub registers_per_thread: usize,
    /// Shared memory per block (bytes)
    pub shared_mem_per_block: usize,
    /// Threads per block
    pub threads_per_block: usize,
}

/// Occupancy calculator
#[cfg(feature = "gpu")]
pub struct OccupancyCalculator {
    /// Device context
    device: Arc<CudaContext>,
    /// Device properties (cached)
    max_threads_per_block: usize,
    max_shared_mem_per_block: usize,
    max_registers_per_block: usize,
}

#[cfg(feature = "gpu")]
impl OccupancyCalculator {
    /// Create a new occupancy calculator
    pub fn new(device: Arc<CudaContext>) -> DbxResult<Self> {
        // Get device properties
        // Note: cudarc doesn't expose all properties directly
        // Using conservative defaults for now
        let max_threads_per_block = 1024; // Common for modern GPUs
        let max_shared_mem_per_block = 48 * 1024; // 48KB for compute capability 7.x+
        let max_registers_per_block = 65536; // 64K registers per SM

        Ok(Self {
            device,
            max_threads_per_block,
            max_shared_mem_per_block,
            max_registers_per_block,
        })
    }

    /// Calculate optimal block size for given parameters
    pub fn calculate_optimal_block_size(&self, params: &OccupancyParams) -> DbxResult<usize> {
        // Start with maximum threads per block
        let mut block_size = self.max_threads_per_block;

        // Constrain by shared memory
        if params.shared_mem_per_block > 0 {
            let max_blocks_by_shmem =
                self.max_shared_mem_per_block / params.shared_mem_per_block;
            let max_threads_by_shmem = max_blocks_by_shmem * params.threads_per_block;
            block_size = block_size.min(max_threads_by_shmem);
        }

        // Constrain by registers
        if params.registers_per_thread > 0 {
            let max_threads_by_regs =
                self.max_registers_per_block / params.registers_per_thread;
            block_size = block_size.min(max_threads_by_regs);
        }

        // Round down to nearest multiple of warp size (32)
        block_size = (block_size / 32) * 32;

        // Ensure at least one warp
        if block_size < 32 {
            return Err(DbxError::Gpu(
                "Insufficient resources for kernel execution".to_string(),
            ));
        }

        Ok(block_size)
    }

    /// Calculate occupancy percentage
    pub fn calculate_occupancy(&self, params: &OccupancyParams) -> DbxResult<f64> {
        let optimal_block_size = self.calculate_optimal_block_size(params)?;
        let occupancy = optimal_block_size as f64 / self.max_threads_per_block as f64;
        Ok(occupancy * 100.0)
    }
}

// Stub implementation for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct OccupancyCalculator;

#[cfg(not(feature = "gpu"))]
pub struct OccupancyParams {
    pub registers_per_thread: usize,
    pub shared_mem_per_block: usize,
    pub threads_per_block: usize,
}

#[cfg(not(feature = "gpu"))]
impl OccupancyCalculator {
    pub fn new(_device: ()) -> DbxResult<Self> {
        Err(DbxError::NotImplemented(
            "GPU acceleration is not enabled".to_string(),
        ))
    }
}
