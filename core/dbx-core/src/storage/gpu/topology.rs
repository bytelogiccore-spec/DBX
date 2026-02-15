//! GPU Device Topology Detection and Management
//!
//! Detects available GPUs, P2P capabilities, and NVLink connections.

use std::sync::Arc;

#[cfg(feature = "gpu")]
use cudarc::driver::CudaContext;

use crate::error::{DbxError, DbxResult};

/// GPU connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    /// PCIe connection
    PCIe,
    /// NVLink high-speed connection
    NVLink,
    /// No direct connection
    None,
}

/// Device Topology - describes GPU interconnection
#[cfg(feature = "gpu")]
pub struct DeviceTopology {
    /// Number of available GPUs
    device_count: usize,
    /// P2P access matrix (device_i can access device_j)
    p2p_matrix: Vec<Vec<bool>>,
    /// Link type matrix
    link_types: Vec<Vec<LinkType>>,
    /// Device handles
    devices: Vec<Arc<CudaContext>>,
}

#[cfg(feature = "gpu")]
impl DeviceTopology {
    /// Detect system GPU topology
    pub fn detect() -> DbxResult<Self> {
        // Get device count
        let device_count = CudaContext::device_count()
            .map_err(|e| DbxError::Gpu(format!("Failed to get device count: {:?}", e)))?;

        if device_count == 0 {
            return Err(DbxError::Gpu("No CUDA devices found".to_string()));
        }

        // Initialize devices
        let mut devices = Vec::new();
        for i in 0..device_count {
            let device = CudaContext::new(i as usize).map_err(|e| {
                DbxError::Gpu(format!("Failed to initialize device {}: {:?}", i, e))
            })?;
            devices.push(device);
        }

        // Build P2P matrix
        let device_count_usize = device_count as usize;
        let mut p2p_matrix = vec![vec![false; device_count_usize]; device_count_usize];
        let mut link_types = vec![vec![LinkType::None; device_count_usize]; device_count_usize];

        for i in 0..device_count_usize {
            for j in 0..device_count_usize {
                if i == j {
                    p2p_matrix[i][j] = true;
                    link_types[i][j] = LinkType::NVLink; // Self-access
                    continue;
                }

                // cudarc 0.19.2 does not expose cudaDeviceCanAccessPeer.
                // Default to PCIe (no P2P) until cudarc adds peer access API.
                p2p_matrix[i][j] = false;
                link_types[i][j] = LinkType::PCIe;
            }
        }

        Ok(Self {
            device_count: device_count_usize,
            p2p_matrix,
            link_types,
            devices,
        })
    }

    /// Get the number of devices
    pub fn device_count(&self) -> usize {
        self.device_count
    }

    /// Check if device i can access device j via P2P
    pub fn can_access_peer(&self, i: usize, j: usize) -> bool {
        if i >= self.device_count || j >= self.device_count {
            return false;
        }
        self.p2p_matrix[i][j]
    }

    /// Get link type between devices
    pub fn link_type(&self, i: usize, j: usize) -> LinkType {
        if i >= self.device_count || j >= self.device_count {
            return LinkType::None;
        }
        self.link_types[i][j]
    }

    /// Get device handle
    pub fn device(&self, i: usize) -> Option<Arc<CudaContext>> {
        self.devices.get(i).cloned()
    }

    /// Enable P2P access between devices
    pub fn enable_peer_access(&self, i: usize, j: usize) -> DbxResult<()> {
        if i >= self.device_count || j >= self.device_count {
            return Err(DbxError::Gpu(format!(
                "Invalid device indices: {} and {}",
                i, j
            )));
        }

        if i == j {
            return Ok(()); // No need to enable self-access
        }

        if !self.p2p_matrix[i][j] {
            return Err(DbxError::Gpu(format!(
                "P2P access not supported between devices {} and {}",
                i, j
            )));
        }

        // cudarc 0.19.2 does not expose cuCtxEnablePeerAccess.
        // P2P will be enabled when cudarc adds peer access support.
        Err(DbxError::NotImplemented(
            "P2P access requires cudarc peer access API (not yet available)".to_string(),
        ))
    }

    /// Check if NVLink is available between any devices
    pub fn has_nvlink(&self) -> bool {
        for i in 0..self.device_count {
            for j in 0..self.device_count {
                if i != j && self.link_types[i][j] == LinkType::NVLink {
                    return true;
                }
            }
        }
        false
    }
}

// Stub implementation for non-GPU builds
#[cfg(not(feature = "gpu"))]
pub struct DeviceTopology;

#[cfg(not(feature = "gpu"))]
impl DeviceTopology {
    pub fn detect() -> DbxResult<Self> {
        Err(DbxError::NotImplemented(
            "GPU acceleration is not enabled".to_string(),
        ))
    }
}
