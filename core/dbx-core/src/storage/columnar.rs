//! Columnar Store — Arrow RecordBatch management.
//!
//! Manages in-memory columnar data using Apache Arrow's `RecordBatch` format.
//! Provides schema enforcement and row→column conversion.

use crate::error::{DbxError, DbxResult};
use arrow::array::{
    ArrayRef, BooleanBuilder, Float64Builder, Int32Builder, Int64Builder, StringBuilder,
};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use rayon::prelude::*;
use std::sync::Arc;

/// Represents a scalar value that can be stored in a column.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Null,
    Int32(i32),
    Int64(i64),
    Float64(f64),
    Utf8(String),
    Boolean(bool),
}

impl ScalarValue {
    /// Get the Arrow DataType for this value.
    pub fn data_type(&self) -> DataType {
        match self {
            ScalarValue::Null => DataType::Null,
            ScalarValue::Int32(_) => DataType::Int32,
            ScalarValue::Int64(_) => DataType::Int64,
            ScalarValue::Float64(_) => DataType::Float64,
            ScalarValue::Utf8(_) => DataType::Utf8,
            ScalarValue::Boolean(_) => DataType::Boolean,
        }
    }

    /// Extract a value from an Arrow array at the given index.
    pub fn from_array(array: &ArrayRef, idx: usize) -> crate::error::DbxResult<Self> {
        use arrow::array::AsArray;
        if array.is_null(idx) {
            return Ok(ScalarValue::Null);
        }
        match array.data_type() {
            DataType::Int32 => Ok(ScalarValue::Int32(
                array
                    .as_primitive::<arrow::datatypes::Int32Type>()
                    .value(idx),
            )),
            DataType::Int64 => Ok(ScalarValue::Int64(
                array
                    .as_primitive::<arrow::datatypes::Int64Type>()
                    .value(idx),
            )),
            DataType::Float64 => Ok(ScalarValue::Float64(
                array
                    .as_primitive::<arrow::datatypes::Float64Type>()
                    .value(idx),
            )),
            DataType::Boolean => Ok(ScalarValue::Boolean(array.as_boolean().value(idx))),
            DataType::Utf8 => Ok(ScalarValue::Utf8(
                array.as_string::<i32>().value(idx).to_string(),
            )),
            dt => Err(crate::error::DbxError::TypeMismatch {
                expected: "Int32|Int64|Float64|Boolean|Utf8".to_string(),
                actual: format!("{dt:?}"),
            }),
        }
    }
}

/// In-memory columnar store backed by Arrow RecordBatch.
///
/// Accumulates rows and converts them to columnar format on demand.
pub struct ColumnarStore {
    schema: Arc<Schema>,
    rows: Vec<Vec<ScalarValue>>,
}

impl ColumnarStore {
    /// Create a new ColumnarStore with the given schema.
    pub fn new(schema: Arc<Schema>) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    /// Append a row of values. Must match the schema's field count and types.
    pub fn append_row(&mut self, values: &[ScalarValue]) -> DbxResult<()> {
        let field_count = self.schema.fields().len();
        if values.len() != field_count {
            return Err(DbxError::Schema(format!(
                "expected {field_count} columns, got {}",
                values.len()
            )));
        }

        // Type check each value against schema
        for (i, (value, field)) in values.iter().zip(self.schema.fields()).enumerate() {
            if !matches!(value, ScalarValue::Null) {
                let expected = field.data_type();
                let actual = value.data_type();
                if *expected != actual {
                    return Err(DbxError::TypeMismatch {
                        expected: format!("column {i} ({}): {:?}", field.name(), expected),
                        actual: format!("{actual:?}"),
                    });
                }
            }
        }

        self.rows.push(values.to_vec());
        Ok(())
    }

