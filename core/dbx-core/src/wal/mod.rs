//! Write-Ahead Logging (WAL) module for crash recovery.
//!
//! The WAL ensures durability by logging all write operations before applying them
//! to the database. In case of a crash, the WAL can be replayed to restore the
//! database to a consistent state.
//!
//! # Architecture
//!
//! - **WalRecord**: Enum representing different types of log entries
//! - **WriteAheadLog**: Core WAL implementation with append/sync/replay
//! - **CheckpointManager**: Manages periodic checkpoints and WAL trimming
//!
//! # Example
//!
//! ```rust
//! use dbx_core::wal::{WriteAheadLog, WalRecord};
//! use std::path::Path;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! let wal = WriteAheadLog::open(Path::new("./wal.log"))?;
//!
//! // Log an insert operation
//! let record = WalRecord::Insert {
//!     table: "users".to_string(),
//!     key: b"user:1".to_vec(),
//!     value: b"Alice".to_vec(),
//!     ts: 0,
//! };
//! let seq = wal.append(&record)?;
//! wal.sync()?;  // Ensure durability
//! # Ok(())
//! # }
//! ```

use crate::error::{DbxError, DbxResult};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

pub mod buffer;
pub mod checkpoint;
pub mod encrypted_wal;

/// WAL record types.
///
/// Each record represents a single operation that can be replayed during recovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WalRecord {
    /// Insert operation: table, key, value
    /// Insert operation: table, key, value, timestamp
    Insert {
        table: String,
        key: Vec<u8>,
        value: Vec<u8>,
        ts: u64,
    },

    /// Delete operation: table, key, timestamp
    Delete {
        table: String,
        key: Vec<u8>,
        ts: u64,
    },

    /// Checkpoint marker: sequence number
    Checkpoint { sequence: u64 },

    /// Transaction commit: transaction ID
    Commit { tx_id: u64 },

    /// Transaction rollback: transaction ID
    Rollback { tx_id: u64 },

    /// Batch operation: table, list of (key, value) pairs
    Batch {
        table: String,
        rows: Vec<(Vec<u8>, Vec<u8>)>,
        ts: u64,
    },
}

/// Write-Ahead Log for crash recovery.
///
/// All write operations are logged to disk before being applied to the database.
/// This ensures that the database can be recovered to a consistent state after a crash.
///
/// # Thread Safety
///
/// `WriteAheadLog` is thread-safe and can be shared across multiple threads using `Arc`.
pub struct WriteAheadLog {
    /// Log file handle (protected by mutex for concurrent writes)
    log_file: Mutex<File>,

    /// Path to the WAL file (for replay)
    path: std::path::PathBuf,

    /// Monotonically increasing sequence number
    sequence: AtomicU64,
}

impl WriteAheadLog {
    /// Opens or creates a WAL file at the specified path.
    ///
    /// If the file exists, it will be opened in append mode.
    /// The sequence number is initialized to the highest sequence in the existing log.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the WAL file
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::wal::WriteAheadLog;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = WriteAheadLog::open(Path::new("./wal.log"))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(path: &Path) -> DbxResult<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;

        // Scan existing log to find the highest sequence number
        let max_seq = Self::scan_max_sequence(path)?;

