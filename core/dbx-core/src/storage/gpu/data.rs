//! GPU memory data types.

#[cfg(feature = "gpu")]
use cudarc::driver::{CudaSlice, PinnedHostSlice};

/// Represents a handle to memory on the GPU for a specific type.
#[cfg(feature = "gpu")]
pub enum GpuData {
    Int32(CudaSlice<i32>),
    Int64(CudaSlice<i64>),
    Float64(CudaSlice<f64>),
    /// Raw bytes, used for unsupported or generic data.
    Raw(CudaSlice<u8>),
    /// Host pinned memory for fast DMA transfers
    PinnedInt32(std::sync::Arc<PinnedHostSlice<i32>>),
}

#[cfg(feature = "gpu")]
impl GpuData {
    pub fn len(&self) -> usize {
        match self {
            GpuData::Int32(s) => s.len(),
            GpuData::Int64(s) => s.len(),
            GpuData::Float64(s) => s.len(),
            GpuData::Raw(s) => s.len(),
            GpuData::PinnedInt32(v) => v.len(),
        }
    }
}
