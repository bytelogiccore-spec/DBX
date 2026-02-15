//! Database Engine Module

pub mod automation_api;
pub mod benchmark;
pub mod compaction;
pub mod constructors;
pub mod crud;
pub mod database;
pub mod database_snapshot;
pub mod delta_variant;
pub mod feature_flags;
pub mod index;
pub mod index_versioning;
pub mod metadata;
pub mod parallel_engine;
pub mod plan;
pub mod rollback;
pub mod schema_versioning;
pub mod serialization;
pub mod snapshot;
pub mod sql_interface;
pub mod stream;
pub mod transactions;
pub mod types;
pub mod udf_api;
pub mod utilities;
pub mod wos_variant;

#[cfg(test)]
mod parallel_engine_tests;

pub use benchmark::{BenchmarkResult, BenchmarkRunner};
pub use database::Database;
pub use delta_variant::DeltaVariant;
pub use feature_flags::{Feature, FeatureFlags};
pub use metadata::{FieldMetadata, IndexMetadata, SchemaMetadata};
pub use parallel_engine::{ParallelExecutionEngine, ParallelizationPolicy};
pub use rollback::{Checkpoint, RollbackManager};
pub use serialization::{SerializationRegistry, TwoLevelCache};
pub use snapshot::{DatabaseSnapshot, TableData};
pub use types::DurabilityLevel;
pub use wos_variant::WosVariant;