        Ok(Self {
            log_file: Mutex::new(file),
            path: path.to_path_buf(),
            sequence: AtomicU64::new(max_seq),
        })
    }

    /// Scans the WAL file to find the maximum sequence number.
    fn scan_max_sequence(path: &Path) -> DbxResult<u64> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };

        let mut reader = BufReader::new(file);
        let mut max_seq = 0u64;

        while let Ok(len_buf) = {
            let mut buf = [0u8; 4];
            std::io::Read::read_exact(&mut reader, &mut buf).map(|_| buf)
        } {
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            if std::io::Read::read_exact(&mut reader, &mut data).is_err() {
                break;
            }
            if let Ok(WalRecord::Checkpoint { sequence }) = bincode::deserialize::<WalRecord>(&data)
            {
                max_seq = max_seq.max(sequence);
            }
        }

        Ok(max_seq)
    }

    /// Appends a record to the WAL.
    ///
    /// Returns the sequence number assigned to this record.
    /// The record is buffered in memory until `sync()` is called.
    ///
    /// # Arguments
    ///
    /// * `record` - The WAL record to append
    ///
    /// # Returns
    ///
    /// The sequence number assigned to this record
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::wal::{WriteAheadLog, WalRecord};
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = WriteAheadLog::open(Path::new("./wal.log"))?;
    /// let record = WalRecord::Insert {
    ///     table: "users".to_string(),
    ///     key: b"key1".to_vec(),
    ///     value: b"value1".to_vec(),
    ///     ts: 0,
    /// };
    /// let seq = wal.append(&record)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn append(&self, record: &WalRecord) -> DbxResult<u64> {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Serialize record to Binary format
        let encoded = bincode::serialize(record)
            .map_err(|e| DbxError::Wal(format!("serialization failed: {}", e)))?;

        // Write to log file (Length-prefixed binary)
        let mut file = self
            .log_file
            .lock()
            .map_err(|e| DbxError::Wal(format!("lock failed: {}", e)))?;

        let len = (encoded.len() as u32).to_le_bytes();
        file.write_all(&len)?;
        file.write_all(&encoded)?;

        Ok(seq)
    }

    /// Synchronizes the WAL to disk (fsync).
    ///
    /// This ensures that all buffered writes are persisted to disk.
    /// Call this after critical operations to guarantee durability.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::wal::{WriteAheadLog, WalRecord};
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = WriteAheadLog::open(Path::new("./wal.log"))?;
    /// let record = WalRecord::Insert {
    ///     table: "users".to_string(),
    ///     key: b"key1".to_vec(),
    ///     value: b"value1".to_vec(),
    ///     ts: 0,
    /// };
    /// wal.append(&record)?;
    /// wal.sync()?;  // Ensure durability
    /// # Ok(())
    /// # }
    /// ```
    pub fn sync(&self) -> DbxResult<()> {
        let file = self
            .log_file
            .lock()
            .map_err(|e| DbxError::Wal(format!("lock failed: {}", e)))?;

        file.sync_all()?;
        Ok(())
    }

    /// Replays all records from the WAL.
    ///
    /// Reads the entire WAL file and returns all records in order.
    /// Used during database recovery to restore the state after a crash.
    ///
    /// # Returns
    ///
    /// A vector of all WAL records in the order they were written
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::wal::WriteAheadLog;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = WriteAheadLog::open(Path::new("./wal.log"))?;
    /// let records = wal.replay()?;
    /// for record in records {
    ///     // Apply record to database
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn replay(&self) -> DbxResult<Vec<WalRecord>> {
        // Open a new file handle for reading from the beginning
        let file = File::open(&self.path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();

        while let Ok(len_buf) = {
            let mut buf = [0u8; 4];
            std::io::Read::read_exact(&mut reader, &mut buf).map(|_| buf)
        } {
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            if std::io::Read::read_exact(&mut reader, &mut data).is_err() {
                break;
            }

            let record = bincode::deserialize::<WalRecord>(&data)
                .map_err(|e| DbxError::Wal(format!("deserialization failed: {}", e)))?;

            records.push(record);
        }

        Ok(records)
    }

    /// Returns the current sequence number.
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn wal_append_and_replay() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = WriteAheadLog::open(temp_file.path()).unwrap();

        // Append records
        let record1 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:1".to_vec(),
            value: b"Alice".to_vec(),
            ts: 1,
        };
        let record2 = WalRecord::Delete {
            table: "users".to_string(),
            key: b"user:2".to_vec(),
            ts: 2,
        };

        let seq1 = wal.append(&record1).unwrap();
        let seq2 = wal.append(&record2).unwrap();
        wal.sync().unwrap();

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);

        // Replay
        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], record1);
        assert_eq!(records[1], record2);
    }

    #[test]
    fn wal_sync_durability() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = WriteAheadLog::open(temp_file.path()).unwrap();

        let record = WalRecord::Insert {
            table: "test".to_string(),
            key: b"key".to_vec(),
            value: b"value".to_vec(),
            ts: 5,
        };

        wal.append(&record).unwrap();
        wal.sync().unwrap();

        // Re-open and verify
        let wal2 = WriteAheadLog::open(temp_file.path()).unwrap();
        let records = wal2.replay().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], record);
    }

    #[test]
    fn wal_sequence_increments() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = WriteAheadLog::open(temp_file.path()).unwrap();

        assert_eq!(wal.current_sequence(), 0);

        let record = WalRecord::Commit { tx_id: 1 };
        wal.append(&record).unwrap();
        assert_eq!(wal.current_sequence(), 1);

        wal.append(&record).unwrap();
        assert_eq!(wal.current_sequence(), 2);
    }

    #[test]
    fn wal_empty_replay() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = WriteAheadLog::open(temp_file.path()).unwrap();

        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 0);
    }

    #[test]
    fn wal_checkpoint_record() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = WriteAheadLog::open(temp_file.path()).unwrap();

        let checkpoint = WalRecord::Checkpoint { sequence: 42 };
        wal.append(&checkpoint).unwrap();
        wal.sync().unwrap();

        let records = wal.replay().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], checkpoint);
    }
}
