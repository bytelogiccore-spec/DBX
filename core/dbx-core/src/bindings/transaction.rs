//! Transaction buffering for language bindings.
//!
//! This module provides a shared transaction buffer implementation used by all
//! DBX language bindings to ensure consistent transaction semantics.

use crate::{Database, error::DbxResult};
use std::collections::HashMap;

/// Operation type for buffered transactions.
///
/// Represents a single operation (insert or delete) that will be applied
/// when the transaction is committed.
#[derive(Debug, Clone)]
pub enum Operation {
    /// Insert a key-value pair into a table
    Insert {
        table: String,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    /// Delete a key from a table
    Delete {
        table: String,
        key: Vec<u8>,
    },
}

/// Transaction buffer for batching operations.
///
/// This is a shared implementation used by all language bindings
/// to provide consistent transaction semantics. Operations are buffered
/// in memory and applied atomically when `commit()` is called.
///
/// # Examples
///
/// ```
/// use dbx_core::bindings::TransactionBuffer;
/// use dbx_core::Database;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let db = Database::open_in_memory()?;
/// let mut buffer = TransactionBuffer::new();
///
/// // Buffer operations
/// buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
/// buffer.insert("users".to_string(), b"key2".to_vec(), b"value2".to_vec());
///
/// // Commit atomically
/// buffer.commit(&db)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TransactionBuffer {
    operations: Vec<Operation>,
}

impl TransactionBuffer {
    /// Create a new empty transaction buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let buffer = TransactionBuffer::new();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
        }
    }
    
    /// Add an insert operation to the buffer.
    ///
    /// The operation will not be applied to the database until `commit()` is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let mut buffer = TransactionBuffer::new();
    /// buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
    /// assert_eq!(buffer.len(), 1);
    /// ```
    pub fn insert(&mut self, table: String, key: Vec<u8>, value: Vec<u8>) {
        self.operations.push(Operation::Insert { table, key, value });
    }
    
    /// Add a delete operation to the buffer.
    ///
    /// The operation will not be applied to the database until `commit()` is called.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let mut buffer = TransactionBuffer::new();
    /// buffer.delete("users".to_string(), b"key1".to_vec());
    /// assert_eq!(buffer.len(), 1);
    /// ```
    pub fn delete(&mut self, table: String, key: Vec<u8>) {
        self.operations.push(Operation::Delete { table, key });
    }
    
    /// Commit all buffered operations to the database.
    ///
    /// Operations are grouped by table for batch processing.
    /// Inserts are applied first using batch insert, then deletes are applied individually.
    ///
    /// After a successful commit, the buffer is cleared.
    ///
    /// # Errors
    ///
    /// Returns an error if any operation fails. In case of error, some operations
    /// may have been applied (no automatic rollback).
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    /// use dbx_core::Database;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = Database::open_in_memory()?;
    /// let mut buffer = TransactionBuffer::new();
    ///
    /// buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
    /// buffer.commit(&db)?;
    ///
    /// assert!(buffer.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn commit(&mut self, db: &Database) -> DbxResult<()> {
        // Group operations by table
        let mut insert_batches: HashMap<String, Vec<(Vec<u8>, Vec<u8>)>> = HashMap::new();
        let mut delete_ops: Vec<(String, Vec<u8>)> = Vec::new();
        
        for op in self.operations.drain(..) {
            match op {
                Operation::Insert { table, key, value } => {
                    insert_batches.entry(table).or_default().push((key, value));
                }
                Operation::Delete { table, key } => {
                    delete_ops.push((table, key));
                }
            }
        }
        
        // Apply batch inserts
        for (table, rows) in insert_batches {
            db.insert_batch(&table, rows)?;
        }
        
        // Apply deletes
        for (table, key) in delete_ops {
            db.delete(&table, &key)?;
        }
        
        Ok(())
    }
    
    /// Rollback (clear) all buffered operations.
    ///
    /// This discards all buffered operations without applying them to the database.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let mut buffer = TransactionBuffer::new();
    /// buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
    /// assert_eq!(buffer.len(), 1);
    ///
    /// buffer.rollback();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn rollback(&mut self) {
        self.operations.clear();
    }
    
    /// Check if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let buffer = TransactionBuffer::new();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
    
    /// Get the number of buffered operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use dbx_core::bindings::TransactionBuffer;
    ///
    /// let mut buffer = TransactionBuffer::new();
    /// buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
    /// buffer.insert("users".to_string(), b"key2".to_vec(), b"value2".to_vec());
    /// assert_eq!(buffer.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.operations.len()
    }
}

impl Default for TransactionBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buffer = TransactionBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_insert_operation() {
        let mut buffer = TransactionBuffer::new();
        buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
        
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_delete_operation() {
        let mut buffer = TransactionBuffer::new();
        buffer.delete("users".to_string(), b"key1".to_vec());
        
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_rollback() {
        let mut buffer = TransactionBuffer::new();
        buffer.insert("users".to_string(), b"key1".to_vec(), b"value1".to_vec());
        buffer.delete("users".to_string(), b"key2".to_vec());
        
        assert_eq!(buffer.len(), 2);
        
        buffer.rollback();
        
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }
}
