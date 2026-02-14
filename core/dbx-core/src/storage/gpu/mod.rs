//! GPU Storage Layer — Optional acceleration using CUDA.
//!
//! Provides utilities to transfer Arrow RecordBatches to GPU memory
//! and execute custom kernels.
//!
//! ## Advanced Optimizations (007)
//!
//! - **CUDA Streams**: Asynchronous data transfer and kernel execution overlap
//! - **Multi-GPU**: Distributed processing with NVLink support
//! - **Histogram Optimization**: Fast aggregation for small cardinality keys
//! - **Unified Memory**: Automatic memory management with prefetching
//! - **Persistent Kernels**: Reduced kernel launch overhead

mod adaptive;
mod aggregation;
mod data;
mod group_by;
mod hash_join;
mod manager;
mod memory_pool;
mod radix_sort;
mod strategy;

// Advanced optimization modules (007)
#[cfg(feature = "gpu")]
mod memory;
#[cfg(feature = "gpu")]
mod multi_device;
#[cfg(feature = "gpu")]
mod occupancy;
#[cfg(feature = "gpu")]
mod persistent;
#[cfg(feature = "gpu")]
mod sharding;
#[cfg(feature = "gpu")]
mod topology;

// Re-exports
pub use adaptive::GpuGroupByStrategy;
#[cfg(feature = "gpu")]
pub use data::GpuData;
pub use manager::GpuManager;
#[cfg(feature = "gpu")]
pub use memory_pool::GpuMemoryPool;
pub use strategy::{GpuHashStrategy, GpuReductionStrategy};

// Advanced optimization re-exports (007)
#[cfg(feature = "gpu")]
pub use memory::{GpuMemoryManager, MemoryStrategy, UnifiedBuffer};
#[cfg(feature = "gpu")]
pub use occupancy::OccupancyCalculator;
#[cfg(feature = "gpu")]
pub use topology::DeviceTopology;
