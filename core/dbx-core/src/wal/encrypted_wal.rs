//! Encrypted WAL (Write-Ahead Log) wrapper.
//!
//! Wraps [`WriteAheadLog`] to encrypt all log records before writing
//! and decrypt during replay. Provides at-rest encryption for the WAL.
//!
//! # Architecture
//!
//! ```text
//! WalRecord
//!     │ serialize (serde_json)
//!     ▼
//! JSON bytes
//!     │ encrypt (AEAD)
//!     ▼
//! base64-encoded ciphertext
//!     │ write to WAL file (one line per record)
//!     ▼
//! Disk
//! ```
//!
//! # Wire Format
//!
//! Each line in the encrypted WAL file is:
//! `base64(encrypt(json(WalRecord)))` + newline
//!
//! Using base64 ensures no newline bytes appear in the ciphertext,
//! preserving the line-oriented WAL format.

use crate::error::{DbxError, DbxResult};
use crate::storage::encryption::EncryptionConfig;
use crate::wal::WalRecord;

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// AEAD info string used as AAD for WAL records.
const WAL_AAD: &[u8] = b"dbx-wal-v1";

/// Encrypted Write-Ahead Log.
///
/// Encrypts each WAL record before writing it to disk.
/// During replay, records are decrypted and deserialized.
///
/// # Security Properties
///
/// - Each record is independently encrypted with a fresh nonce
/// - AAD prevents record type confusion
/// - Records cannot be read without the encryption key
///
/// # Examples
///
/// ```rust,no_run
/// use dbx_core::wal::encrypted_wal::EncryptedWal;
/// use dbx_core::wal::WalRecord;
/// use dbx_core::storage::encryption::EncryptionConfig;
/// use std::path::Path;
///
/// let enc = EncryptionConfig::from_password("secret");
/// let wal = EncryptedWal::open(Path::new("./wal.log"), enc).unwrap();
///
/// let record = WalRecord::Insert {
///     table: "users".to_string(),
///     key: b"user:1".to_vec(),
///     value: b"Alice".to_vec(),
///     ts: 0,
/// };
/// wal.append(&record).unwrap();
/// wal.sync().unwrap();
/// ```
pub struct EncryptedWal {
    /// Log file handle
    log_file: Mutex<File>,
    /// Path to WAL file (for replay)
    path: PathBuf,
    /// Monotonically increasing sequence number
    sequence: AtomicU64,
    /// Encryption configuration
    encryption: EncryptionConfig,
}

impl EncryptedWal {
    /// Open or create an encrypted WAL file.
    pub fn open(path: &Path, encryption: EncryptionConfig) -> DbxResult<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;

        // Scan for max sequence (try to decrypt each line)
        let max_seq = Self::scan_max_sequence(path, &encryption)?;

