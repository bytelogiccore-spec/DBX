//! Hash Index for fast key lookups
//!
//! Provides O(1) key lookup performance for indexed columns.

use crate::error::{DbxError, DbxResult};
use ahash::AHashMap;
use dashmap::DashMap;
use std::sync::RwLock;

/// Type alias for column-level index: column_name → RwLock<value → row_ids>
type ColumnIndex = DashMap<String, RwLock<AHashMap<Vec<u8>, Vec<usize>>>>;

/// Hash Index structure
///
/// Maintains indexes for fast key lookups.
/// Structure: table_name → (column_name → HashMap<value, Vec<row_id>>)
pub struct HashIndex {
    /// Indexes organized by table and column
    indexes: DashMap<String, ColumnIndex>,
}

impl HashIndex {
    /// Create a new empty hash index
    pub fn new() -> Self {
        Self {
            indexes: DashMap::new(),
        }
    }

    /// Create an index on a specific column
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name to index
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::index::HashIndex;
    /// let index = HashIndex::new();
    /// index.create_index("users", "id").unwrap();
    /// ```
    pub fn create_index(&self, table: &str, column: &str) -> DbxResult<()> {
        let table_indexes = self.indexes.entry(table.to_string()).or_default();

        if table_indexes.contains_key(column) {
            return Err(DbxError::IndexAlreadyExists {
                table: table.to_string(),
                column: column.to_string(),
            });
        }

        table_indexes.insert(column.to_string(), RwLock::new(AHashMap::new()));
        Ok(())
    }

    /// Drop an index
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name
    pub fn drop_index(&self, table: &str, column: &str) -> DbxResult<()> {
        if let Some(table_indexes) = self.indexes.get(table) {
            if table_indexes.remove(column).is_none() {
                return Err(DbxError::IndexNotFound {
                    table: table.to_string(),
                    column: column.to_string(),
                });
            }
            Ok(())
        } else {
            Err(DbxError::IndexNotFound {
                table: table.to_string(),
                column: column.to_string(),
            })
        }
    }

    /// Lookup row IDs by indexed value
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name
    /// * `value` - Value to lookup
    ///
    /// # Returns
    ///
    /// Vector of row IDs that match the value
    pub fn lookup(&self, table: &str, column: &str, value: &[u8]) -> DbxResult<Vec<usize>> {
        if let Some(table_indexes) = self.indexes.get(table) {
            if let Some(index) = table_indexes.get(column) {
                let index_read = index.read().unwrap();
                Ok(index_read.get(value).cloned().unwrap_or_default())
            } else {
                Err(DbxError::IndexNotFound {
                    table: table.to_string(),
                    column: column.to_string(),
                })
            }
        } else {
            Err(DbxError::IndexNotFound {
                table: table.to_string(),
                column: column.to_string(),
            })
        }
    }

    /// Update index on insert
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name
    /// * `value` - Value being inserted
    /// * `row_id` - Row ID of the inserted row
    pub fn update_on_insert(
        &self,
        table: &str,
        column: &str,
        value: &[u8],
        row_id: usize,
    ) -> DbxResult<()> {
        if let Some(table_indexes) = self.indexes.get(table)
            && let Some(index) = table_indexes.get(column)
        {
            let mut index_write = index.write().unwrap();
            index_write.entry(value.to_vec()).or_default().push(row_id);
        }
        Ok(())
    }

    /// Update index on delete
    ///
    /// # Arguments
    ///
    /// * `table` - Table name
    /// * `column` - Column name
    /// * `value` - Value being deleted
    /// * `row_id` - Row ID of the deleted row
    pub fn update_on_delete(
        &self,
        table: &str,
        column: &str,
        value: &[u8],
        row_id: usize,
    ) -> DbxResult<()> {
        if let Some(table_indexes) = self.indexes.get(table)
            && let Some(index) = table_indexes.get(column)
        {
            let mut index_write = index.write().unwrap();
            if let Some(row_ids) = index_write.get_mut(value) {
                row_ids.retain(|&id| id != row_id);
                if row_ids.is_empty() {
                    index_write.remove(value);
                }
            }
        }
        Ok(())
    }

    /// Check if an index exists
    pub fn has_index(&self, table: &str, column: &str) -> bool {
        self.indexes
            .get(table)
            .map(|t| t.contains_key(column))
            .unwrap_or(false)
    }
}

impl Default for HashIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index() {
        let index = HashIndex::new();
        assert!(index.create_index("users", "id").is_ok());
        assert!(index.has_index("users", "id"));
    }

    #[test]
    fn test_create_duplicate_index() {
        let index = HashIndex::new();
        index.create_index("users", "id").unwrap();
        assert!(index.create_index("users", "id").is_err());
    }

    #[test]
    fn test_drop_index() {
        let index = HashIndex::new();
        index.create_index("users", "id").unwrap();
        assert!(index.drop_index("users", "id").is_ok());
        assert!(!index.has_index("users", "id"));
    }

    #[test]
    fn test_drop_nonexistent_index() {
        let index = HashIndex::new();
        assert!(index.drop_index("users", "id").is_err());
    }

    #[test]
    fn test_insert_and_lookup() {
        let index = HashIndex::new();
        index.create_index("users", "id").unwrap();

        let value = b"user:123";
        index.update_on_insert("users", "id", value, 0).unwrap();
        index.update_on_insert("users", "id", value, 1).unwrap();

        let result = index.lookup("users", "id", value).unwrap();
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn test_delete_and_lookup() {
        let index = HashIndex::new();
        index.create_index("users", "id").unwrap();

        let value = b"user:123";
        index.update_on_insert("users", "id", value, 0).unwrap();
        index.update_on_insert("users", "id", value, 1).unwrap();

        index.update_on_delete("users", "id", value, 0).unwrap();

        let result = index.lookup("users", "id", value).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_lookup_nonexistent() {
        let index = HashIndex::new();
        index.create_index("users", "id").unwrap();

        let result = index.lookup("users", "id", b"nonexistent").unwrap();
        assert!(result.is_empty());
    }
}
