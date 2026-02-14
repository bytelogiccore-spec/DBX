//! Checkpoint manager for WAL maintenance and crash recovery.
//!
//! The checkpoint manager coordinates with the WAL to:
//! - Apply WAL changes to the persistent storage
//! - Trim old WAL records after successful checkpoint
//! - Recover database state by replaying WAL records
//!
//! # Example
//!
//! ```rust
//! use dbx_core::wal::WriteAheadLog;
//! use dbx_core::wal::checkpoint::CheckpointManager;
//! use std::sync::Arc;
//! use std::path::Path;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! let wal = Arc::new(WriteAheadLog::open(Path::new("./wal.log"))?);
//! let checkpoint_mgr = CheckpointManager::new(wal, Path::new("./wal.log"));
//!
//! // Perform checkpoint (apply WAL to storage)
//! // checkpoint_mgr.checkpoint(&db)?;
//! # Ok(())
//! # }
//! ```

use crate::error::{DbxError, DbxResult};
use crate::wal::{WalRecord, WriteAheadLog};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Checkpoint manager for WAL maintenance.
///
/// Manages periodic checkpoints and WAL trimming to keep the WAL file size bounded.
pub struct CheckpointManager {
    /// Reference to the WAL
    wal: Arc<WriteAheadLog>,

    /// Checkpoint interval (default: 30 seconds)
    interval: Duration,

