//! Integration layer between ColumnarDelta and SQL engine.
//!
//! Provides utilities for converting between key-value pairs and RecordBatches,
//! enabling ColumnarDelta to work with the existing Database API.

use crate::error::{DbxError, DbxResult};
use arrow::array::{ArrayRef, BinaryArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

/// Standard schema for key-value RecordBatches.
///
/// All key-value data stored in ColumnarDelta uses this schema:
/// - `key`: Binary (variable-length byte array)
/// - `value`: Binary (variable-length byte array)
pub fn kv_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("key", DataType::Binary, false),
        Field::new("value", DataType::Binary, false),
    ]))
}

/// Convert key-value pairs to a RecordBatch.
///
/// This is used when inserting data into ColumnarDelta from the Database API.
pub fn kv_to_batch(rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<RecordBatch> {
    if rows.is_empty() {
        return Ok(RecordBatch::new_empty(kv_schema()));
    }

    let keys: Vec<&[u8]> = rows.iter().map(|(k, _)| k.as_slice()).collect();
    let values: Vec<&[u8]> = rows.iter().map(|(_, v)| v.as_slice()).collect();

    let key_array = BinaryArray::from(keys);
    let value_array = BinaryArray::from(values);

    let batch = RecordBatch::try_new(
        kv_schema(),
        vec![Arc::new(key_array), Arc::new(value_array)],
    )?;

    Ok(batch)
}

/// Convert a RecordBatch to key-value pairs.
///
/// This is used when reading data from ColumnarDelta for the Database API.
pub fn batch_to_kv(batch: &RecordBatch) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
    if batch.num_rows() == 0 {
        return Ok(Vec::new());
    }

    // Verify schema
    if batch.num_columns() != 2 {
        return Err(DbxError::Schema(format!(
            "Expected 2 columns (key, value), got {}",
            batch.num_columns()
        )));
    }

    let key_array = batch
        .column(0)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| DbxError::Schema("Key column is not Binary".to_string()))?;

    let value_array = batch
        .column(1)
        .as_any()
        .downcast_ref::<BinaryArray>()
        .ok_or_else(|| DbxError::Schema("Value column is not Binary".to_string()))?;

    let mut result = Vec::with_capacity(batch.num_rows());

    for i in 0..batch.num_rows() {
        let key = key_array.value(i).to_vec();
        let value = value_array.value(i).to_vec();
        result.push((key, value));
    }

    Ok(result)
}

/// Merge multiple RecordBatches with the same schema.
///
/// This is useful for combining results from multiple VersionedBatches.
pub fn merge_batches(batches: Vec<Arc<RecordBatch>>) -> DbxResult<RecordBatch> {
    if batches.is_empty() {
        return Ok(RecordBatch::new_empty(kv_schema()));
    }

    if batches.len() == 1 {
        return Ok((*batches[0]).clone());
    }

    // Verify all batches have the same schema
    let schema = batches[0].schema();
    for batch in &batches[1..] {
        if batch.schema() != schema {
            return Err(DbxError::Schema(
                "Cannot merge batches with different schemas".to_string(),
            ));
        }
    }

    // Concatenate all columns
    let num_columns = batches[0].num_columns();
    let mut merged_columns: Vec<ArrayRef> = Vec::with_capacity(num_columns);

    for col_idx in 0..num_columns {
        let arrays: Vec<&dyn arrow::array::Array> =
            batches.iter().map(|b| b.column(col_idx).as_ref()).collect();

        let merged = arrow::compute::concat(&arrays)?;
        merged_columns.push(merged);
    }

    Ok(RecordBatch::try_new(schema, merged_columns)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_schema() {
        let schema = kv_schema();
        assert_eq!(schema.fields().len(), 2);
        assert_eq!(schema.field(0).name(), "key");
        assert_eq!(schema.field(1).name(), "value");
        assert_eq!(schema.field(0).data_type(), &DataType::Binary);
        assert_eq!(schema.field(1).data_type(), &DataType::Binary);
    }

    #[test]
    fn test_kv_to_batch() {
        let rows = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];

        let batch = kv_to_batch(rows).unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 2);
    }

    #[test]
    fn test_batch_to_kv() {
        let rows = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];

        let batch = kv_to_batch(rows.clone()).unwrap();
        let recovered = batch_to_kv(&batch).unwrap();

        assert_eq!(recovered, rows);
    }

    #[test]
    fn test_round_trip() {
        let original = vec![
            (b"alice".to_vec(), b"data1".to_vec()),
            (b"bob".to_vec(), b"data2".to_vec()),
            (b"charlie".to_vec(), b"data3".to_vec()),
        ];

        let batch = kv_to_batch(original.clone()).unwrap();
        let recovered = batch_to_kv(&batch).unwrap();

        assert_eq!(recovered, original);
    }

    #[test]
    fn test_empty_batch() {
        let batch = kv_to_batch(vec![]).unwrap();
        assert_eq!(batch.num_rows(), 0);

        let recovered = batch_to_kv(&batch).unwrap();
        assert_eq!(recovered.len(), 0);
    }

    #[test]
    fn test_merge_batches() {
        let batch1 = kv_to_batch(vec![(b"key1".to_vec(), b"value1".to_vec())]).unwrap();

        let batch2 = kv_to_batch(vec![(b"key2".to_vec(), b"value2".to_vec())]).unwrap();

        let merged = merge_batches(vec![Arc::new(batch1), Arc::new(batch2)]).unwrap();
        assert_eq!(merged.num_rows(), 2);

        let rows = batch_to_kv(&merged).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, b"key1");
        assert_eq!(rows[1].0, b"key2");
    }

    #[test]
    fn test_merge_empty() {
        let merged = merge_batches(vec![]).unwrap();
        assert_eq!(merged.num_rows(), 0);
    }

    #[test]
    fn test_merge_single() {
        let batch = kv_to_batch(vec![(b"key1".to_vec(), b"value1".to_vec())]).unwrap();

        let merged = merge_batches(vec![Arc::new(batch.clone())]).unwrap();
        assert_eq!(merged.num_rows(), 1);
    }
}