    /// Convert accumulated rows into an Arrow RecordBatch (Parallelized).
    pub fn to_record_batch(&self) -> DbxResult<RecordBatch> {
        if self.rows.is_empty() {
            return Ok(RecordBatch::new_empty(Arc::clone(&self.schema)));
        }

        // Use Rayon for parallel column building
        let columns: Vec<ArrayRef> = self
            .schema
            .fields()
            .par_iter()
            .enumerate()
            .map(|(col_idx, field)| self.build_column(col_idx, field.data_type()))
            .collect::<DbxResult<_>>()?;

        Ok(RecordBatch::try_new(Arc::clone(&self.schema), columns)?)
    }

    /// Get the schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Get the number of accumulated rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Clear all accumulated rows.
    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Build a single column array from row data.
    fn build_column(&self, col_idx: usize, data_type: &DataType) -> DbxResult<ArrayRef> {
        match data_type {
            DataType::Int32 => {
                let mut builder = Int32Builder::with_capacity(self.rows.len());
                for row in &self.rows {
                    match &row[col_idx] {
                        ScalarValue::Int32(v) => builder.append_value(*v),
                        ScalarValue::Null => builder.append_null(),
                        other => {
                            return Err(DbxError::TypeMismatch {
                                expected: "Int32".to_string(),
                                actual: format!("{other:?}"),
                            });
                        }
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Int64 => {
                let mut builder = Int64Builder::with_capacity(self.rows.len());
                for row in &self.rows {
                    match &row[col_idx] {
                        ScalarValue::Int64(v) => builder.append_value(*v),
                        ScalarValue::Null => builder.append_null(),
                        other => {
                            return Err(DbxError::TypeMismatch {
                                expected: "Int64".to_string(),
                                actual: format!("{other:?}"),
                            });
                        }
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Float64 => {
                let mut builder = Float64Builder::with_capacity(self.rows.len());
                for row in &self.rows {
                    match &row[col_idx] {
                        ScalarValue::Float64(v) => builder.append_value(*v),
                        ScalarValue::Null => builder.append_null(),
                        other => {
                            return Err(DbxError::TypeMismatch {
                                expected: "Float64".to_string(),
                                actual: format!("{other:?}"),
                            });
                        }
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Utf8 => {
                let mut builder = StringBuilder::with_capacity(self.rows.len(), 256);
                for row in &self.rows {
                    match &row[col_idx] {
                        ScalarValue::Utf8(v) => builder.append_value(v),
                        ScalarValue::Null => builder.append_null(),
                        other => {
                            return Err(DbxError::TypeMismatch {
                                expected: "Utf8".to_string(),
                                actual: format!("{other:?}"),
                            });
                        }
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            DataType::Boolean => {
                let mut builder = BooleanBuilder::with_capacity(self.rows.len());
                for row in &self.rows {
                    match &row[col_idx] {
                        ScalarValue::Boolean(v) => builder.append_value(*v),
                        ScalarValue::Null => builder.append_null(),
                        other => {
                            return Err(DbxError::TypeMismatch {
                                expected: "Boolean".to_string(),
                                actual: format!("{other:?}"),
                            });
                        }
                    }
                }
                Ok(Arc::new(builder.finish()))
            }
            dt => Err(DbxError::Schema(format!("unsupported data type: {dt:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Array, BooleanArray, Float64Array, Int32Array, Int64Array, StringArray};
    use arrow::datatypes::Field;

    fn test_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("age", DataType::Int64, true),
            Field::new("score", DataType::Float64, true),
            Field::new("active", DataType::Boolean, false),
        ]))
    }

    #[test]
    fn create_empty_store() {
        let store = ColumnarStore::new(test_schema());
        assert_eq!(store.row_count(), 0);
        let batch = store.to_record_batch().unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 5);
    }

    #[test]
    fn append_and_convert() {
        let mut store = ColumnarStore::new(test_schema());
        store
            .append_row(&[
                ScalarValue::Int32(1),
                ScalarValue::Utf8("Alice".to_string()),
                ScalarValue::Int64(30),
                ScalarValue::Float64(95.5),
                ScalarValue::Boolean(true),
            ])
            .unwrap();
        store
            .append_row(&[
                ScalarValue::Int32(2),
                ScalarValue::Utf8("Bob".to_string()),
                ScalarValue::Int64(25),
                ScalarValue::Float64(87.3),
                ScalarValue::Boolean(false),
            ])
            .unwrap();

        assert_eq!(store.row_count(), 2);
        let batch = store.to_record_batch().unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 5);

        // Verify column data
        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.value(0), 1);
        assert_eq!(ids.value(1), 2);

        let names = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(names.value(0), "Alice");
        assert_eq!(names.value(1), "Bob");

        let ages = batch
            .column(2)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert_eq!(ages.value(0), 30);
        assert_eq!(ages.value(1), 25);

        let scores = batch
            .column(3)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert!((scores.value(0) - 95.5).abs() < f64::EPSILON);

        let active = batch
            .column(4)
            .as_any()
            .downcast_ref::<BooleanArray>()
            .unwrap();
        assert!(active.value(0));
        assert!(!active.value(1));
    }

    #[test]
    fn null_handling() {
        let mut store = ColumnarStore::new(test_schema());
        store
            .append_row(&[
                ScalarValue::Int32(1),
                ScalarValue::Utf8("Alice".to_string()),
                ScalarValue::Null, // nullable age
                ScalarValue::Null, // nullable score
                ScalarValue::Boolean(true),
            ])
            .unwrap();

        let batch = store.to_record_batch().unwrap();
        let ages = batch
            .column(2)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        assert!(ages.is_null(0));

        let scores = batch
            .column(3)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert!(scores.is_null(0));
    }

    #[test]
    fn wrong_column_count_rejected() {
        let mut store = ColumnarStore::new(test_schema());
        let result = store.append_row(&[ScalarValue::Int32(1), ScalarValue::Utf8("x".into())]);
        assert!(result.is_err());
    }

    #[test]
    fn type_mismatch_rejected() {
        let mut store = ColumnarStore::new(test_schema());
        let result = store.append_row(&[
            ScalarValue::Utf8("wrong".into()), // should be Int32
            ScalarValue::Utf8("name".into()),
            ScalarValue::Int64(0),
            ScalarValue::Float64(0.0),
            ScalarValue::Boolean(false),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn clear_rows() {
        let mut store = ColumnarStore::new(test_schema());
        store
            .append_row(&[
                ScalarValue::Int32(1),
                ScalarValue::Utf8("x".into()),
                ScalarValue::Int64(0),
                ScalarValue::Float64(0.0),
                ScalarValue::Boolean(false),
            ])
            .unwrap();
        assert_eq!(store.row_count(), 1);
        store.clear();
        assert_eq!(store.row_count(), 0);
    }

    #[test]
    fn schema_accessible() {
        let schema = test_schema();
        let store = ColumnarStore::new(Arc::clone(&schema));
        assert_eq!(store.schema().fields().len(), 5);
        assert_eq!(store.schema().field(0).name(), "id");
    }

    #[test]
    fn round_trip_1000_rows() {
        let mut store = ColumnarStore::new(test_schema());
        for i in 0..1000 {
            store
                .append_row(&[
                    ScalarValue::Int32(i),
                    ScalarValue::Utf8(format!("user_{i}")),
                    ScalarValue::Int64(i as i64 * 2),
                    ScalarValue::Float64(i as f64 * 1.5),
                    ScalarValue::Boolean(i % 2 == 0),
                ])
                .unwrap();
        }

        let batch = store.to_record_batch().unwrap();
        assert_eq!(batch.num_rows(), 1000);

        let ids = batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.value(0), 0);
        assert_eq!(ids.value(999), 999);

        let names = batch
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(names.value(0), "user_0");
        assert_eq!(names.value(999), "user_999");
    }
}
