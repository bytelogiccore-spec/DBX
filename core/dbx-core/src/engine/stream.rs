//! GPU Stream Management for asynchronous operations
//!
//! Provides CUDA Streams for overlapping data transfer and kernel execution.

#[cfg(feature = "gpu")]
use cudarc::driver::{CudaContext, CudaStream};

use crate::error::{DbxError, DbxResult};

/// Priority level for GPU streams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamPriority {
    High,
    Normal,
}

/// GPU Stream Context - manages a single CUDA stream
#[cfg(feature = "gpu")]
pub struct GpuStreamContext {
    /// Unique stream identifier
    pub stream_id: usize,
    /// Stream priority
    pub priority: StreamPriority,
    /// CUDA stream handle
    stream: Arc<CudaStream>,
    /// Parent device
    device: Arc<CudaContext>,
}

#[cfg(feature = "gpu")]
impl GpuStreamContext {
    /// Create a new stream context
    pub fn new(
        stream_id: usize,
        priority: StreamPriority,
        device: Arc<CudaContext>,
    ) -> DbxResult<Self> {
        // Note: cudarc's CudaContext doesn't support creating new streams directly
        // We use the default stream for now
        // TODO: Implement proper stream creation when cudarc supports it
        let stream = device.default_stream();

        Ok(Self {
            stream_id,
            priority,
            stream,
            device,
        })
    }

    /// Get the underlying CUDA stream
    pub fn stream(&self) -> &CudaStream {
        &self.stream
    }

    /// Synchronize this stream (wait for all operations to complete)
    pub fn synchronize(&self) -> DbxResult<()> {
        self.stream
            .synchronize()
            .map_err(|e| DbxError::Gpu(format!("Stream sync failed: {:?}", e)))
    }
}

/// Stream Manager - manages multiple CUDA streams for async operations
#[cfg(feature = "gpu")]
pub struct StreamManager {
    /// Device context
    device: Arc<CudaContext>,
    /// Active streams
    streams: Vec<GpuStreamContext>,
    /// Next stream ID
    next_id: usize,
}

#[cfg(feature = "gpu")]
impl StreamManager {
    /// Create a new stream manager
    pub fn new(device: Arc<CudaContext>) -> DbxResult<Self> {
        Ok(Self {
            device,
            streams: Vec::new(),
            next_id: 0,
        })
    }

    /// Create a new stream with the given priority
    pub fn create_stream(&mut self, priority: StreamPriority) -> DbxResult<usize> {
        let stream_id = self.next_id;
        self.next_id += 1;

        let context = GpuStreamContext::new(stream_id, priority, self.device.clone())?;
        self.streams.push(context);

        Ok(stream_id)
    }

    /// Get a stream by ID
    pub fn get_stream(&self, stream_id: usize) -> Option<&GpuStreamContext> {
        self.streams.iter().find(|s| s.stream_id == stream_id)
    }

    /// Synchronize all streams
    pub fn synchronize_all(&self) -> DbxResult<()> {
        for stream in &self.streams {
            stream.synchronize()?;
        }
        Ok(())
    }

    /// Get the number of active streams
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }
}

// Stub implementations for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct GpuStreamContext;

#[cfg(not(feature = "gpu"))]
pub struct StreamManager;

#[cfg(not(feature = "gpu"))]
impl StreamManager {
    pub fn new(_device: ()) -> DbxResult<Self> {
        Err(DbxError::NotImplemented(
            "GPU acceleration is not enabled".to_string(),
        ))
    }
}
