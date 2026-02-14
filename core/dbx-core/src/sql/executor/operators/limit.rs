//! Limit Operator — LIMIT/OFFSET clause handling

use crate::error::DbxResult;
use crate::sql::executor::operators::PhysicalOperator;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;

/// Limit 연산자 (LIMIT/OFFSET)
pub struct LimitOperator {
    input: Box<dyn PhysicalOperator>,
    count: usize,
    offset: usize,
    /// Total rows emitted so far
    emitted: usize,
    /// Total rows skipped so far (for offset)
    skipped: usize,
}

impl LimitOperator {
    pub fn new(input: Box<dyn PhysicalOperator>, count: usize, offset: usize) -> Self {
        Self {
            input,
            count,
            offset,
            emitted: 0,
            skipped: 0,
        }
    }
}

impl PhysicalOperator for LimitOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        // Already reached the limit
        if self.emitted >= self.count {
            return Ok(None);
        }

        loop {
            match self.input.next()? {
                None => return Ok(None),
                Some(batch) => {
                    let batch_rows = batch.num_rows();

                    // Handle offset: skip rows
                    if self.skipped < self.offset {
                        let remaining_to_skip = self.offset - self.skipped;
                        if batch_rows <= remaining_to_skip {
                            // Skip entire batch
                            self.skipped += batch_rows;
                            continue;
                        } else {
                            // Skip partial batch
                            self.skipped = self.offset;
                            let remaining_rows = batch_rows - remaining_to_skip;
                            let remaining_to_emit = self.count - self.emitted;
                            let take = remaining_rows.min(remaining_to_emit);
                            self.emitted += take;
                            return Ok(Some(batch.slice(remaining_to_skip, take)));
                        }
                    }

                    // Apply count limit
                    let remaining = self.count - self.emitted;
                    if batch_rows <= remaining {
                        self.emitted += batch_rows;
                        return Ok(Some(batch));
                    } else {
                        self.emitted += remaining;
                        return Ok(Some(batch.slice(0, remaining)));
                    }
                }
            }
        }
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.emitted = 0;
        self.skipped = 0;
        self.input.reset()
    }
}
