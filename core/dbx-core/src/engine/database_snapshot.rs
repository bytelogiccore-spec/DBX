//! Database snapshot save/load implementation

use crate::engine::metadata::SchemaMetadata;
use crate::engine::snapshot::{DatabaseSnapshot, TableData};
use crate::engine::{Database, WosVariant};
use crate::error::{DbxError, DbxResult};
use arrow::datatypes::Schema;
use std::path::Path;
use std::sync::Arc;

impl Database {
    /// Save in-memory database to file
    ///
    /// Only works for in-memory databases. Returns error for file-based DBs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.execute_sql("CREATE TABLE users (id INT, name TEXT)")?;
    /// db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")?;
    ///
    /// // Save to file
    /// db.save_to_file("backup.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> DbxResult<()> {
        // 1. Check if this is an in-memory DB
        if !self.is_in_memory() {
            return Err(DbxError::InvalidOperation {
                message: "save_to_file only works for in-memory databases".to_string(),
                context: "Use flush() for file-based databases".to_string(),
            });
        }

        // 2. Create snapshot
        let snapshot = self.create_snapshot()?;

        // 3. Serialize to JSON
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| DbxError::Serialization(e.to_string()))?;

        // 4. Write to file
        std::fs::write(path, json)?;

        Ok(())
    }

    /// Load database from file into in-memory database
    ///
    /// Creates a new in-memory DB and loads all data from file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// // Load from file
    /// let db = Database::load_from_file("backup.json")?;
    ///
    /// // Query data
    /// let results = db.execute_sql("SELECT * FROM users")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> DbxResult<Self> {
        // 1. Read file
        let json = std::fs::read_to_string(path)?;

        // 2. Deserialize snapshot
        let snapshot: DatabaseSnapshot =
            serde_json::from_str(&json).map_err(|e| DbxError::Serialization(e.to_string()))?;

        // 3. Create new in-memory DB
        let db = Self::open_in_memory()?;

        // 4. Restore snapshot
        db.restore_snapshot(snapshot)?;

        Ok(db)
    }

    /// Check if this is an in-memory database
    fn is_in_memory(&self) -> bool {
        matches!(self.wos, WosVariant::InMemory(_))
    }

    /// Create a snapshot of the current database state
    fn create_snapshot(&self) -> DbxResult<DatabaseSnapshot> {
        let mut snapshot = DatabaseSnapshot::new();

        // 1. Capture schemas
        let schemas = self.table_schemas.read().unwrap();
        for (table_name, schema) in schemas.iter() {
            let metadata = SchemaMetadata::from(schema.as_ref());
            snapshot.schemas.insert(table_name.clone(), metadata);
        }
        drop(schemas);

        // 2. Capture indexes
        let indexes = self.index_registry.read().unwrap();
        snapshot.indexes = indexes.clone();
        drop(indexes);

        // 3. Capture table data
        // Use row_counters to get table list (more reliable than WOS table_names for in-memory)
        let table_list: Vec<String> = self
            .row_counters
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for table_name in table_list {
            // Skip metadata tables
            if table_name.starts_with("__meta__") {
                continue;
            }

            let entries = self.wos.scan(&table_name, ..)?;
            snapshot.tables.insert(table_name, TableData { entries });
        }

        // 4. Capture row counters
        for entry in self.row_counters.iter() {
            let table = entry.key().clone();
            let counter = entry.value().load(std::sync::atomic::Ordering::SeqCst);
            snapshot.row_counters.insert(table, counter);
        }

        Ok(snapshot)
    }

    /// Restore database state from snapshot
    fn restore_snapshot(&self, snapshot: DatabaseSnapshot) -> DbxResult<()> {
        // 1. Validate version
        if snapshot.version != DatabaseSnapshot::CURRENT_VERSION {
            return Err(DbxError::InvalidOperation {
                message: format!("Unsupported snapshot version: {}", snapshot.version),
                context: format!("Expected version {}", DatabaseSnapshot::CURRENT_VERSION),
            });
        }

        // 2. Restore schemas (both table_schemas and schemas for compatibility)
        let mut table_schemas = self.table_schemas.write().unwrap();
        let mut schemas = self.schemas.write().unwrap();
        for (table_name, metadata) in snapshot.schemas {
            let schema = Arc::new(
                Schema::try_from(metadata)
                    .map_err(|e| DbxError::Schema(format!("Failed to restore schema: {}", e)))?,
            );
            table_schemas.insert(table_name.clone(), schema.clone());
            schemas.insert(table_name, schema);
        }
        drop(table_schemas);
        drop(schemas);

        // 3. Restore indexes
        let mut indexes = self.index_registry.write().unwrap();
        *indexes = snapshot.indexes;
        drop(indexes);

        // 4. Restore table data
        for (table_name, table_data) in snapshot.tables {
            for (key, value) in table_data.entries {
                self.wos.insert(&table_name, &key, &value)?;
            }
        }

        // 5. Restore row counters
        for (table, count) in snapshot.row_counters {
            self.row_counters
                .insert(table, std::sync::atomic::AtomicUsize::new(count));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_in_memory() {
        let db = Database::open_in_memory().unwrap();
        assert!(db.is_in_memory());
    }

    #[test]
    fn test_file_based_db_rejects_save() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db = Database::open(temp_dir.path()).unwrap();

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let result = db.save_to_file(temp_file.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("in-memory"));
    }
}
