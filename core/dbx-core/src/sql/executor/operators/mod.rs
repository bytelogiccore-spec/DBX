//! Physical Operators Module

mod filter;
mod hash_aggregate;
mod join;
mod limit;
mod physical_operator;
mod projection;
mod sort;
mod table_scan;

pub use filter::FilterOperator;
pub use hash_aggregate::HashAggregateOperator;
pub use join::HashJoinOperator;
pub use limit::LimitOperator;
pub use physical_operator::PhysicalOperator;
pub use projection::ProjectionOperator;
pub use sort::SortOperator;
pub use table_scan::TableScanOperator;
