//! Filter Operator — WHERE clause evaluation

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::planner::PhysicalExpr;
use arrow::array::BooleanArray;
use arrow::compute;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;

/// 필터 연산자 (WHERE 조건) — Arrow compute kernel로 predicate 평가
pub struct FilterOperator {
    input: Box<dyn PhysicalOperator>,
    predicate: PhysicalExpr,
}

impl FilterOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, predicate: PhysicalExpr) -> Self {
        Self { input, predicate }
    }
}

impl PhysicalOperator for FilterOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        loop {
            match self.input.next()? {
                None => return Ok(None),
                Some(batch) => {
                    if batch.num_rows() == 0 {
                        continue;
                    }
                    // Evaluate predicate → BooleanArray
                    let predicate_result = super::super::evaluate_expr(&self.predicate, &batch)?;
                    let mask = predicate_result
                        .as_any()
                        .downcast_ref::<BooleanArray>()
                        .ok_or_else(|| DbxError::TypeMismatch {
                            expected: "BooleanArray".to_string(),
                            actual: format!("{:?}", predicate_result.data_type()),
                        })?;

                    // Apply filter
                    let filtered = compute::filter_record_batch(&batch, mask)?;
                    if filtered.num_rows() > 0 {
                        return Ok(Some(filtered));
                    }
                    // If all rows filtered out, try next batch
                }
            }
        }
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.input.reset()
    }
}
