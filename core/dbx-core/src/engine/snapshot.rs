//! Database snapshot for save/load functionality
//!
//! Provides serialization/deserialization of entire database state

use crate::engine::metadata::SchemaMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete database snapshot for serialization
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseSnapshot {
    /// Version for future compatibility
    pub version: u32,

    /// Table schemas
    pub schemas: HashMap<String, SchemaMetadata>,

    /// Index registry: index_name → (table, column)
    pub indexes: HashMap<String, (String, String)>,

    /// All table data
    pub tables: HashMap<String, TableData>,

    /// Row counters: table_name → next_row_id
    pub row_counters: HashMap<String, usize>,
}

/// Data for a single table
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TableData {
    /// All key-value pairs in the table
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
}

impl DatabaseSnapshot {
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty snapshot
    pub fn new() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            schemas: HashMap::new(),
            indexes: HashMap::new(),
            tables: HashMap::new(),
            row_counters: HashMap::new(),
        }
    }
}

impl Default for DatabaseSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serialization() {
        let mut snapshot = DatabaseSnapshot::new();
        snapshot.row_counters.insert("users".to_string(), 42);

        // Serialize to JSON
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"version\":1"));

        // Deserialize back
        let restored: DatabaseSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version, DatabaseSnapshot::CURRENT_VERSION);
        assert_eq!(restored.row_counters.get("users"), Some(&42));
    }

    #[test]
    fn test_table_data_serialization() {
        let table_data = TableData {
            entries: vec![
                (b"key1".to_vec(), b"value1".to_vec()),
                (b"key2".to_vec(), b"value2".to_vec()),
            ],
        };

        let json = serde_json::to_string(&table_data).unwrap();
        let restored: TableData = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.entries.len(), 2);
        assert_eq!(restored.entries[0].0, b"key1");
        assert_eq!(restored.entries[0].1, b"value1");
    }
}
