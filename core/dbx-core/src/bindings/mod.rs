//! Common utilities for language bindings.
//!
//! This module provides shared functionality used across all DBX bindings
//! (FFI, Node.js, Python, C#) to reduce code duplication and ensure consistency.
//!
//! # Purpose
//!
//! Language bindings often require similar patterns:
//! - Transaction buffering and batch operations
//! - Error conversion between Rust and foreign types
//! - Common data structures
//!
//! This module extracts these patterns into reusable components.

pub mod transaction;

pub use transaction::TransactionBuffer;
