//! Parquet I/O — Tier 5 (ROS) file operations.
//!
//! Provides RecordBatch ↔ Parquet file read/write with configurable compression.
//!
//! See [`super::compression::CompressionConfig`] for algorithm selection.

use crate::error::DbxResult;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use super::compression::CompressionConfig;

/// Writes Arrow RecordBatches to Parquet files.
pub struct ParquetWriter;

/// Reads Parquet files into Arrow RecordBatches.
pub struct ParquetReader;

impl ParquetWriter {
    /// Write a single RecordBatch with default compression (Snappy).
    pub fn write(path: &Path, batch: &RecordBatch) -> DbxResult<()> {
        Self::write_with_compression(path, batch, &CompressionConfig::default())
    }

    /// Write a single RecordBatch with the specified compression.
    pub fn write_with_compression(
        path: &Path,
        batch: &RecordBatch,
        compression: &CompressionConfig,
    ) -> DbxResult<()> {
        Self::write_batches_with_compression(path, std::slice::from_ref(batch), compression)
    }

    /// Write multiple RecordBatches with default compression (Snappy).
    pub fn write_batches(path: &Path, batches: &[RecordBatch]) -> DbxResult<()> {
        Self::write_batches_with_compression(path, batches, &CompressionConfig::default())
    }

    /// Write multiple RecordBatches with the specified compression.
    pub fn write_batches_with_compression(
        path: &Path,
        batches: &[RecordBatch],
        compression: &CompressionConfig,
    ) -> DbxResult<()> {
        if batches.is_empty() {
            return Ok(());
        }

        let file = File::create(path)?;
        let props = WriterProperties::builder()
            .set_compression(compression.to_parquet_compression())
            .build();

        let mut writer = ArrowWriter::try_new(file, batches[0].schema(), Some(props))?;

        for batch in batches {
            writer.write(batch)?;
        }

        writer.close()?;
        Ok(())
    }
}

impl ParquetReader {
    /// Read all RecordBatches from a Parquet file.
    pub fn read(path: &Path) -> DbxResult<Vec<RecordBatch>> {
        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut batches = Vec::new();
        for batch_result in reader {
            batches.push(batch_result?);
        }
        Ok(batches)
    }

