//! # DBX — High-Performance Embedded Database
//!
//! DBX is a high-performance embedded database built on a 5-Tier Hybrid Storage architecture.
//! Written in pure Rust, it leverages Apache Arrow and Parquet for columnar storage.
//!
//! ## Key Features
//!
//! - **5-Tier Hybrid Storage**: Delta → Cache → WOS → Index → ROS
//! - **Apache Arrow-based**: Columnar storage and vectorized operations
//! - **SQL Support**: SELECT, WHERE, JOIN, GROUP BY, ORDER BY
//! - **ACID Transactions**: Type-safe transactions using the Typestate pattern
//! - **Performance Optimized**: LRU Cache, Bloom Filter, SIMD vectorization
//!
//! ## Quick Start
//!
//! ### Basic Usage (CRUD)
//!
//! ```rust
//! use dbx_core::Database;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! // Open database
//! let db = Database::open_in_memory()?;
//!
//! // Insert data
//! db.insert("users", b"user:1", b"Alice")?;
//!
//! // Get data
//! let value = db.get("users", b"user:1")?;
//! assert_eq!(value, Some(b"Alice".to_vec()));
//!
//! // Delete data
//! db.delete("users", b"user:1")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Transactions
//!
//! ```rust
//! use dbx_core::Database;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! let db = Database::open_in_memory()?;
//!
//! // Begin transaction
//! let _tx = db.begin()?;
//!
//! // Basic CRUD uses Database directly
//! db.insert("users", b"user:2", b"Bob")?;
//!
//! // Transactions are used with Query Builder (Phase 6)
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//!
//! ### 5-Tier Hybrid Storage
//!
//! 1. **Delta Store** (DashMap) — In-memory write buffer, lock-free concurrency
//! 2. **Cache** (LRU) — Read cache for frequently accessed data
//! 3. **WOS** (sled) — Write-Optimized Store, persistent storage
//! 4. **Index** (Bloom Filter) — Fast existence checks
//! 5. **ROS** (Parquet) — Read-Optimized Store, columnar compression
//!
//! ### SQL Execution Pipeline
//!
//! ```text
//! SQL String → Parser → AST → Planner → LogicalPlan
//!          → Optimizer → PhysicalPlan → Executor → RecordBatch
//! ```
//!
//! ## Module Structure
//! - [`engine`] — Database engine ([`Database`])
//! - [`sql`] — SQL parser, planner, optimizer, executor
//! - [`storage`] — 5-Tier storage backends
//! - [`transaction`] — MVCC transaction management
//! - [`index`] — Hash Index
//! - [`wal`] — Write-Ahead Log

pub mod api;
pub mod engine;
pub mod error;
pub mod index;
pub mod sql;
pub mod storage;
pub mod transaction;
pub mod wal;

// Hash index for fast key lookups
// This comment was for `pub mod index;` but the instruction moved `index` up without its comment.
// Keeping the comment here as it was not explicitly removed.

// Write-Ahead Logging for crash recovery
// This comment was for `pub mod wal;` but the instruction moved `wal` up without its comment.
// Keeping the comment here as it was not explicitly removed.

// ===== Re-exports =====
#[cfg(feature = "simd")]
pub mod simd;

// Logging utilities
pub mod logging;

// Re-export commonly used types
pub use engine::{Database, DurabilityLevel};
pub use error::{DbxError, DbxResult};