    /// Path to the WAL file (for trimming)
    wal_path: PathBuf,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager.
    ///
    /// # Arguments
    ///
    /// * `wal` - Shared reference to the WAL
    /// * `wal_path` - Path to the WAL file
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::wal::WriteAheadLog;
    /// # use dbx_core::wal::checkpoint::CheckpointManager;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = Arc::new(WriteAheadLog::open(Path::new("./wal.log"))?);
    /// let checkpoint_mgr = CheckpointManager::new(wal, Path::new("./wal.log"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(wal: Arc<WriteAheadLog>, wal_path: &Path) -> Self {
        Self {
            wal,
            interval: Duration::from_secs(30),
            wal_path: wal_path.to_path_buf(),
        }
    }

    /// Sets the checkpoint interval.
    ///
    /// # Arguments
    ///
    /// * `interval` - Duration between checkpoints
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Performs a checkpoint.
    ///
    /// Applies all WAL records to the database and writes a checkpoint marker.
    /// This method should be called by the Database engine.
    ///
    /// # Arguments
    ///
    /// * `apply_fn` - Function to apply a WAL record to the database
    ///
    /// # Returns
    ///
    /// The sequence number of the checkpoint
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use dbx_core::wal::WriteAheadLog;
    /// # use dbx_core::wal::checkpoint::CheckpointManager;
    /// # use dbx_core::wal::WalRecord;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = Arc::new(WriteAheadLog::open(Path::new("./wal.log"))?);
    /// let checkpoint_mgr = CheckpointManager::new(wal, Path::new("./wal.log"));
    ///
    /// let apply_fn = |record: &WalRecord| -> dbx_core::DbxResult<()> {
    ///     // Apply record to database
    ///     Ok(())
    /// };
    ///
    /// let checkpoint_seq = checkpoint_mgr.checkpoint(apply_fn)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn checkpoint<F>(&self, apply_fn: F) -> DbxResult<u64>
    where
        F: Fn(&WalRecord) -> DbxResult<()>,
    {
        // Replay all WAL records
        let records = self.wal.replay()?;

        for record in &records {
            // Skip checkpoint markers
            if matches!(record, WalRecord::Checkpoint { .. }) {
                continue;
            }

            // Apply record to database
            apply_fn(record)?;
        }

        // Write checkpoint marker
        let seq = self.wal.current_sequence();
        let checkpoint_record = WalRecord::Checkpoint { sequence: seq };
        self.wal.append(&checkpoint_record)?;
        self.wal.sync()?;

        Ok(seq)
    }

    /// Recovers the database by replaying WAL records.
    ///
    /// This is called during database startup to restore the state after a crash.
    ///
    /// # Arguments
    ///
    /// * `wal_path` - Path to the WAL file
    /// * `apply_fn` - Function to apply a WAL record to the database
    ///
    /// # Returns
    ///
    /// The number of records replayed
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use dbx_core::wal::checkpoint::CheckpointManager;
    /// # use dbx_core::wal::WalRecord;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let apply_fn = |record: &WalRecord| -> dbx_core::DbxResult<()> {
    ///     // Apply record to database
    ///     Ok(())
    /// };
    ///
    /// let count = CheckpointManager::recover(Path::new("./wal.log"), apply_fn)?;
    /// println!("Replayed {} records", count);
    /// # Ok(())
    /// # }
    /// ```
    pub fn recover<F>(wal_path: &Path, apply_fn: F) -> DbxResult<usize>
    where
        F: Fn(&WalRecord) -> DbxResult<()>,
    {
        // Check if WAL file exists
        if !wal_path.exists() {
            return Ok(0);
        }

        let wal = WriteAheadLog::open(wal_path)?;
        let records = wal.replay()?;

        // Find the last checkpoint
        let mut last_checkpoint_idx = None;
        for (i, record) in records.iter().enumerate() {
            if matches!(record, WalRecord::Checkpoint { .. }) {
                last_checkpoint_idx = Some(i);
            }
        }

        // Replay records after the last checkpoint
        let start_idx = last_checkpoint_idx.map(|i| i + 1).unwrap_or(0);
        let replay_count = records.len() - start_idx;

        for record in &records[start_idx..] {
            apply_fn(record)?;
        }

        Ok(replay_count)
    }

    /// Trims the WAL file by removing records before the specified sequence.
    ///
    /// This is called after a successful checkpoint to keep the WAL file size bounded.
    ///
    /// # Arguments
    ///
    /// * `sequence` - Sequence number to trim before
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use dbx_core::wal::WriteAheadLog;
    /// # use dbx_core::wal::checkpoint::CheckpointManager;
    /// # use std::sync::Arc;
    /// # use std::path::Path;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let wal = Arc::new(WriteAheadLog::open(Path::new("./wal.log"))?);
    /// let checkpoint_mgr = CheckpointManager::new(wal, Path::new("./wal.log"));
    ///
    /// // Trim records before sequence 100
    /// checkpoint_mgr.trim_before(100)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn trim_before(&self, sequence: u64) -> DbxResult<()> {
        // Read all records
        let records = self.wal.replay()?;

        // Find the last checkpoint with sequence >= target
        let mut last_checkpoint_idx = None;
        for (i, record) in records.iter().enumerate() {
            if let WalRecord::Checkpoint { sequence: seq } = record
                && *seq >= sequence
            {
                last_checkpoint_idx = Some(i);
            }
        }

        // Keep only records from the last checkpoint onwards
        let trimmed_records: Vec<WalRecord> = if let Some(idx) = last_checkpoint_idx {
            records.into_iter().skip(idx).collect()
        } else {
            // No checkpoint found, keep all records
            records
        };

        // Write trimmed records to a temporary file
        let temp_path = self.wal_path.with_extension("tmp");
        let mut temp_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_path)?;

        for record in &trimmed_records {
            let encoded = bincode::serialize(record)
                .map_err(|e| DbxError::Wal(format!("serialization failed: {}", e)))?;
            let len = (encoded.len() as u32).to_le_bytes();
            temp_file.write_all(&len)?;
            temp_file.write_all(&encoded)?;
        }

        temp_file.sync_all()?;
        drop(temp_file);

        // Replace the original WAL file with the trimmed one
        std::fs::rename(&temp_path, &self.wal_path)?;

        Ok(())
    }

    /// Returns the checkpoint interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn checkpoint_applies_wal() {
        use std::cell::RefCell;

        let temp_file = NamedTempFile::new().unwrap();
        let wal = Arc::new(WriteAheadLog::open(temp_file.path()).unwrap());
        let checkpoint_mgr = CheckpointManager::new(wal.clone(), temp_file.path());

        // Add some records
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

        wal.append(&record1).unwrap();
        wal.append(&record2).unwrap();
        wal.sync().unwrap();

        // Checkpoint
        let applied_records = RefCell::new(Vec::new());
        let apply_fn = |record: &WalRecord| {
            applied_records.borrow_mut().push(record.clone());
            Ok(())
        };

        let checkpoint_seq = checkpoint_mgr.checkpoint(apply_fn).unwrap();
        assert!(checkpoint_seq > 0);
        let records = applied_records.borrow();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0], record1);
        assert_eq!(records[1], record2);
    }

    #[test]
    fn recover_replays_after_checkpoint() {
        use std::cell::RefCell;

        let temp_file = NamedTempFile::new().unwrap();
        let wal = Arc::new(WriteAheadLog::open(temp_file.path()).unwrap());

        // Add records before checkpoint
        let record1 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:1".to_vec(),
            value: b"Alice".to_vec(),
            ts: 0,
        };
        wal.append(&record1).unwrap();

        // Checkpoint
        let checkpoint = WalRecord::Checkpoint { sequence: 1 };
        wal.append(&checkpoint).unwrap();

        // Add records after checkpoint
        let record2 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:2".to_vec(),
            value: b"Bob".to_vec(),
            ts: 2, // After checkpoint
        };
        wal.append(&record2).unwrap();
        wal.sync().unwrap();

        // Recover
        let recovered_records = RefCell::new(Vec::new());
        let apply_fn = |record: &WalRecord| {
            recovered_records.borrow_mut().push(record.clone());
            Ok(())
        };

        let count = CheckpointManager::recover(temp_file.path(), apply_fn).unwrap();

        // Should only replay record2 (after checkpoint)
        assert_eq!(count, 1);
        let records = recovered_records.borrow();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0], record2);
    }

    #[test]
    fn trim_removes_old_records() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = Arc::new(WriteAheadLog::open(temp_file.path()).unwrap());
        let checkpoint_mgr = CheckpointManager::new(wal.clone(), temp_file.path());

        // Add records
        let record1 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:1".to_vec(),
            value: b"Alice".to_vec(),
            ts: 0,
        };
        wal.append(&record1).unwrap();

        // Checkpoint
        let checkpoint = WalRecord::Checkpoint { sequence: 1 };
        wal.append(&checkpoint).unwrap();

        let record2 = WalRecord::Insert {
            table: "users".to_string(),
            key: b"user:2".to_vec(),
            value: b"Bob".to_vec(),
            ts: 2,
        };
        wal.append(&record2).unwrap();
        wal.sync().unwrap();

        // Trim before sequence 1
        checkpoint_mgr.trim_before(1).unwrap();

        // Re-open and verify
        let wal2 = WriteAheadLog::open(temp_file.path()).unwrap();
        let records = wal2.replay().unwrap();

        // Should only have checkpoint and record2
        assert_eq!(records.len(), 2);
        assert!(matches!(records[0], WalRecord::Checkpoint { sequence: 1 }));
        assert_eq!(records[1], record2);
    }

    #[test]
    fn recover_empty_wal() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::remove_file(temp_file.path()).unwrap();

        let apply_fn = |_: &WalRecord| Ok(());
        let count = CheckpointManager::recover(temp_file.path(), apply_fn).unwrap();

        assert_eq!(count, 0);
    }

    #[test]
    fn checkpoint_interval() {
        let temp_file = NamedTempFile::new().unwrap();
        let wal = Arc::new(WriteAheadLog::open(temp_file.path()).unwrap());

        let checkpoint_mgr =
            CheckpointManager::new(wal, temp_file.path()).with_interval(Duration::from_secs(60));

        assert_eq!(checkpoint_mgr.interval(), Duration::from_secs(60));
    }
}
