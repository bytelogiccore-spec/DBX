//! Sort Operator — ORDER BY clause handling

use crate::error::DbxResult;
use crate::sql::executor::operators::PhysicalOperator;
use arrow::array::ArrayRef;
use arrow::compute::{self, SortColumn, SortOptions};
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use rayon::prelude::*;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

/// Sort 연산자 (ORDER BY) — Arrow sort_to_indices 활용
pub struct SortOperator {
    input: Box<dyn PhysicalOperator>,
    /// (column_index, ascending)
    order_by: Vec<(usize, bool)>,
    /// Materialized sorted result (sort requires all data)
    sorted: Option<RecordBatch>,
    emitted: bool,
}

impl SortOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, order_by: Vec<(usize, bool)>) -> Self {
        Self {
            input,
            order_by,
            sorted: None,
            emitted: false,
        }
    }

    /// Materialize all input batches into one sorted RecordBatch.
    fn materialize(&mut self) -> DbxResult<()> {
        // Collect all batches
        let mut batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        while let Some(batch) = self.input.next()? {
            if batch.num_rows() > 0 {
                batches.push(batch);
            }
        }

        if batches.is_empty() {
            self.sorted = None;
            return Ok(());
        }

        // Concatenate all batches into one
        let schema = batches[0].schema();
        let merged = super::super::concat_batches(&schema, batches.as_slice())?;

        // Build sort columns
        let sort_columns: Vec<SortColumn> = self
            .order_by
            .iter()
            .map(|(col_idx, asc)| SortColumn {
                values: Arc::clone(merged.column(*col_idx)),
                options: Some(SortOptions {
                    descending: !asc,
                    nulls_first: true,
                }),
            })
            .collect();

        // Sort
        let indices = compute::lexsort_to_indices(&sort_columns, None)?;
        let sorted_columns: Vec<ArrayRef> = merged
            .columns()
            .par_iter()
            .map(|col| compute::take(col.as_ref(), &indices, None))
            .collect::<Result<_, _>>()?;

        self.sorted = Some(RecordBatch::try_new(schema, sorted_columns)?);
        Ok(())
    }
}

impl PhysicalOperator for SortOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.sorted.is_none() && !self.emitted {
            self.materialize()?;
        }

        if self.emitted {
            return Ok(None);
        }

        self.emitted = true;
        Ok(self.sorted.take())
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.sorted = None;
        self.emitted = false;
        self.input.reset()
    }
}
