//! GPU Memory Management with Unified Memory support
//!
//! Provides both traditional GPU memory pooling and CUDA Unified Memory
//! for automatic data migration between CPU and GPU.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use cudarc::driver::{CudaContext, CudaSlice, CudaStream};

use crate::error::{DbxError, DbxResult};

/// Unified Memory Buffer - automatically migrated between CPU and GPU
#[cfg(feature = "gpu")]
pub struct UnifiedBuffer<T: Clone> {
    /// Device context
    device: Arc<CudaContext>,
    /// Data pointer (managed memory)
    data: Vec<T>,
    /// Size in elements
    size: usize,
    /// Whether data has been prefetched to GPU
    prefetched: bool,
}

#[cfg(feature = "gpu")]
impl<T: Clone + cudarc::driver::DeviceRepr> UnifiedBuffer<T> {
    /// Create a new unified buffer
    pub fn new(device: Arc<CudaContext>, size: usize) -> DbxResult<Self> {
        // cudarc 0.19.2 does not expose cudaMallocManaged.
        // Host memory is used as a portable fallback with explicit htod transfers.
        // When cudarc adds Unified Memory support, replace with managed allocation.
        let data = vec![unsafe { std::mem::zeroed() }; size];

        Ok(Self {
            device,
            data,
            size,
            prefetched: false,
        })
    }

    /// Create from existing data
    pub fn from_vec(device: Arc<CudaContext>, data: Vec<T>) -> DbxResult<Self> {
        let size = data.len();
        Ok(Self {
            device,
            data,
            size,
            prefetched: false,
        })
    }

    /// Prefetch data to GPU asynchronously
    pub fn prefetch_to_gpu(&mut self) -> DbxResult<()> {
        // cudarc 0.19.2 does not expose cudaMemPrefetchAsync.
        // Simulated via explicit htod upload on first prefetch call.
        // Replace with native prefetch when cudarc adds UVM support.

        if !self.prefetched {
            // Upload data to GPU (simulating prefetch)
            let stream = self.device.default_stream();
            let _gpu_slice = stream
                .clone_htod(&self.data)
                .map_err(|e| DbxError::Gpu(format!("Prefetch failed: {:?}", e)))?;

            self.prefetched = true;
        }

        Ok(())
    }

    /// Prefetch data to CPU asynchronously
    pub fn prefetch_to_cpu(&mut self) -> DbxResult<()> {
        // Note: In true Unified Memory, this would hint the driver
        // to migrate pages to CPU
        // For now, this is a no-op since we're using host memory
        self.prefetched = false;
        Ok(())
    }

    /// Get data as slice (CPU-accessible)
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Get mutable data as slice (CPU-accessible)
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.prefetched = false; // Mark as potentially modified
        &mut self.data
    }

    /// Upload to GPU and get CudaSlice
    pub fn to_device(&self) -> DbxResult<CudaSlice<T>> {
        let stream = self.device.default_stream();
        stream
            .clone_htod(&self.data)
            .map_err(|e| DbxError::Gpu(format!("Upload failed: {:?}", e)))
    }

    /// Get size in elements
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

/// Memory allocation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStrategy {
    /// Traditional GPU memory (explicit transfers)
    DeviceOnly,
    /// Unified Memory (automatic migration)
    Unified,
    /// Pinned host memory (faster transfers)
    Pinned,
}

/// Memory manager for GPU operations
#[cfg(feature = "gpu")]
pub struct GpuMemoryManager {
    /// Device context
    device: Arc<CudaContext>,
    /// Default memory strategy
    strategy: MemoryStrategy,
}

#[cfg(feature = "gpu")]
impl GpuMemoryManager {
    /// Create a new memory manager
    pub fn new(device: Arc<CudaContext>, strategy: MemoryStrategy) -> Self {
        Self { device, strategy }
    }

    /// Allocate buffer with default strategy
    pub fn alloc<T: Clone + cudarc::driver::DeviceRepr>(
        &self,
        size: usize,
    ) -> DbxResult<UnifiedBuffer<T>> {
        match self.strategy {
            MemoryStrategy::Unified => UnifiedBuffer::new(self.device.clone(), size),
            MemoryStrategy::DeviceOnly => {
                // For device-only, we still use UnifiedBuffer but don't prefetch
                UnifiedBuffer::new(self.device.clone(), size)
            }
            MemoryStrategy::Pinned => {
                // Pinned memory would use cudaMallocHost
                // For now, fall back to regular allocation
                UnifiedBuffer::new(self.device.clone(), size)
            }
        }
    }

    /// Allocate buffer from existing data
    pub fn alloc_from<T: Clone + cudarc::driver::DeviceRepr>(
        &self,
        data: Vec<T>,
    ) -> DbxResult<UnifiedBuffer<T>> {
        UnifiedBuffer::from_vec(self.device.clone(), data)
    }

    /// Get current strategy
    pub fn strategy(&self) -> MemoryStrategy {
        self.strategy
    }

    /// Set memory strategy
    pub fn set_strategy(&mut self, strategy: MemoryStrategy) {
        self.strategy = strategy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "gpu")]
    fn test_unified_buffer_creation() {
        // This test requires CUDA runtime
        // Skip if not available
        if let Ok(device) = CudaContext::new(0) {
            let device = Arc::new(device);
            let buffer: UnifiedBuffer<i32> = UnifiedBuffer::new(device, 1000).unwrap();
            assert_eq!(buffer.len(), 1000);
            assert!(!buffer.is_empty());
        }
    }

    #[test]
    #[cfg(feature = "gpu")]
    fn test_unified_buffer_from_vec() {
        if let Ok(device) = CudaContext::new(0) {
            let device = Arc::new(device);
            let data = vec![1, 2, 3, 4, 5];
            let buffer = UnifiedBuffer::from_vec(device, data.clone()).unwrap();
            assert_eq!(buffer.as_slice(), &data[..]);
        }
    }
}
