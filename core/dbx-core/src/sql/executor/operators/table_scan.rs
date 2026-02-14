//! TableScan Operator — Sequential RecordBatch emission

use crate::error::DbxResult;
use crate::sql::executor::operators::PhysicalOperator;
use arrow::array::ArrayRef;
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

/// 테이블 스캔 연산자 — RecordBatch 데이터를 순차적으로 반환
pub struct TableScanOperator {
    table: String,
    schema: Arc<Schema>,
    projection: Vec<usize>,
    /// Pre-loaded data batches to emit
    data: Vec<RecordBatch>,
    /// Current position in data
    position: usize,
}

impl TableScanOperator {
    pub fn new(table: String, schema: Arc<Schema>, projection: Vec<usize>) -> Self {
        Self {
            table,
            schema,
            projection,
            data: Vec::new(),
            position: 0,
        }
    }

    /// Inject data to be scanned (called by the query engine before execution).
    pub fn set_data(&mut self, batches: Vec<RecordBatch>) {
        self.data = batches;
        self.position = 0;
    }

    /// Get the table name this operator scans.
    pub fn table_name(&self) -> &str {
        &self.table
    }
}

impl PhysicalOperator for TableScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.position >= self.data.len() {
            return Ok(None);
        }

        let batch = &self.data[self.position];
        self.position += 1;

        // Apply projection if specified
        if self.projection.is_empty() {
            Ok(Some(batch.clone()))
        } else {
            let projected_columns: Vec<ArrayRef> = self
                .projection
                .iter()
                .map(|&idx| Arc::clone(batch.column(idx)))
                .collect();
            let projected_fields: Vec<Field> = self
                .projection
                .iter()
                .map(|&idx| batch.schema().field(idx).clone())
                .collect();
            let projected_schema = Arc::new(Schema::new(projected_fields));
            Ok(Some(RecordBatch::try_new(
                projected_schema,
                projected_columns,
            )?))
        }
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.position = 0;
        Ok(())
    }
}