    /// Read only the schema from a Parquet file without loading data.
    pub fn read_schema(path: &Path) -> DbxResult<Arc<Schema>> {
        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        Ok(builder.schema().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field};
    use tempfile::NamedTempFile;

    fn test_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("value", DataType::Float64, true),
        ]))
    }

    fn test_batch(schema: &Arc<Schema>, count: usize) -> RecordBatch {
        let ids: Vec<i32> = (0..count as i32).collect();
        let names: Vec<String> = (0..count).map(|i| format!("item_{i}")).collect();
        let values: Vec<f64> = (0..count).map(|i| i as f64 * 1.5).collect();

        RecordBatch::try_new(
            Arc::clone(schema),
            vec![
                Arc::new(Int32Array::from(ids)),
                Arc::new(StringArray::from(names)),
                Arc::new(Float64Array::from(values)),
            ],
        )
        .unwrap()
    }

    #[test]
    fn write_and_read_round_trip() {
        let schema = test_schema();
        let batch = test_batch(&schema, 100);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write(tmp.path(), &batch).unwrap();

        let read_batches = ParquetReader::read(tmp.path()).unwrap();
        assert!(!read_batches.is_empty());

        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 100);

        let first = &read_batches[0];
        let ids = first
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.value(0), 0);

        let names = first
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(names.value(0), "item_0");
    }

    #[test]
    fn read_schema_only() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write(tmp.path(), &batch).unwrap();

        let read_schema = ParquetReader::read_schema(tmp.path()).unwrap();
        assert_eq!(read_schema.fields().len(), 3);
        assert_eq!(read_schema.field(0).name(), "id");
        assert_eq!(*read_schema.field(0).data_type(), DataType::Int32);
    }

    #[test]
    fn round_trip_1000_rows() {
        let schema = test_schema();
        let batch = test_batch(&schema, 1000);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write(tmp.path(), &batch).unwrap();

        let read_batches = ParquetReader::read(tmp.path()).unwrap();
        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 1000);

        // Verify last row
        let last_batch = read_batches.last().unwrap();
        let last_row_idx = last_batch.num_rows() - 1;
        let ids = last_batch
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.value(last_row_idx), 999);
    }

    #[test]
    fn write_multiple_batches() {
        let schema = test_schema();
        let batch1 = test_batch(&schema, 50);
        let batch2 = test_batch(&schema, 50);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write_batches(tmp.path(), &[batch1, batch2]).unwrap();

        let read_batches = ParquetReader::read(tmp.path()).unwrap();
        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 100);
    }

    #[test]
    fn file_size_with_compression() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10_000);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write(tmp.path(), &batch).unwrap();

        let metadata = std::fs::metadata(tmp.path()).unwrap();
        // Snappy-compressed 10K rows should be reasonably small
        assert!(
            metadata.len() < 500_000,
            "file too large: {} bytes",
            metadata.len()
        );
    }

    #[test]
    fn round_trip_all_algorithms() {
        use super::super::compression::{CompressionAlgorithm, CompressionConfig};

        let schema = test_schema();
        let batch = test_batch(&schema, 500);

        for algo in CompressionAlgorithm::ALL {
            let config = CompressionConfig::new(*algo);
            let tmp = NamedTempFile::new().unwrap();

            ParquetWriter::write_with_compression(tmp.path(), &batch, &config).unwrap();
            let read_batches = ParquetReader::read(tmp.path()).unwrap();

            let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
            assert_eq!(
                total_rows, 500,
                "Round-trip failed for {:?}: expected 500 rows, got {}",
                algo, total_rows
            );

            // Verify data integrity
            let ids = read_batches[0]
                .column(0)
                .as_any()
                .downcast_ref::<Int32Array>()
                .unwrap();
            assert_eq!(ids.value(0), 0, "Data mismatch for {:?}", algo);
        }
    }

    #[test]
    fn zstd_levels_produce_different_sizes() {
        use super::super::compression::CompressionConfig;

        let schema = test_schema();
        let batch = test_batch(&schema, 10_000);

        let mut sizes = Vec::new();
        for level in &[1, 9, 19] {
            let config = CompressionConfig::zstd_level(*level);
            let tmp = NamedTempFile::new().unwrap();
            ParquetWriter::write_with_compression(tmp.path(), &batch, &config).unwrap();
            let size = std::fs::metadata(tmp.path()).unwrap().len();
            sizes.push((*level, size));
        }

        // Higher levels should generally produce smaller or equal files
        // (not strict because Parquet has overhead, but level 1 should be >= level 19)
        assert!(
            sizes[0].1 >= sizes[2].1,
            "ZSTD level 1 ({} bytes) should be >= level 19 ({} bytes)",
            sizes[0].1,
            sizes[2].1
        );
    }

    #[test]
    fn no_compression_largest() {
        use super::super::compression::CompressionConfig;

        let schema = test_schema();
        let batch = test_batch(&schema, 5_000);

        let tmp_none = NamedTempFile::new().unwrap();
        ParquetWriter::write_with_compression(tmp_none.path(), &batch, &CompressionConfig::none())
            .unwrap();
        let size_none = std::fs::metadata(tmp_none.path()).unwrap().len();

        let tmp_snappy = NamedTempFile::new().unwrap();
        ParquetWriter::write_with_compression(
            tmp_snappy.path(),
            &batch,
            &CompressionConfig::snappy(),
        )
        .unwrap();
        let size_snappy = std::fs::metadata(tmp_snappy.path()).unwrap().len();

        // Uncompressed should be larger than Snappy
        assert!(
            size_none > size_snappy,
            "Uncompressed ({} bytes) should be > Snappy ({} bytes)",
            size_none,
            size_snappy
        );
    }
}
