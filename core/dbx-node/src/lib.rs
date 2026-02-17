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
        Ok(Database { db })
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
    // DDL API Operations
    // ═══════════════════════════════════════════════════════

    /// Drop a table
    #[napi]
    pub fn drop_table(&self, table_name: String) -> Result<()> {
        self.db
            .drop_table(&table_name)
            .map_err(|e| Error::from_reason(format!("Drop table failed: {e}")))
    }

    /// Check if a table exists
    #[napi]
    pub fn table_exists(&self, table_name: String) -> bool {
        self.db.table_exists(&table_name)
    }

    /// List all tables
    #[napi]
    pub fn list_tables(&self) -> Vec<String> {
        self.db.list_tables()
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

// ═══════════════════════════════════════════════════════════════
// Zero-Copy Result Types
// ═══════════════════════════════════════════════════════════════

/// Zero-copy scan result - Rust owns the data, JavaScript gets read-only access
#[napi]
pub struct ScanResult {
    // Rust owns the serialized data
    data: Vec<u8>,
    // Metadata for parsing
    count: usize,
}

#[napi]
impl ScanResult {
    /// Get the raw data as a Buffer (zero-copy, read-only)
    #[napi]
    pub fn as_buffer(&self) -> Buffer {
        // Return a reference to the data as Buffer (no copy)
        Buffer::from(&self.data[..])
    }

    /// Get the number of key-value pairs
    #[napi]
    pub fn count(&self) -> u32 {
        self.count as u32
    }

    /// Parse the data into key-value pairs (fallback, requires copy)
    #[napi]
    pub fn to_pairs(&self) -> Result<Vec<Vec<Buffer>>> {
        // Deserialize from the flat buffer
        let mut offset = 0;
        let mut pairs = Vec::with_capacity(self.count);

        for _ in 0..self.count {
            // Read key length
            if offset + 4 > self.data.len() {
                return Err(Error::from_reason("Invalid data format"));
            }
            let key_len = u32::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]) as usize;
            offset += 4;

            // Read key
            if offset + key_len > self.data.len() {
                return Err(Error::from_reason("Invalid data format"));
            }
            let key = Buffer::from(&self.data[offset..offset + key_len]);
            offset += key_len;

            // Read value length
            if offset + 4 > self.data.len() {
                return Err(Error::from_reason("Invalid data format"));
            }
            let value_len = u32::from_le_bytes([
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            ]) as usize;
            offset += 4;

            // Read value
            if offset + value_len > self.data.len() {
                return Err(Error::from_reason("Invalid data format"));
            }
            let value = Buffer::from(&self.data[offset..offset + value_len]);
            offset += value_len;

            pairs.push(vec![key, value]);
        }

        Ok(pairs)
    }
}

#[napi]
impl Database {
    /// Zero-copy scan - returns ScanResult that owns the data
    #[napi]
    pub fn scan_zero_copy(&self, table: String) -> Result<ScanResult> {
        let entries = self
            .db
            .scan(&table)
            .map_err(|e| Error::from_reason(format!("Scan failed: {e}")))?;

        // Serialize into a flat buffer
        let mut data = Vec::new();
        let count = entries.len();

        for (key, value) in entries {
            // Write key length + key
            data.extend_from_slice(&(key.len() as u32).to_le_bytes());
            data.extend_from_slice(&key);

            // Write value length + value
            data.extend_from_slice(&(value.len() as u32).to_le_bytes());
            data.extend_from_slice(&value);
        }

        Ok(ScanResult { data, count })
    }

    /// Batch get - get multiple keys at once (reduces FFI overhead)
    #[napi]
    pub fn get_batch(&self, table: String, keys: Vec<Buffer>) -> Result<Vec<Option<Buffer>>> {
        let mut results = Vec::with_capacity(keys.len());

        for key in keys {
            match self.db.get(&table, &key) {
                Ok(Some(value)) => results.push(Some(value.into())),
                Ok(None) => results.push(None),
                Err(e) => return Err(Error::from_reason(format!("Get failed: {e}"))),
            }
        }

        Ok(results)
    }
}
