//! DBX PyO3 Native Python Bindings
//!
//! High-performance native Python bindings using PyO3.

#![allow(clippy::useless_conversion)]

use dbx_core::Database as CoreDatabase;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

/// Python Database class
#[pyclass]
struct Database {
    db: CoreDatabase,
}

#[pymethods]
impl Database {
    // ═══════════════════════════════════════════════════════
    // Constructors
    // ═══════════════════════════════════════════════════════

    /// Open an in-memory database
    #[staticmethod]
    fn open_in_memory() -> PyResult<Self> {
        CoreDatabase::open_in_memory()
            .map(|db| Database { db })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to open database: {e}")))
    }

    /// Open a database at the given path
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        CoreDatabase::open(std::path::Path::new(path))
            .map(|db| Database { db })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to open database: {e}")))
    }

    /// Load a database from a snapshot file
    #[staticmethod]
    fn load_from_file(path: &str) -> PyResult<Self> {
        CoreDatabase::load_from_file(path)
            .map(|db| Database { db })
            .map_err(|e| PyRuntimeError::new_err(format!("Load failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════════

    /// Insert a key-value pair into a table
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> PyResult<()> {
        self.db
            .insert(table, key, value)
            .map_err(|e| PyRuntimeError::new_err(format!("Insert failed: {e}")))
    }

    /// Get a value by key from a table
    fn get<'py>(
        &self,
        py: Python<'py>,
        table: &str,
        key: &[u8],
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match self.db.get(table, key) {
            Ok(Some(value)) => Ok(Some(PyBytes::new_bound(py, &value))),
            Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(format!("Get failed: {e}"))),
        }
    }

    /// Delete a key from a table
    fn delete(&self, table: &str, key: &[u8]) -> PyResult<()> {
        self.db
            .delete(table, key)
            .map(|_| ())
            .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // Batch Operations
    // ═══════════════════════════════════════════════════════

    /// Insert multiple key-value pairs at once
    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> PyResult<()> {
        self.db
            .insert_batch(table, rows)
            .map_err(|e| PyRuntimeError::new_err(format!("Batch insert failed: {e}")))
    }

    /// Scan all key-value pairs in a table
    fn scan<'py>(
        &self,
        py: Python<'py>,
        table: &str,
    ) -> PyResult<Vec<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)>> {
        let entries = self
            .db
            .scan(table)
            .map_err(|e| PyRuntimeError::new_err(format!("Scan failed: {e}")))?;
        Ok(entries
            .into_iter()
            .map(|(k, v)| (PyBytes::new_bound(py, &k), PyBytes::new_bound(py, &v)))
            .collect())
    }

    /// Scan a range of keys [start_key, end_key)
    fn range<'py>(
        &self,
        py: Python<'py>,
        table: &str,
        start_key: &[u8],
        end_key: &[u8],
    ) -> PyResult<Vec<(Bound<'py, PyBytes>, Bound<'py, PyBytes>)>> {
        let entries = self
            .db
            .range(table, start_key, end_key)
            .map_err(|e| PyRuntimeError::new_err(format!("Range scan failed: {e}")))?;
        Ok(entries
            .into_iter()
            .map(|(k, v)| (PyBytes::new_bound(py, &k), PyBytes::new_bound(py, &v)))
            .collect())
    }

    // ═══════════════════════════════════════════════════════
    // Utility Operations
    // ═══════════════════════════════════════════════════════

    /// Count the number of rows in a table
    fn count(&self, table: &str) -> PyResult<usize> {
        self.db
            .count(table)
            .map_err(|e| PyRuntimeError::new_err(format!("Count failed: {e}")))
    }

    /// Flush the database to disk
    fn flush(&self) -> PyResult<()> {
        self.db
            .flush()
            .map_err(|e| PyRuntimeError::new_err(format!("Flush failed: {e}")))
    }

    /// Get all table names
    fn table_names(&self) -> PyResult<Vec<String>> {
        self.db
            .table_names()
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get table names: {e}")))
    }

    /// Run garbage collection (MVCC version cleanup)
    fn gc(&self) -> PyResult<usize> {
        self.db
            .gc()
            .map_err(|e| PyRuntimeError::new_err(format!("GC failed: {e}")))
    }

    /// Check if the database is encrypted
    fn is_encrypted(&self) -> bool {
        self.db.is_encrypted()
    }

    // ═══════════════════════════════════════════════════════
    // SQL Operations
    // ═══════════════════════════════════════════════════════

    /// Execute a SQL statement (SELECT/INSERT/UPDATE/DELETE)
    fn execute_sql(&self, sql: &str) -> PyResult<usize> {
        self.db
            .execute_sql(sql)
            .map(|batches| batches.iter().map(|b| b.num_rows()).sum::<usize>())
            .map_err(|e| PyRuntimeError::new_err(format!("SQL execution failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // Index Operations
    // ═══════════════════════════════════════════════════════

    /// Create an index on a table column
    fn create_index(&self, table: &str, column: &str) -> PyResult<()> {
        self.db
            .create_index(table, column)
            .map_err(|e| PyRuntimeError::new_err(format!("Create index failed: {e}")))
    }

    /// Drop an index from a table column
    fn drop_index(&self, table: &str, column: &str) -> PyResult<()> {
        self.db
            .drop_index(table, column)
            .map_err(|e| PyRuntimeError::new_err(format!("Drop index failed: {e}")))
    }

    /// Check if an index exists on a table column
    fn has_index(&self, table: &str, column: &str) -> bool {
        self.db.has_index(table, column)
    }

    // ═══════════════════════════════════════════════════════
    // Snapshot Operations
    // ═══════════════════════════════════════════════════════

    /// Save the in-memory database to a file
    fn save_to_file(&self, path: &str) -> PyResult<()> {
        self.db
            .save_to_file(path)
            .map_err(|e| PyRuntimeError::new_err(format!("Save failed: {e}")))
    }

    // ═══════════════════════════════════════════════════════
    // MVCC Operations
    // ═══════════════════════════════════════════════════════

    /// Get the current MVCC timestamp
    fn current_timestamp(&self) -> u64 {
        self.db.current_timestamp()
    }

    /// Allocate a new commit timestamp
    fn allocate_commit_ts(&self) -> u64 {
        self.db.allocate_commit_ts()
    }

    /// Insert a versioned key-value pair (MVCC)
    fn insert_versioned(
        &self,
        table: &str,
        key: &[u8],
        value: &[u8],
        commit_ts: u64,
    ) -> PyResult<()> {
        self.db
            .insert_versioned(table, key, Some(value), commit_ts)
            .map_err(|e| PyRuntimeError::new_err(format!("Versioned insert failed: {e}")))
    }

    /// Read a specific version of a key (Snapshot Read)
    fn get_snapshot<'py>(
        &self,
        py: Python<'py>,
        table: &str,
        key: &[u8],
        read_ts: u64,
    ) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match self.db.get_snapshot(table, key, read_ts) {
            Ok(Some(Some(value))) => Ok(Some(PyBytes::new_bound(py, &value))),
            Ok(Some(None)) | Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(format!(
                "Snapshot read failed: {e}"
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Transaction & Close
    // ═══════════════════════════════════════════════════════

    /// Begin a transaction
    fn begin_transaction(slf: PyRef<'_, Self>) -> PyResult<Transaction> {
        Ok(Transaction {
            db: slf.into(),
            operations: Vec::new(),
        })
    }

    /// Close the database
    fn close(&self) -> PyResult<()> {
        Ok(())
    }
}

/// Transaction operation
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

/// Python Transaction class
#[pyclass]
struct Transaction {
    db: Py<Database>,
    operations: Vec<TxOperation>,
}

#[pymethods]
impl Transaction {
    /// Insert a key-value pair (buffered)
    fn insert(&mut self, table: &str, key: &[u8], value: &[u8]) {
        self.operations.push(TxOperation::Insert {
            table: table.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
        });
    }

    /// Delete a key (buffered)
    fn delete(&mut self, table: &str, key: &[u8]) {
        self.operations.push(TxOperation::Delete {
            table: table.to_string(),
            key: key.to_vec(),
        });
    }

    /// Commit the transaction
    fn commit(&mut self, py: Python) -> PyResult<()> {
        let db = self.db.borrow(py);

        type InsertBatch = std::collections::HashMap<String, Vec<(Vec<u8>, Vec<u8>)>>;
        let mut insert_batches: InsertBatch = std::collections::HashMap::new();
        let mut delete_ops: Vec<(String, Vec<u8>)> = Vec::new();

        for op in self.operations.drain(..) {
            match op {
                TxOperation::Insert { table, key, value } => {
                    insert_batches.entry(table).or_default().push((key, value));
                }
                TxOperation::Delete { table, key } => {
                    delete_ops.push((table, key));
                }
            }
        }

        for (table, rows) in insert_batches {
            db.db
                .insert_batch(&table, rows)
                .map_err(|e| PyRuntimeError::new_err(format!("Batch insert failed: {e}")))?;
        }

        for (table, key) in delete_ops {
            db.db
                .delete(&table, &key)
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {e}")))?;
        }

        Ok(())
    }

    /// Rollback the transaction
    fn rollback(&mut self) {
        self.operations.clear();
    }
}

/// PyO3 module definition
#[pymodule]
fn dbx_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Database>()?;
    m.add_class::<Transaction>()?;
    Ok(())
}
