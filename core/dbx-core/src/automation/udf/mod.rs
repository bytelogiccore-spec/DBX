//! UDF System
//!
//! User-Defined Functions

pub mod aggregate;
pub mod scalar;
pub mod table;

pub use aggregate::{AggregateState, AggregateUDF};
pub use scalar::ScalarUDF;
pub use table::TableUDF;
