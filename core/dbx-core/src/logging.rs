//! Logging utilities for DBX
//!
//! Provides helpers for initializing tracing subscribers.

#[cfg(feature = "logging")]
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize logging with default settings
///
/// # Environment Variables
/// - `RUST_LOG` - Log level filter (default: "info")
///
/// # Example
/// ```rust
/// dbx_core::logging::init();
/// ```
#[cfg(feature = "logging")]
pub fn init() {
    init_with_level("info")
}

/// Initialize logging with a specific level
///
/// # Arguments
/// * `level` - Log level (trace, debug, info, warn, error)
///
/// # Example
/// ```rust
/// dbx_core::logging::init_with_level("debug");
/// ```
#[cfg(feature = "logging")]
pub fn init_with_level(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();
}

/// Initialize logging for tests
///
/// Uses a more verbose format suitable for debugging tests.
#[cfg(feature = "logging")]
pub fn init_test() {
    let _ = fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .with_test_writer()
        .try_init();
}

// Stub implementations when logging feature is disabled
#[cfg(not(feature = "logging"))]
pub fn init() {}

#[cfg(not(feature = "logging"))]
pub fn init_with_level(_level: &str) {}

#[cfg(not(feature = "logging"))]
pub fn init_test() {}
