//! DBX Node.js Native Bindings using napi-rs

use dbx_core::Database as CoreDatabase;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::Arc;

/// Database handle
#[napi]
pub struct Database {
    db: Arc<CoreDatabase>,
}

#[napi]
impl Database {
    /// Open an in-memory database
    #[napi(factory)]
    pub fn open_in_memory() -> Result<Self> {
        let db = CoreDatabase::open_in_memory()
            .map_err(|e| Error::from_reason(format!("Failed to open database: {e}")))?;
        Ok(Database { db: Arc::new(db) })
    }

    /// Open a database at the given path
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let db = CoreDatabase::open(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(format!("Failed to open database: {e}")))?;
        Ok(Database { db: Arc::new(db) })
    }

    // ═══════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════

    /// Insert a key-value pair into a table
    #[napi]
    pub fn insert(&self, table: String, key: Buffer, value: Buffer) -> Result<()> {
        self.db
            .insert(&table, &key, &value)
            .map(|_| ())
            .map_err(|e| Error::from_reason(format!("Insert failed: {e}")))
    }

    /// Get a value by key from a table
    #[napi]
    pub fn get(&self, table: String, key: Buffer) -> Result<Option<Buffer>> {
        match self.db.get(&table, &key) {
            Ok(Some(value)) => Ok(Some(value.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::from_reason(format!("Get failed: {e}"))),
        }
    }

    /// Delete a key from a table
    #[napi]
    pub fn delete(&self, table: String, key: Buffer) -> Result<()> {
        self.db
            .delete(&table, &key)
            .map(|_| ())
            .map_err(|e| Error::from_reason(format!("Delete failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // Batch Operations
    // ═══════════════════════════════════════════════════════

    /// Insert multiple key-value pairs at once (batch)
    #[napi]
    pub fn insert_batch(&self, table: String, rows: Vec<Vec<Buffer>>) -> Result<()> {
        let batch: Vec<(Vec<u8>, Vec<u8>)> = rows
            .into_iter()
            .filter_map(|row| {
                if row.len() == 2 {
                    Some((row[0].to_vec(), row[1].to_vec()))
                } else {
                    None
                }
            })
            .collect();

        self.db
            .insert_batch(&table, batch)
            .map_err(|e| Error::from_reason(format!("Batch insert failed: {e}")))
    }

    /// Delete multiple keys at once (batch)
    #[napi]
    pub fn delete_batch(&self, table: String, keys: Vec<Buffer>) -> Result<()> {
        for key in keys {
            self.db
                .delete(&table, &key)
                .map(|_| ())
                .map_err(|e| Error::from_reason(format!("Delete failed: {e}")))?;
        }
        Ok(())
    }

    /// Scan all key-value pairs in a table
    #[napi]
    pub fn scan(&self, table: String) -> Result<Vec<Vec<Buffer>>> {
        let entries = self
            .db
            .scan(&table)
            .map_err(|e| Error::from_reason(format!("Scan failed: {e}")))?;
        Ok(entries
            .into_iter()
            .map(|(k, v)| vec![Buffer::from(k), Buffer::from(v)])
            .collect())
    }

    /// Scan a range of keys in a table [start_key, end_key)
    #[napi]
    pub fn range(
        &self,
        table: String,
        start_key: Buffer,
        end_key: Buffer,
    ) -> Result<Vec<Vec<Buffer>>> {
        let entries = self
            .db
            .range(&table, &start_key, &end_key)
            .map_err(|e| Error::from_reason(format!("Range scan failed: {e}")))?;
        Ok(entries
            .into_iter()
            .map(|(k, v)| vec![Buffer::from(k), Buffer::from(v)])
            .collect())
    }

    // ═══════════════════════════════════════════════════════
    // Utility Operations
    // ═══════════════════════════════════════════════════════

    /// Count the number of rows in a table
    #[napi]
    pub fn count(&self, table: String) -> Result<u32> {
        self.db
            .count(&table)
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(format!("Count failed: {e}")))
    }

    /// Flush the database to disk
    #[napi]
    pub fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .map_err(|e| Error::from_reason(format!("Flush failed: {e}")))
    }

    /// Get all table names
    #[napi]
    pub fn table_names(&self) -> Result<Vec<String>> {
        self.db
            .table_names()
            .map_err(|e| Error::from_reason(format!("Failed to get table names: {e}")))
    }

    /// Run garbage collection (MVCC version cleanup)
    #[napi]
    pub fn gc(&self) -> Result<u32> {
        self.db
            .gc()
            .map(|c| c as u32)
            .map_err(|e| Error::from_reason(format!("GC failed: {e}")))
    }

    /// Check if the database is encrypted
    #[napi]
    pub fn is_encrypted(&self) -> bool {
        self.db.is_encrypted()
    }

    // ═══════════════════════════════════════════════════════
    // SQL Operations
    // ═══════════════════════════════════════════════════════

    /// Execute a SQL statement (SELECT/INSERT/UPDATE/DELETE)
    #[napi]
    pub fn execute_sql(&self, sql: String) -> Result<u32> {
        self.db
            .execute_sql(&sql)
            .map(|batches| batches.iter().map(|b| b.num_rows()).sum::<usize>() as u32)
            .map_err(|e| Error::from_reason(format!("SQL execution failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // Index Operations
    // ═══════════════════════════════════════════════════════

    /// Create an index on a table column
    #[napi]
    pub fn create_index(&self, table: String, column: String) -> Result<()> {
        self.db
            .create_index(&table, &column)
            .map_err(|e| Error::from_reason(format!("Create index failed: {e}")))
    }

    /// Drop an index from a table column
    #[napi]
    pub fn drop_index(&self, table: String, column: String) -> Result<()> {
        self.db
            .drop_index(&table, &column)
            .map_err(|e| Error::from_reason(format!("Drop index failed: {e}")))
    }

    /// Check if an index exists on a table column
    #[napi]
    pub fn has_index(&self, table: String, column: String) -> bool {
        self.db.has_index(&table, &column)
    }

    // ═══════════════════════════════════════════════════════
    // Snapshot Operations
    // ═══════════════════════════════════════════════════════

    /// Save the in-memory database to a file
    #[napi]
    pub fn save_to_file(&self, path: String) -> Result<()> {
        self.db
            .save_to_file(&path)
            .map_err(|e| Error::from_reason(format!("Save failed: {e}")))
    }

    /// Load a database from a file into memory
    #[napi(factory)]
    pub fn load_from_file(path: String) -> Result<Self> {
        let db = CoreDatabase::load_from_file(&path)
            .map_err(|e| Error::from_reason(format!("Load failed: {e}")))?;
        Ok(Database { db: Arc::new(db) })
    }

    // ═══════════════════════════════════════════════════════
    // MVCC Operations
    // ═══════════════════════════════════════════════════════

    /// Get the current MVCC timestamp
    #[napi]
    pub fn current_timestamp(&self) -> u32 {
        self.db.current_timestamp() as u32
    }

    /// Allocate a new commit timestamp
    #[napi]
    pub fn allocate_commit_ts(&self) -> u32 {
        self.db.allocate_commit_ts() as u32
    }

    /// Insert a versioned key-value pair (MVCC)
    #[napi]
    pub fn insert_versioned(
        &self,
        table: String,
        key: Buffer,
        value: Buffer,
        commit_ts: u32,
    ) -> Result<()> {
        self.db
            .insert_versioned(&table, &key, Some(&value[..]), commit_ts as u64)
            .map_err(|e| Error::from_reason(format!("Versioned insert failed: {e}")))
    }

    /// Read a specific version of a key (Snapshot Read)
    #[napi]
    pub fn get_snapshot(&self, table: String, key: Buffer, read_ts: u32) -> Result<Option<Buffer>> {
        match self.db.get_snapshot(&table, &key, read_ts as u64) {
            Ok(Some(Some(value))) => Ok(Some(value.into())),
            Ok(Some(None)) | Ok(None) => Ok(None),
            Err(e) => Err(Error::from_reason(format!("Snapshot read failed: {e}"))),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Transaction & Close
    // ═══════════════════════════════════════════════════════

    /// Begin a new transaction
    #[napi]
    pub fn begin_transaction(&self) -> Transaction {
        Transaction {
            db: Arc::clone(&self.db),
            operations: Vec::new(),
        }
    }

    /// Close the database
    #[napi]
    pub fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Transaction handle
#[napi]
pub struct Transaction {
    db: Arc<CoreDatabase>,
    operations: Vec<TxOperation>,
}

enum TxOperation {
    Insert {
        table: String,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Delete {
        table: String,
        key: Vec<u8>,
    },
}

#[napi]
impl Transaction {
    /// Insert a key-value pair (buffered)
    #[napi]
    pub fn insert(&mut self, table: String, key: Buffer, value: Buffer) -> Result<()> {
        self.operations.push(TxOperation::Insert {
            table,
            key: key.to_vec(),
            value: value.to_vec(),
        });
        Ok(())
    }

    /// Delete a key (buffered)
    #[napi]
    pub fn delete(&mut self, table: String, key: Buffer) -> Result<()> {
        self.operations.push(TxOperation::Delete {
            table,
            key: key.to_vec(),
        });
        Ok(())
    }

    /// Execute SQL statement (INSERT/UPDATE/DELETE)
    #[napi]
    pub fn execute(&self, sql: String) -> Result<u32> {
        self.db
            .execute_sql(&sql)
            .map(|batches| batches.iter().map(|b| b.num_rows()).sum::<usize>() as u32)
            .map_err(|e| Error::from_reason(format!("SQL execution failed: {e}")))
    }

    /// Commit the transaction (batch processing)
    #[napi]
    pub fn commit(&mut self) -> Result<()> {
        type InsertBatch = std::collections::HashMap<String, Vec<(Vec<u8>, Vec<u8>)>>;
        let mut insert_batches: InsertBatch = std::collections::HashMap::new();
        let mut deletes: Vec<(String, Vec<u8>)> = Vec::new();

        for op in self.operations.drain(..) {
            match op {
                TxOperation::Insert { table, key, value } => {
                    insert_batches.entry(table).or_default().push((key, value));
                }
                TxOperation::Delete { table, key } => {
                    deletes.push((table, key));
                }
            }
        }

        for (table, rows) in insert_batches {
            self.db
                .insert_batch(&table, rows)
                .map_err(|e| Error::from_reason(format!("Batch insert failed: {e}")))?;
        }

        for (table, key) in deletes {
            self.db
                .delete(&table, &key)
                .map(|_| ())
                .map_err(|e| Error::from_reason(format!("Delete failed: {e}")))?;
        }

        Ok(())
    }
}
