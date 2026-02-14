//! Encrypted Parquet I/O — application-level encryption for ROS (Tier 5).
//!
//! Since the `parquet` crate's Rust implementation does not yet support
//! Parquet's native Modular Encryption, we provide application-level
//! encryption: the entire Parquet file is encrypted/decrypted as a blob.
//!
//! # Architecture
//!
//! ```text
//! RecordBatch
//!     │ ArrowWriter (with compression)
//!     ▼
//! Parquet bytes (in-memory buffer)
//!     │ AEAD encrypt
//!     ▼
//! Encrypted file on disk
//! ```
//!
//! # Wire Format
//!
//! ```text
//! [magic: 4 bytes "EPQT"] [version: 1 byte] [nonce: 12 bytes] [ciphertext + tag]
//! ```

use crate::error::{DbxError, DbxResult};
use crate::storage::compression::CompressionConfig;
use crate::storage::encryption::EncryptionConfig;
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Magic bytes identifying encrypted Parquet files.
const MAGIC: &[u8; 4] = b"EPQT";
/// Current wire format version.
const VERSION: u8 = 1;
/// AAD for Parquet file encryption.
const PARQUET_AAD: &[u8] = b"dbx-parquet-v1";

/// Writes encrypted Parquet files.
pub struct EncryptedParquetWriter;

/// Reads encrypted Parquet files.
pub struct EncryptedParquetReader;

impl EncryptedParquetWriter {
    /// Write a single RecordBatch with encryption and default compression.
    pub fn write(path: &Path, batch: &RecordBatch, encryption: &EncryptionConfig) -> DbxResult<()> {
        Self::write_with_compression(path, batch, encryption, &CompressionConfig::default())
    }

    /// Write a single RecordBatch with encryption and specified compression.
    pub fn write_with_compression(
        path: &Path,
        batch: &RecordBatch,
        encryption: &EncryptionConfig,
        compression: &CompressionConfig,
    ) -> DbxResult<()> {
        Self::write_batches(path, std::slice::from_ref(batch), encryption, compression)
    }

    /// Write multiple RecordBatches with encryption and compression.
    pub fn write_batches(
        path: &Path,
        batches: &[RecordBatch],
        encryption: &EncryptionConfig,
        compression: &CompressionConfig,
    ) -> DbxResult<()> {
        if batches.is_empty() {
            return Ok(());
        }

        // Step 1: Write Parquet to a temp file, then read its bytes
        let tmp = tempfile::NamedTempFile::new()?;
        {
            let file = tmp.reopen()?;
            let props = WriterProperties::builder()
                .set_compression(compression.to_parquet_compression())
                .build();

            let mut writer = ArrowWriter::try_new(file, batches[0].schema(), Some(props))?;
            for batch in batches {
                writer.write(batch)?;
            }
            writer.close()?;
        }

        // Read the temp file bytes
        let buf = std::fs::read(tmp.path())?;

        // Step 2: Encrypt the buffer
        let ciphertext = encryption.encrypt_with_aad(&buf, PARQUET_AAD)?;

        // Step 3: Write to target file with header
        let mut file = File::create(path)?;
        file.write_all(MAGIC)?;
        file.write_all(&[VERSION])?;
        file.write_all(&ciphertext)?;
        file.flush()?;

        Ok(())
    }

    /// Re-key an existing encrypted Parquet file.
    ///
    /// Reads the file with `current_encryption`, decrypts it,
    /// and re-encrypts with `new_encryption`.
    pub fn rekey(
        path: &Path,
        current_encryption: &EncryptionConfig,
        new_encryption: &EncryptionConfig,
    ) -> DbxResult<()> {
        use super::encrypted_parquet::EncryptedParquetReader;

        // 1. Read and decrypt
        let batches = EncryptedParquetReader::read(path, current_encryption)?;

        // 2. Re-encrypt and write
        Self::write_batches(
            path,
            &batches,
            new_encryption,
            &crate::storage::compression::CompressionConfig::default(),
        )
    }
}