        Ok(Self {
            log_file: Mutex::new(file),
            path: path.to_path_buf(),
            sequence: AtomicU64::new(max_seq),
            encryption,
        })
    }

    /// Scan encrypted WAL to find the maximum sequence number.
    fn scan_max_sequence(path: &Path, encryption: &EncryptionConfig) -> DbxResult<u64> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };

        let reader = BufReader::new(file);
        let mut max_seq = 0u64;

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            // Try to decrypt and deserialize
            if let Ok(record) = Self::decrypt_line(&line, encryption)
                && let WalRecord::Checkpoint { sequence } = record
            {
                max_seq = max_seq.max(sequence);
            }
            // Count valid records for sequence tracking
            max_seq += 1;
        }

        Ok(max_seq)
    }

    /// Decrypt a single base64-encoded line.
    fn decrypt_line(line: &str, encryption: &EncryptionConfig) -> DbxResult<WalRecord> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        let ciphertext = STANDARD
            .decode(line.as_bytes())
            .map_err(|e| DbxError::Encryption(format!("base64 decode failed: {}", e)))?;

        let json_bytes = encryption.decrypt_with_aad(&ciphertext, WAL_AAD)?;

        serde_json::from_slice(&json_bytes)
            .map_err(|e| DbxError::Wal(format!("deserialization failed: {}", e)))
    }

    /// Append an encrypted record to the WAL.
    ///
    /// The record is serialized to JSON, encrypted, base64-encoded,
    /// and written as a single line.
    pub fn append(&self, record: &WalRecord) -> DbxResult<u64> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Serialize → encrypt → base64
        let json = serde_json::to_vec(record)
            .map_err(|e| DbxError::Wal(format!("serialization failed: {}", e)))?;

        let ciphertext = self.encryption.encrypt_with_aad(&json, WAL_AAD)?;
        let encoded = STANDARD.encode(&ciphertext);

        // Write as single line
        let mut file = self
            .log_file
            .lock()
            .map_err(|e| DbxError::Wal(format!("lock failed: {}", e)))?;

        file.write_all(encoded.as_bytes())?;
        file.write_all(b"\n")?;

        Ok(seq)
    }

    /// Synchronize WAL to disk (fsync).
    pub fn sync(&self) -> DbxResult<()> {
        let file = self
            .log_file
            .lock()
            .map_err(|e| DbxError::Wal(format!("lock failed: {}", e)))?;

        file.sync_all()?;
        Ok(())
    }

    /// Replay all encrypted records from the WAL.
    pub fn replay(&self) -> DbxResult<Vec<WalRecord>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let record = Self::decrypt_line(&line, &self.encryption)?;
            records.push(record);
        }

        Ok(records)
    }

    /// Returns the current sequence number.
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Get a reference to the encryption config.
    pub fn encryption_config(&self) -> &EncryptionConfig {
        &self.encryption
    }

    /// Re-key the WAL with a new encryption configuration.
    ///
    /// Reads all existing records, decrypts with the current key,
    /// then re-writes them encrypted with the new key.
    /// The old WAL file is atomically replaced.
    ///
    /// # Warning
    ///
    /// Callers should ensure no concurrent writes during rekey.
    pub fn rekey(&mut self, new_encryption: EncryptionConfig) -> DbxResult<usize> {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD;

        // Step 1: Replay all records with current key
        let records = self.replay()?;
        let count = records.len();

        // Step 2: Write records to a temp file with new key
        let tmp_path = self.path.with_extension("rekey.tmp");
        {
            let mut tmp_file = File::create(&tmp_path)?;
            for record in &records {
                let json = serde_json::to_vec(record)
                    .map_err(|e| DbxError::Wal(format!("serialization failed: {}", e)))?;
                let ciphertext = new_encryption.encrypt_with_aad(&json, WAL_AAD)?;
                let encoded = STANDARD.encode(&ciphertext);
                tmp_file.write_all(encoded.as_bytes())?;
                tmp_file.write_all(b"\n")?;
            }
            tmp_file.sync_all()?;
        }

        // Step 3: Atomically replace the old WAL file
        std::fs::rename(&tmp_path, &self.path)?;

        // Step 4: Re-open the new WAL file
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.path)?;

        *self
            .log_file
            .lock()
            .map_err(|e| DbxError::Wal(format!("lock failed: {}", e)))? = file;
        self.encryption = new_encryption;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_encryption() -> EncryptionConfig {
        EncryptionConfig::from_password("test-wal-password")
    }

    #[test]
    fn append_and_replay_round_trip() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let record1 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:1".to_vec(),
            value: b"Alice".to_vec(),
            ts: 0,
        };
        let record2 = WalRecord::Delete {
            table: "users".to_string(),
            key: b"user:2".to_vec(),
            ts: 1,
        };

        let seq1 = wal.append(&record1).unwrap();
        let seq2 = wal.append(&record2).unwrap();
        wal.sync().unwrap();

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);

        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], record1);
        assert_eq!(records[1], record2);
    }

    #[test]
    fn sync_durability() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let record = WalRecord::Insert {
            table: "test".to_string(),
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            ts: 0,
        };

        wal.append(&record).unwrap();
        wal.sync().unwrap();

        // Re-open and verify
        let wal2 = EncryptedWal::open(temp.path(), test_encryption()).unwrap();
        let records = wal2.replay().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], record);
    }

    #[test]
    fn wrong_key_cannot_replay() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let record = WalRecord::Insert {
            table: "secret".to_string(),
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            ts: 0,
        };

        wal.append(&record).unwrap();
        wal.sync().unwrap();

        // Try to replay with wrong key
        let wrong_enc = EncryptionConfig::from_password("wrong-password");
        let wal2 = EncryptedWal::open(temp.path(), wrong_enc).unwrap();
        let result = wal2.replay();
        assert!(result.is_err(), "Replay with wrong key should fail");
    }

    #[test]
    fn empty_wal_replay() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();
        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn checkpoint_record() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let checkpoint = WalRecord::Checkpoint { sequence: 42 };
        wal.append(&checkpoint).unwrap();
        wal.sync().unwrap();

        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], checkpoint);
    }

    #[test]
    fn multiple_record_types() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let records_to_write = vec![
            WalRecord::Insert {
                table: "t".to_string(),
                key: b"k1".to_vec(),
                value: b"v1".to_vec(),
                ts: 0,
            },
            WalRecord::Delete {
                table: "t".to_string(),
                key: b"k2".to_vec(),
                ts: 1,
            },
            WalRecord::Commit { tx_id: 1 },
            WalRecord::Rollback { tx_id: 2 },
            WalRecord::Checkpoint { sequence: 10 },
        ];

        for r in &records_to_write {
            wal.append(r).unwrap();
        }
        wal.sync().unwrap();

        let replayed = wal.replay().unwrap();
        assert_eq!(replayed, records_to_write);
    }

    #[test]
    fn raw_file_is_not_readable() {
        let temp = NamedTempFile::new().unwrap();
        let wal = EncryptedWal::open(temp.path(), test_encryption()).unwrap();

        let record = WalRecord::Insert {
            table: "secret".to_string(),
            key: b"key".to_vec(),
            value: b"sensitive_data".to_vec(),
            ts: 0,
        };

        wal.append(&record).unwrap();
        wal.sync().unwrap();

        // Read raw file content — should NOT contain plaintext
        let raw = std::fs::read_to_string(temp.path()).unwrap();
        assert!(!raw.contains("secret"));
        assert!(!raw.contains("sensitive_data"));
        assert!(!raw.contains("key"));
    }

    #[test]
    fn rekey_preserves_records() {
        let temp = NamedTempFile::new().unwrap();
        let enc_old = EncryptionConfig::from_password("old-key");
        let mut wal = EncryptedWal::open(temp.path(), enc_old).unwrap();

        let record1 = WalRecord::Insert {
            table: "t".to_string(),
            key: b"k1".to_vec(),
            value: b"v1".to_vec(),
            ts: 0,
        };
        let record2 = WalRecord::Delete {
            table: "t".to_string(),
            key: b"k2".to_vec(),
            ts: 1,
        };

        wal.append(&record1).unwrap();
        wal.append(&record2).unwrap();
        wal.sync().unwrap();

        // Rekey
        let enc_new = EncryptionConfig::from_password("new-key");
        let count = wal.rekey(enc_new.clone()).unwrap();
        assert_eq!(count, 2);

        // Verify with new key
        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], record1);
        assert_eq!(records[1], record2);

        // Re-open with new key should also work
        let wal2 = EncryptedWal::open(temp.path(), enc_new).unwrap();
        let records2 = wal2.replay().unwrap();
        assert_eq!(records2.len(), 2);

        // Old key should NOT work
        let enc_old2 = EncryptionConfig::from_password("old-key");
        let wal3 = EncryptedWal::open(temp.path(), enc_old2).unwrap();
        let result = wal3.replay();
        assert!(result.is_err());
    }
}
