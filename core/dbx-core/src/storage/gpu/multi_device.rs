//! Multi-Device GPU Operations
//!
//! Coordinates parallel kernel execution across multiple GPUs.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use cudarc::driver::CudaContext;

use crate::error::{DbxError, DbxResult};

/// Multi-device coordinator for parallel GPU operations
#[cfg(feature = "gpu")]
pub struct MultiDeviceCoordinator {
    /// Device handles
    devices: Vec<Arc<CudaContext>>,
}

#[cfg(feature = "gpu")]
impl MultiDeviceCoordinator {
    /// Create a new multi-device coordinator
    pub fn new(devices: Vec<Arc<CudaContext>>) -> Self {
        Self { devices }
    }

    /// Get the number of devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Get a device handle
    pub fn device(&self, index: usize) -> Option<Arc<CudaContext>> {
        self.devices.get(index).cloned()
    }

    /// Execute a function on all devices in parallel
    pub fn parallel_execute<F, R>(&self, f: F) -> DbxResult<Vec<R>>
    where
        F: Fn(usize, Arc<CudaContext>) -> DbxResult<R> + Send + Sync + 'static,
        R: Send + 'static,
    {
        use std::thread;

        let f = Arc::new(f);
        let handles: Vec<_> = self
            .devices
            .iter()
            .enumerate()
            .map(|(idx, device)| {
                let device = Arc::clone(device);
                let f = Arc::clone(&f);
                thread::spawn(move || f(idx, device))
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| DbxError::Gpu("Thread join failed".to_string()))??;
            results.push(result);
        }

        Ok(results)
    }

    /// Synchronize all devices
    pub fn synchronize_all(&self) -> DbxResult<()> {
        for device in &self.devices {
            device
                .synchronize()
                .map_err(|e| DbxError::Gpu(format!("Device sync failed: {:?}", e)))?;
        }
        Ok(())
    }
}

// Stub implementation for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct MultiDeviceCoordinator;

#[cfg(not(feature = "gpu"))]
impl MultiDeviceCoordinator {
    pub fn new(_devices: Vec<()>) -> Self {
        Self
    }
}
