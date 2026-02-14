//! Metadata Persistence — Schema and Index metadata serialization and storage
//!
//! This module provides functionality to persist table schemas and index definitions
//! to the sled backend, enabling automatic restoration on database reopen.

use crate::error::{DbxError, DbxResult};
use crate::storage::{StorageBackend, wos::WosBackend};
use arrow::datatypes::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

// ════════════════════════════════════════════
// Metadata Structures
// ════════════════════════════════════════════

/// Serializable schema metadata for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub table_name: String,
    pub fields: Vec<FieldMetadata>,
}

/// Serializable field metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

/// Serializable index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub index_name: String,
    pub table_name: String,
    pub column_name: String,
}

// ════════════════════════════════════════════
// Schema Conversion
// ════════════════════════════════════════════

impl From<&Schema> for SchemaMetadata {
    fn from(schema: &Schema) -> Self {
        let fields = schema
            .fields()
            .iter()
            .map(|field| FieldMetadata {
                name: field.name().clone(),
                data_type: datatype_to_string(field.data_type()),
                nullable: field.is_nullable(),
            })
            .collect();

        SchemaMetadata {
            table_name: String::new(), // Will be set by caller
            fields,
        }
    }
}

impl TryFrom<SchemaMetadata> for Schema {
    type Error = DbxError;

    fn try_from(metadata: SchemaMetadata) -> Result<Self, Self::Error> {
        let fields: Result<Vec<Field>, DbxError> = metadata
            .fields
            .iter()
            .map(|field_meta| {
                let data_type = string_to_datatype(&field_meta.data_type)?;
                Ok(Field::new(&field_meta.name, data_type, field_meta.nullable))
            })
            .collect();

        Ok(Schema::new(fields?))
    }
}

// ════════════════════════════════════════════
// DataType Conversion Helpers
// ════════════════════════════════════════════

/// Convert Arrow DataType to string representation
fn datatype_to_string(data_type: &DataType) -> String {
    match data_type {
        DataType::Int8 => "Int8".to_string(),
        DataType::Int16 => "Int16".to_string(),
        DataType::Int32 => "Int32".to_string(),
        DataType::Int64 => "Int64".to_string(),
        DataType::UInt8 => "UInt8".to_string(),
        DataType::UInt16 => "UInt16".to_string(),
        DataType::UInt32 => "UInt32".to_string(),
        DataType::UInt64 => "UInt64".to_string(),
        DataType::Float32 => "Float32".to_string(),
        DataType::Float64 => "Float64".to_string(),
        DataType::Utf8 => "Utf8".to_string(),
        DataType::Boolean => "Boolean".to_string(),
        DataType::Binary => "Binary".to_string(),
        DataType::Date32 => "Date32".to_string(),
        DataType::Date64 => "Date64".to_string(),
        DataType::Timestamp(unit, tz) => {
            format!("Timestamp({:?}, {:?})", unit, tz)
        }
        _ => format!("{:?}", data_type), // Fallback for complex types
    }
}

/// Convert string representation to Arrow DataType
fn string_to_datatype(s: &str) -> DbxResult<DataType> {
    match s {
        "Int8" => Ok(DataType::Int8),
        "Int16" => Ok(DataType::Int16),
        "Int32" => Ok(DataType::Int32),
        "Int64" => Ok(DataType::Int64),
        "UInt8" => Ok(DataType::UInt8),
        "UInt16" => Ok(DataType::UInt16),
        "UInt32" => Ok(DataType::UInt32),
        "UInt64" => Ok(DataType::UInt64),
        "Float32" => Ok(DataType::Float32),
        "Float64" => Ok(DataType::Float64),
        "Utf8" => Ok(DataType::Utf8),
        "Boolean" => Ok(DataType::Boolean),
        "Binary" => Ok(DataType::Binary),
        "Date32" => Ok(DataType::Date32),
        "Date64" => Ok(DataType::Date64),
        _ => Err(DbxError::Schema(format!("Unsupported data type: {}", s))),
    }
}

// ════════════════════════════════════════════
// Schema Persistence Functions
// ════════════════════════════════════════════

/// Save table schema to persistent storage
pub fn save_schema(wos: &WosBackend, table: &str, schema: &Schema) -> DbxResult<()> {
    let mut metadata = SchemaMetadata::from(schema);
    metadata.table_name = table.to_string();

    let json_bytes =
        serde_json::to_vec(&metadata).map_err(|e| DbxError::Serialization(e.to_string()))?;

    wos.insert("__meta__/schemas", table.as_bytes(), &json_bytes)?;
    Ok(())
}

/// Load table schema from persistent storage
pub fn load_schema(wos: &WosBackend, table: &str) -> DbxResult<Option<Arc<Schema>>> {
    match wos.get("__meta__/schemas", table.as_bytes())? {
        Some(json_bytes) => {
            let metadata: SchemaMetadata = serde_json::from_slice(&json_bytes)
                .map_err(|e| DbxError::Serialization(e.to_string()))?;
            let schema = Schema::try_from(metadata)?;
            Ok(Some(Arc::new(schema)))
        }
        None => Ok(None),
    }
}

