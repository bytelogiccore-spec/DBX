//! DBX PyO3 Native Python Bindings
//!
//! High-performance native Python bindings using PyO3.

use pyo3::prelude::*;
use pyo3::exceptions::PyRuntimeError;
use pyo3::types::PyBytes;
use dbx_core::Database as CoreDatabase;

/// Python Database class
#[pyclass]
struct Database {
    db: CoreDatabase,
}

#[pymethods]
impl Database {
    /// Open an in-memory database
    #[staticmethod]
    fn open_in_memory() -> PyResult<Self> {
        CoreDatabase::open_in_memory()
            .map(|db| Database { db })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to open database: {}", e)))
    }
    
    /// Open a database at the given path
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        CoreDatabase::open(std::path::Path::new(path))
            .map(|db| Database { db })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to open database: {}", e)))
    }
    
    /// Insert a key-value pair into a table
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> PyResult<()> {
        self.db.insert(table, key, value)
            .map_err(|e| PyRuntimeError::new_err(format!("Insert failed: {}", e)))
    }
    
    /// Get a value by key from a table
    fn get<'py>(&self, py: Python<'py>, table: &str, key: &[u8]) -> PyResult<Option<Bound<'py, PyBytes>>> {
        match self.db.get(table, key) {
            Ok(Some(value)) => Ok(Some(PyBytes::new_bound(py, &value))),
            Ok(None) => Ok(None),
            Err(e) => Err(PyRuntimeError::new_err(format!("Get failed: {}", e))),
        }
    }
    
    /// Delete a key from a table
    fn delete(&self, table: &str, key: &[u8]) -> PyResult<()> {
        self.db.delete(table, key)
            .map(|_| ())
            .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))
    }
    
    /// Begin a transaction
    fn begin_transaction(slf: PyRef<'_, Self>) -> PyResult<Transaction> {
        Ok(Transaction {
            db: slf.into(),
            operations: Vec::new(),
        })
    }
    
    /// Close the database
    fn close(&self) -> PyResult<()> {
        // Database will be closed when dropped
        Ok(())
    }
}

/// Transaction operation
enum TxOperation {
    Insert { table: String, key: Vec<u8>, value: Vec<u8> },
    Delete { table: String, key: Vec<u8> },
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
        
        // Group operations by table for batch processing
        let mut insert_batches: std::collections::HashMap<String, Vec<(Vec<u8>, Vec<u8>)>> = 
            std::collections::HashMap::new();
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
        
        // Apply batch inserts
        for (table, rows) in insert_batches {
            db.db.insert_batch(&table, rows)
                .map_err(|e| PyRuntimeError::new_err(format!("Batch insert failed: {}", e)))?;
        }
        
        // Apply deletes
        for (table, key) in delete_ops {
            db.db.delete(&table, &key)
                .map_err(|e| PyRuntimeError::new_err(format!("Delete failed: {}", e)))?;
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
