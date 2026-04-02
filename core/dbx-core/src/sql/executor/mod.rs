//! SQL Query Executor Module

use crate::error::DbxResult;
use arrow::array::RecordBatch;
use arrow::compute;
use arrow::datatypes::Schema;
use std::sync::Arc;

pub mod distributed_executor;
pub mod expr;
pub mod fragment_splitter;
pub mod local_executor;
pub mod operators;
pub mod parallel_query;

pub use distributed_executor::DistributedExecutor;
pub use expr::evaluate_expr;
pub use fragment_splitter::{FragmentDAG, FragmentSplitter, FragmentStage};
pub use local_executor::LocalExecutor;
pub use operators::{
    FilterOperator, GridExchangeOperator, HashAggregateOperator, HashJoinOperator, LimitOperator,
    PhysicalOperator, ProjectionOperator, SortOperator, TableScanOperator,
};
pub use parallel_query::{AggregateResult, AggregateType, ParallelQueryExecutor};

// Helper function for concatenating RecordBatches
pub fn concat_batches(schema: &Arc<Schema>, batches: &[RecordBatch]) -> DbxResult<RecordBatch> {
    if batches.len() == 1 {
        return Ok(batches[0].clone());
    }
    Ok(compute::concat_batches(schema, batches)?)
}
