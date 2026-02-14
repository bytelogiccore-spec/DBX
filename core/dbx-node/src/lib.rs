//! DBX Node.js Native Bindings using napi-rs

#![deny(clippy::all)]

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
            .map_err(|e| Error::from_reason(format!("Failed to open database: {}", e)))?;
        Ok(Database { db: Arc::new(db) })
    }

    /// Open a database at the given path
    #[napi(factory)]
    pub fn open(path: String) -> Result<Self> {
        let db = CoreDatabase::open(std::path::Path::new(&path))
            .map_err(|e| Error::from_reason(format!("Failed to open database: {}", e)))?;
        Ok(Database { db: Arc::new(db) })
    }

    /// Insert a key-value pair into a table
    #[napi]
    pub fn insert(&self, table: String, key: Buffer, value: Buffer) -> Result<()> {
        self.db
            .insert(&table, &key, &value)
            .map(|_| ())
            .map_err(|e| Error::from_reason(format!("Insert failed: {}", e)))
    }

    /// Get a value by key from a table
    #[napi]
    pub fn get(&self, table: String, key: Buffer) -> Result<Option<Buffer>> {
        match self.db.get(&table, &key) {
            Ok(Some(value)) => Ok(Some(value.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::from_reason(format!("Get failed: {}", e))),
        }
    }

    /// Delete a key from a table
    #[napi]
    pub fn delete(&self, table: String, key: Buffer) -> Result<()> {
        self.db
            .delete(&table, &key)
            .map(|_| ())
            .map_err(|e| Error::from_reason(format!("Delete failed: {}", e)))
    }

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
            .map_err(|e| Error::from_reason(format!("Batch insert failed: {}", e)))
    }

    /// Delete multiple keys at once (batch)
    #[napi]
    pub fn delete_batch(&self, table: String, keys: Vec<Buffer>) -> Result<()> {
        for key in keys {
            self.db
                .delete(&table, &key)
                .map(|_| ())
                .map_err(|e| Error::from_reason(format!("Delete failed: {}", e)))?;
        }
        Ok(())
    }

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
        // Database will be dropped automatically
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
        // Execute SQL directly on the database
        // Note: This bypasses the transaction buffer
        // For proper transactional SQL, use db.execute() within a transaction block
        self.db
            .execute_sql(&sql)
            .map(|batches| batches.iter().map(|b| b.num_rows()).sum::<usize>() as u32)
            .map_err(|e| Error::from_reason(format!("SQL execution failed: {}", e)))
    }

    /// Commit the transaction (batch processing)
    #[napi]
    pub fn commit(&mut self) -> Result<()> {
        use std::collections::HashMap;

        // Group operations by table for batch processing
        let mut insert_batches: HashMap<String, Vec<(Vec<u8>, Vec<u8>)>> = HashMap::new();
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

        // Apply batch inserts
        for (table, rows) in insert_batches {
            self.db
                .insert_batch(&table, rows)
                .map_err(|e| Error::from_reason(format!("Batch insert failed: {}", e)))?;
        }

        // Apply deletes
        for (table, key) in deletes {
            self.db
                .delete(&table, &key)
                .map(|_| ())
                .map_err(|e| Error::from_reason(format!("Delete failed: {}", e)))?;
        }

        Ok(())
    }
}
