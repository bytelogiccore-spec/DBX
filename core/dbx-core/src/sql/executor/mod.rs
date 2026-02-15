//! SQL Query Executor Module

use crate::error::DbxResult;
use arrow::array::RecordBatch;
use arrow::compute;
use arrow::datatypes::Schema;
use std::sync::Arc;

pub mod expr;
pub mod operators;
pub mod parallel_query;

pub use expr::evaluate_expr;
pub use operators::{
    FilterOperator, HashAggregateOperator, HashJoinOperator, LimitOperator, PhysicalOperator,
    ProjectionOperator, SortOperator, TableScanOperator,
};
pub use parallel_query::{ParallelQueryExecutor, AggregateType, AggregateResult};

// Helper function for concatenating RecordBatches
pub fn concat_batches(schema: &Arc<Schema>, batches: &[RecordBatch]) -> DbxResult<RecordBatch> {
    if batches.len() == 1 {
        return Ok(batches[0].clone());
    }
    Ok(compute::concat_batches(schema, batches)?)
}