/// Delete table schema from persistent storage
pub fn delete_schema(wos: &WosBackend, table: &str) -> DbxResult<()> {
    wos.delete("__meta__/schemas", table.as_bytes())?;
    Ok(())
}

/// Load all schemas from persistent storage
pub fn load_all_schemas(wos: &WosBackend) -> DbxResult<HashMap<String, Arc<Schema>>> {
    let mut schemas = HashMap::new();
    let all_records = wos.scan("__meta__/schemas", ..)?;

    for (key_vec, value_vec) in all_records {
        let table_name =
            String::from_utf8(key_vec).map_err(|e| DbxError::Serialization(e.to_string()))?;
        let metadata: SchemaMetadata = serde_json::from_slice(&value_vec)
            .map_err(|e| DbxError::Serialization(e.to_string()))?;
        let schema = Schema::try_from(metadata)?;
        schemas.insert(table_name, Arc::new(schema));
    }

    Ok(schemas)
}

// ════════════════════════════════════════════
// Index Persistence Functions
// ════════════════════════════════════════════

/// Save index metadata to persistent storage
pub fn save_index(wos: &WosBackend, index_name: &str, table: &str, column: &str) -> DbxResult<()> {
    let metadata = IndexMetadata {
        index_name: index_name.to_string(),
        table_name: table.to_string(),
        column_name: column.to_string(),
    };

    let json_bytes =
        serde_json::to_vec(&metadata).map_err(|e| DbxError::Serialization(e.to_string()))?;

    wos.insert("__meta__/indexes", index_name.as_bytes(), &json_bytes)?;
    Ok(())
}

/// Delete index metadata from persistent storage
pub fn delete_index(wos: &WosBackend, index_name: &str) -> DbxResult<()> {
    wos.delete("__meta__/indexes", index_name.as_bytes())?;
    Ok(())
}

/// Load all index metadata from persistent storage
pub fn load_all_indexes(wos: &WosBackend) -> DbxResult<HashMap<String, (String, String)>> {
    let mut indexes = HashMap::new();
    let all_records = wos.scan("__meta__/indexes", ..)?;

    for (key_vec, value_vec) in all_records {
        let index_name =
            String::from_utf8(key_vec).map_err(|e| DbxError::Serialization(e.to_string()))?;
        let metadata: IndexMetadata = serde_json::from_slice(&value_vec)
            .map_err(|e| DbxError::Serialization(e.to_string()))?;
        indexes.insert(index_name, (metadata.table_name, metadata.column_name));
    }

    Ok(indexes)
}

// ════════════════════════════════════════════
// Tests
// ════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::datatypes::{DataType, Field, Schema};

    #[test]
    fn test_schema_metadata_conversion() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("age", DataType::Int32, true),
        ]);

        let metadata = SchemaMetadata::from(&schema);
        assert_eq!(metadata.fields.len(), 3);
        assert_eq!(metadata.fields[0].name, "id");
        assert_eq!(metadata.fields[0].data_type, "Int64");
        assert!(!metadata.fields[0].nullable);

        let restored_schema = Schema::try_from(metadata).unwrap();
        assert_eq!(restored_schema.fields().len(), 3);
        assert_eq!(restored_schema.field(0).name(), "id");
        assert_eq!(restored_schema.field(0).data_type(), &DataType::Int64);
    }

    #[test]
    fn test_schema_persistence() {
        let wos = WosBackend::open_temporary().unwrap();
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
        ]));

        // Save schema
        save_schema(&wos, "users", &schema).unwrap();

        // Load schema
        let loaded = load_schema(&wos, "users").unwrap();
        assert!(loaded.is_some());
        let loaded_schema = loaded.unwrap();
        assert_eq!(loaded_schema.fields().len(), 2);
        assert_eq!(loaded_schema.field(0).name(), "id");
        assert_eq!(loaded_schema.field(1).name(), "name");

        // Delete schema
        delete_schema(&wos, "users").unwrap();
        let deleted = load_schema(&wos, "users").unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_load_all_schemas() {
        let wos = WosBackend::open_temporary().unwrap();

        // Save multiple schemas
        let schema1 = Arc::new(Schema::new(vec![Field::new("id", DataType::Int64, false)]));
        let schema2 = Arc::new(Schema::new(vec![Field::new("name", DataType::Utf8, true)]));

        save_schema(&wos, "users", &schema1).unwrap();
        save_schema(&wos, "products", &schema2).unwrap();

        // Load all
        let all_schemas = load_all_schemas(&wos).unwrap();
        assert_eq!(all_schemas.len(), 2);
        assert!(all_schemas.contains_key("users"));
        assert!(all_schemas.contains_key("products"));
    }

    #[test]
    fn test_index_persistence() {
        let wos = WosBackend::open_temporary().unwrap();

        // Save index
        save_index(&wos, "idx_name", "users", "name").unwrap();

        // Load all indexes
        let indexes = load_all_indexes(&wos).unwrap();
        assert_eq!(indexes.len(), 1);
        assert_eq!(
            indexes.get("idx_name"),
            Some(&("users".to_string(), "name".to_string()))
        );

        // Delete index
        delete_index(&wos, "idx_name").unwrap();
        let deleted = load_all_indexes(&wos).unwrap();
        assert!(deleted.is_empty());
    }
}