impl EncryptedParquetReader {
    /// Read all RecordBatches from an encrypted Parquet file.
    pub fn read(path: &Path, encryption: &EncryptionConfig) -> DbxResult<Vec<RecordBatch>> {
        // Step 1: Read file
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        // Step 2: Validate header
        if data.len() < 5 {
            return Err(DbxError::Encryption(
                "file too short for encrypted Parquet".into(),
            ));
        }
        if &data[0..4] != MAGIC {
            return Err(DbxError::Encryption(
                "invalid encrypted Parquet magic".into(),
            ));
        }
        if data[4] != VERSION {
            return Err(DbxError::Encryption(format!(
                "unsupported encrypted Parquet version: {}",
                data[4]
            )));
        }

        // Step 3: Decrypt
        let ciphertext = &data[5..];
        let parquet_bytes = encryption.decrypt_with_aad(ciphertext, PARQUET_AAD)?;

        // Step 4: Write decrypted bytes to temp file and read as Parquet
        let mut tmp = tempfile::NamedTempFile::new()?;
        tmp.write_all(&parquet_bytes)?;
        tmp.flush()?;

        let file = File::open(tmp.path())?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;

        let mut batches = Vec::new();
        for batch_result in reader {
            batches.push(batch_result?);
        }
        Ok(batches)
    }

    /// Check if a file is an encrypted Parquet file.
    pub fn is_encrypted_parquet(path: &Path) -> DbxResult<bool> {
        let mut file = File::open(path)?;
        let mut magic_buf = [0u8; 4];
        match file.read_exact(&mut magic_buf) {
            Ok(()) => Ok(&magic_buf == MAGIC),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;
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

    fn test_encryption() -> EncryptionConfig {
        EncryptionConfig::from_password("test-parquet-password")
    }

    #[test]
    fn encrypted_parquet_round_trip() {
        let schema = test_schema();
        let batch = test_batch(&schema, 100);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write(tmp.path(), &batch, &enc).unwrap();

        let read_batches = EncryptedParquetReader::read(tmp.path(), &enc).unwrap();
        assert!(!read_batches.is_empty());

        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 100);

        let ids = read_batches[0]
            .column(0)
            .as_any()
            .downcast_ref::<Int32Array>()
            .unwrap();
        assert_eq!(ids.value(0), 0);
    }

    #[test]
    fn wrong_key_fails() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write(tmp.path(), &batch, &enc).unwrap();

        let wrong_enc = EncryptionConfig::from_password("wrong-password");
        let result = EncryptedParquetReader::read(tmp.path(), &wrong_enc);
        assert!(result.is_err());
    }

    #[test]
    fn is_encrypted_check() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write(tmp.path(), &batch, &enc).unwrap();

        assert!(EncryptedParquetReader::is_encrypted_parquet(tmp.path()).unwrap());
    }

    #[test]
    fn plain_parquet_not_encrypted() {
        use crate::storage::parquet_io::ParquetWriter;

        let schema = test_schema();
        let batch = test_batch(&schema, 10);

        let tmp = NamedTempFile::new().unwrap();
        ParquetWriter::write(tmp.path(), &batch).unwrap();

        assert!(!EncryptedParquetReader::is_encrypted_parquet(tmp.path()).unwrap());
    }

    #[test]
    fn large_batch_round_trip() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10_000);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write(tmp.path(), &batch, &enc).unwrap();

        let read_batches = EncryptedParquetReader::read(tmp.path(), &enc).unwrap();
        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 10_000);
    }

    #[test]
    fn raw_file_not_readable_as_parquet() {
        let schema = test_schema();
        let batch = test_batch(&schema, 10);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write(tmp.path(), &batch, &enc).unwrap();

        // Try to read as plain Parquet — should fail
        let result = ParquetRecordBatchReaderBuilder::try_new(File::open(tmp.path()).unwrap());
        assert!(
            result.is_err(),
            "Encrypted file should not be readable as plain Parquet"
        );
    }

    #[test]
    fn multiple_batches() {
        let schema = test_schema();
        let batch1 = test_batch(&schema, 50);
        let batch2 = test_batch(&schema, 50);
        let enc = test_encryption();

        let tmp = NamedTempFile::new().unwrap();
        EncryptedParquetWriter::write_batches(
            tmp.path(),
            &[batch1, batch2],
            &enc,
            &CompressionConfig::default(),
        )
        .unwrap();

        let read_batches = EncryptedParquetReader::read(tmp.path(), &enc).unwrap();
        let total_rows: usize = read_batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 100);
    }
}
