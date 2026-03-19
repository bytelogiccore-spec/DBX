//! UDF System
//!
//! User-Defined Functions

pub mod aggregate;
pub mod metadata;
pub mod scalar;
pub mod table;
pub mod vectorized;

pub use aggregate::{AggregateState, AggregateUDF};
pub use metadata::{UdfMetadata, UdfType};
pub use scalar::ScalarUDF;
pub use table::TableUDF;
pub use vectorized::VectorizedUDF;
