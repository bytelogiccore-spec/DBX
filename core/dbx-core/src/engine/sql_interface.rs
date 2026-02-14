//! SQL Execution Pipeline — SQL query execution methods

use crate::engine::Database;
use crate::error::{DbxError, DbxResult};
use crate::sql::executor::{
    FilterOperator, HashAggregateOperator, HashJoinOperator, LimitOperator, PhysicalOperator,
    ProjectionOperator, SortOperator, TableScanOperator,
};
use crate::sql::planner::{LogicalPlanner, PhysicalExpr, PhysicalPlan, PhysicalPlanner};
use crate::storage::columnar_cache::ColumnarCache;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use std::collections::HashMap;
use std::sync::Arc;

// ════════════════════════════════════════════
// Helper Functions for WHERE Clause Evaluation
// ════════════════════════════════════════════

/// Reconstruct a full record by prepending the key (col 0) to the stored value JSON.
/// INSERT stores `row_values[1..]` as JSON, excluding the key column.
/// This function re-adds the key as the first element so that schema column indices
/// align correctly with the data during filter evaluation.
fn reconstruct_full_record(key: &[u8], value_bytes: &[u8]) -> Vec<u8> {
    // Try to interpret the key as an integer (le_bytes from INSERT)
    let key_json = if key.len() == 4 {
        let val = i32::from_le_bytes(key.try_into().unwrap_or([0; 4]));
        serde_json::Value::Number(val.into())
    } else if key.len() == 8 {
        let val = i64::from_le_bytes(key.try_into().unwrap_or([0; 8]));
        serde_json::Value::Number(val.into())
    } else {
        // Treat as UTF-8 string key
        let s = String::from_utf8_lossy(key).to_string();
        serde_json::Value::String(s)
    };

    // Parse existing values and prepend key
    let mut values: Vec<serde_json::Value> =
        serde_json::from_slice(value_bytes).unwrap_or_else(|_| vec![]);
    values.insert(0, key_json);
    serde_json::to_vec(&values).unwrap_or_else(|_| value_bytes.to_vec())
}

/// Convert a single JSON record to a RecordBatch for filter evaluation
fn json_record_to_batch(value_bytes: &[u8]) -> DbxResult<RecordBatch> {
    use arrow::array::{ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    let current_values: Vec<serde_json::Value> =
        serde_json::from_slice(value_bytes).unwrap_or_else(|_| vec![]);

    let mut fields = Vec::new();
    let mut columns: Vec<ArrayRef> = Vec::new();

    for (i, val) in current_values.iter().enumerate() {
        match val {
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    fields.push(Field::new(format!("col_{}", i), DataType::Int64, true));
                    columns.push(Arc::new(Int64Array::from(vec![n.as_i64()])));
                } else if n.is_f64() {
                    fields.push(Field::new(format!("col_{}", i), DataType::Float64, true));
                    columns.push(Arc::new(Float64Array::from(vec![n.as_f64()])));
                } else {
                    fields.push(Field::new(format!("col_{}", i), DataType::Int64, true));
                    columns.push(Arc::new(Int64Array::from(vec![n.as_i64()])));
                }
            }
            serde_json::Value::String(s) => {
                fields.push(Field::new(format!("col_{}", i), DataType::Utf8, true));
                columns.push(Arc::new(StringArray::from(vec![Some(s.as_str())])));
            }
            serde_json::Value::Bool(b) => {
                fields.push(Field::new(format!("col_{}", i), DataType::Boolean, true));
                columns.push(Arc::new(BooleanArray::from(vec![Some(*b)])));
            }
            serde_json::Value::Null => {
                fields.push(Field::new(format!("col_{}", i), DataType::Int64, true));
                columns.push(Arc::new(Int64Array::from(vec![None::<i64>])));
            }
            _ => {
                fields.push(Field::new(format!("col_{}", i), DataType::Int64, true));
                columns.push(Arc::new(Int64Array::from(vec![None::<i64>])));
            }
        }
    }

    let schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(schema, columns).map_err(|e| DbxError::from(e))
}

/// Evaluate a filter expression for a single record
/// Returns true if the record matches the filter (or if no filter is provided)
fn evaluate_filter_for_record(
    filter_expr: Option<&PhysicalExpr>,
    value_bytes: &[u8],
) -> DbxResult<bool> {
    if let Some(expr) = filter_expr {
        let batch = json_record_to_batch(value_bytes)?;

        use crate::sql::executor::evaluate_expr;
        let result = evaluate_expr(expr, &batch)?;

        use arrow::array::BooleanArray;
        let bool_array = result
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| DbxError::TypeMismatch {
                expected: "BooleanArray".to_string(),
                actual: format!("{:?}", result.data_type()),
            })?;

        Ok(bool_array.value(0))
    } else {
        Ok(true) // No filter, all records match
    }
}

impl Database {
    // ════════════════════════════════════════════
    // SQL Execution Pipeline
    // ════════════════════════════════════════════

    /// Register table data for SQL queries.
    ///
    /// Tables registered here can be queried via `execute_sql()`.
    pub fn register_table(&self, name: &str, batches: Vec<RecordBatch>) {
        // Store schema from first batch
        if let Some(first_batch) = batches.first() {
            let schema = first_batch.schema();
            let mut schemas = self.table_schemas.write().unwrap();
            schemas.insert(name.to_string(), schema);
        }

        // Store batches
        let mut tables = self.tables.write().unwrap();
        tables.insert(name.to_string(), batches);
    }

    /// Append a RecordBatch to an existing registered table.
    pub fn append_batch(&self, table: &str, batch: RecordBatch) {
        let mut tables = self.tables.write().unwrap();
        tables.entry(table.to_string()).or_default().push(batch);
    }

    /// Execute a SQL query and return RecordBatch results.
    ///
    /// Full pipeline: Parse → LogicalPlan → Optimize → PhysicalPlan → Execute
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::Database;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// // Register table data first, then:
    /// // let batches = db.execute_sql("SELECT * FROM users WHERE age > 18")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn execute_sql(&self, sql: &str) -> DbxResult<Vec<RecordBatch>> {
        // Step 1: Parse SQL → AST
        let statements = self.sql_parser.parse(sql)?;
        if statements.is_empty() {
            return Ok(vec![]);
        }

        // Step 2: Logical Plan
        let planner = LogicalPlanner::new();
        let logical_plan = planner.plan(&statements[0])?;

        // Step 3: Optimize
        let optimized = self.sql_optimizer.optimize(logical_plan)?;

        // Step 4: Physical Plan
        let physical_planner = PhysicalPlanner::new(Arc::clone(&self.table_schemas));
        let physical_plan = physical_planner.plan(&optimized)?;

        // Step 5: Execute
        self.execute_physical_plan(&physical_plan)
    }

    /// Execute a physical plan against registered table data.
    fn execute_physical_plan(&self, plan: &PhysicalPlan) -> DbxResult<Vec<RecordBatch>> {
        match plan {
            PhysicalPlan::Insert {
                table,
                columns: _,
                values,
            } => {
                // Execute INSERT: convert PhysicalExpr values to bytes and insert into Delta Store
                let mut rows_inserted = 0;

                for row_values in values {
                    // For simplicity, use first column as key, rest as value
                    // TODO: Proper schema-based serialization
                    if row_values.is_empty() {
                        continue;
                    }

                    // Extract key from first value
                    let key = match &row_values[0] {
                        PhysicalExpr::Literal(scalar) => {
                            use crate::storage::columnar::ScalarValue;
                            match scalar {
                                ScalarValue::Utf8(s) => s.as_bytes().to_vec(),
                                ScalarValue::Int32(i) => i.to_le_bytes().to_vec(),
                                ScalarValue::Int64(i) => i.to_le_bytes().to_vec(),
                                ScalarValue::Float64(f) => f.to_le_bytes().to_vec(),
                                ScalarValue::Boolean(b) => vec![if *b { 1 } else { 0 }],
                                // The following cases are not expected here, but are added as per instruction
                                // They would typically be handled in a `build_operator` method or similar
                                // where DDL/DML plans are distinguished from query plans.
                                // For now, we'll place them here as a placeholder for the instruction.
                                _ => {
                                    return Err(DbxError::NotImplemented(
                                        "Non-literal key in INSERT".to_string(),
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(DbxError::NotImplemented(
                                "Non-literal key in INSERT".to_string(),
                            ));
                        }
                    };

                    // Extract remaining values as JSON-serializable format
                    let mut value_vec = Vec::new();
                    for expr in &row_values[1..] {
                        match expr {
                            PhysicalExpr::Literal(scalar) => {
                                use crate::storage::columnar::ScalarValue;
                                let json_val = match scalar {
                                    ScalarValue::Utf8(s) => serde_json::Value::String(s.clone()),
                                    ScalarValue::Int32(i) => serde_json::Value::Number((*i).into()),
                                    ScalarValue::Int64(i) => serde_json::Value::Number((*i).into()),
                                    ScalarValue::Float64(f) => serde_json::Number::from_f64(*f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null),
                                    ScalarValue::Boolean(b) => serde_json::Value::Bool(*b),
                                    ScalarValue::Null => serde_json::Value::Null,
                                };
                                value_vec.push(json_val);
                            }
                            _ => {
                                return Err(DbxError::NotImplemented(
                                    "Non-literal value in INSERT".to_string(),
                                ));
                            }
                        }
                    }

                    // Serialize values as JSON
                    let value_json = serde_json::to_vec(&value_vec)
                        .map_err(|e| DbxError::Serialization(e.to_string()))?;

                    // Insert into Delta Store
                    self.insert(table, &key, &value_json)?;
                    rows_inserted += 1;
                }

                // Return result batch indicating success
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_inserted",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![rows_inserted as i64]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::Update {
                table,
                assignments,
                filter,
            } => {
                // Execute UPDATE: scan, evaluate filter, update matching records
                let all_records = self.scan(table)?;
                let mut rows_updated = 0_i64;

                // Build column name → index mapping from schema
                let column_index_map = {
                    let schemas = self.table_schemas.read().unwrap();
                    schemas.get(table.as_str()).map(|schema| {
                        schema
                            .fields()
                            .iter()
                            .enumerate()
                            .map(|(i, field)| (field.name().clone(), i))
                            .collect::<std::collections::HashMap<String, usize>>()
                    })
                };

                for (key, value_bytes) in all_records {
                    // Reconstruct full record: prepend key (col 0) to value
                    // INSERT stores row_values[1..] as JSON, so key must be re-added
                    let full_record = reconstruct_full_record(&key, &value_bytes);
                    let should_update = evaluate_filter_for_record(filter.as_ref(), &full_record)?;

                    if should_update {
                        let mut current_values: Vec<serde_json::Value> =
                            serde_json::from_slice(&full_record).unwrap_or_else(|_| vec![]);

                        // Apply assignments using schema-based column mapping
                        for (column_name, expr) in assignments.iter() {
                            let target_idx = column_index_map
                                .as_ref()
                                .and_then(|map| map.get(column_name).copied())
                                .unwrap_or_else(|| {
                                    // Fallback: linear search by position
                                    assignments
                                        .iter()
                                        .position(|(n, _)| n == column_name)
                                        .unwrap_or(0)
                                });

                            if let PhysicalExpr::Literal(scalar) = expr {
                                use crate::storage::columnar::ScalarValue;
                                let new_value = match scalar {
                                    ScalarValue::Utf8(s) => serde_json::Value::String(s.clone()),
                                    ScalarValue::Int32(v) => serde_json::Value::Number((*v).into()),
                                    ScalarValue::Int64(v) => serde_json::Value::Number((*v).into()),
                                    ScalarValue::Float64(f) => serde_json::Number::from_f64(*f)
                                        .map(serde_json::Value::Number)
                                        .unwrap_or(serde_json::Value::Null),
                                    ScalarValue::Boolean(b) => serde_json::Value::Bool(*b),
                                    ScalarValue::Null => serde_json::Value::Null,
                                };

                                if target_idx < current_values.len() {
                                    current_values[target_idx] = new_value;
                                }
                            }
                        }

                        // Remove key (col 0) before serializing back to storage
                        let storage_values = if current_values.len() > 1 {
                            &current_values[1..]
                        } else {
                            &current_values[..]
                        };
                        let new_value_bytes = serde_json::to_vec(storage_values)
                            .map_err(|e| DbxError::Serialization(e.to_string()))?;

                        self.insert(table, &key, &new_value_bytes)?;
                        rows_updated += 1;
                    }
                }

                // Return result batch
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_updated",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![rows_updated]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::Delete { table, filter } => {
                // Execute DELETE: scan table, evaluate filter, delete matching records

                // Step 1: Scan all records from the table
                let all_records = self.scan(table)?;

                let mut rows_deleted = 0_i64;

                // Step 2: Process each record
                for (key, value_bytes) in all_records {
                    // Reconstruct full record: prepend key (col 0) to value
                    let full_record = reconstruct_full_record(&key, &value_bytes);
                    let should_delete = evaluate_filter_for_record(filter.as_ref(), &full_record)?;

                    if should_delete {
                        // Delete from Delta Store
                        self.delete(table, &key)?;
                        rows_deleted += 1;
                    }
                }

                // Return result batch
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_deleted",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![rows_deleted]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::DropTable { table, if_exists } => {
                // DROP TABLE implementation
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                // Check if table exists
                let exists = self.table_schemas.read().unwrap().contains_key(table);

                if !exists && !if_exists {
                    return Err(DbxError::TableNotFound(table.clone()));
                }

                if exists {
                    // Remove table schema from memory
                    self.table_schemas.write().unwrap().remove(table);

                    // Delete schema from persistent storage
                    self.wos.delete_schema_metadata(table)?;

                    // Note: Data deletion from Delta Store/WOS would require
                    // scanning and deleting all keys with table prefix
                    // For now, we just remove the schema metadata
                }

                // Return success
                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_affected",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![1]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::CreateTable {
                table,
                columns,
                if_not_exists,
            } => {
                // CREATE TABLE implementation
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                // Check if table already exists
                let exists = self.table_schemas.read().unwrap().contains_key(table);

                if exists && !if_not_exists {
                    return Err(DbxError::Schema(format!(
                        "Table '{}' already exists",
                        table
                    )));
                }

                if !exists {
                    // Create Arrow schema from column definitions
                    let fields: Vec<Field> = columns
                        .iter()
                        .map(|(name, type_str)| {
                            let data_type = match type_str.to_uppercase().as_str() {
                                "INT" | "INTEGER" => DataType::Int64,
                                "TEXT" | "STRING" | "VARCHAR" => DataType::Utf8,
                                "FLOAT" | "DOUBLE" => DataType::Float64,
                                "BOOL" | "BOOLEAN" => DataType::Boolean,
                                _ => DataType::Utf8, // Default to string
                            };
                            Field::new(name, data_type, true)
                        })
                        .collect();

                    let schema = Arc::new(Schema::new(fields));

                    // Store schema in memory
                    self.table_schemas
                        .write()
                        .unwrap()
                        .insert(table.clone(), schema.clone());

                    // Persist schema to storage
                    self.wos.save_schema_metadata(table, &schema)?;
                }

                // Return success
                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_affected",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![1]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::CreateIndex {
                table,
                index_name,
                columns,
                if_not_exists,
            } => {
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                // Validate: target table must exist
                {
                    let schemas = self.table_schemas.read().unwrap();
                    if !schemas.contains_key(table.as_str()) {
                        return Err(DbxError::Schema(format!(
                            "Table '{}' does not exist",
                            table
                        )));
                    }
                }

                let column = columns.get(0).ok_or_else(|| {
                    DbxError::Schema("CREATE INDEX requires at least one column".to_string())
                })?;

                let exists = self.index.has_index(table, column);

                if exists && !if_not_exists {
                    return Err(DbxError::IndexAlreadyExists {
                        table: table.clone(),
                        column: column.clone(),
                    });
                }

                if !exists {
                    self.index.create_index(table, column)?;

                    // Register index_name → (table, column) mapping in memory
                    self.index_registry
                        .write()
                        .unwrap()
                        .insert(index_name.clone(), (table.clone(), column.clone()));

                    // Persist index metadata to storage
                    self.wos.save_index_metadata(index_name, table, column)?;
                }

                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_affected",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![1]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::DropIndex {
                table,
                index_name,
                if_exists,
            } => {
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                // Resolve index_name → actual column via registry
                let resolved_column = {
                    let registry = self.index_registry.read().unwrap();
                    registry
                        .get(index_name.as_str())
                        .map(|(_, col)| col.clone())
                };

                // Fallback: if not in registry, try index_name as column name
                let column = resolved_column.as_deref().unwrap_or(index_name.as_str());

                let exists = self.index.has_index(table, column);

                if !exists && !if_exists {
                    return Err(DbxError::IndexNotFound {
                        table: table.clone(),
                        column: column.to_string(),
                    });
                }

                if exists {
                    self.index.drop_index(table, column)?;

                    // Remove from registry in memory
                    self.index_registry
                        .write()
                        .unwrap()
                        .remove(index_name.as_str());

                    // Delete index metadata from storage
                    self.wos.delete_index_metadata(index_name)?;
                }

                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_affected",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![1]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            PhysicalPlan::AlterTable { table, operation } => {
                // ALTER TABLE implementation
                use crate::sql::planner::types::AlterTableOperation;
                use arrow::array::{Int64Array, RecordBatch};
                use arrow::datatypes::{DataType, Field, Schema};

                match operation {
                    AlterTableOperation::AddColumn {
                        column_name,
                        data_type,
                    } => {
                        // Get current schema
                        let mut schemas = self.table_schemas.write().unwrap();
                        let current_schema = schemas.get(table).ok_or_else(|| {
                            DbxError::Schema(format!("Table '{}' not found", table))
                        })?;

                        // Convert data type string to Arrow DataType
                        let arrow_type = match data_type.to_uppercase().as_str() {
                            "INT" | "INTEGER" => DataType::Int64,
                            "TEXT" | "VARCHAR" | "STRING" => DataType::Utf8,
                            "FLOAT" | "DOUBLE" | "REAL" => DataType::Float64,
                            "BOOL" | "BOOLEAN" => DataType::Boolean,
                            _ => DataType::Utf8, // Default to string
                        };

                        // Create new field
                        let new_field = Field::new(column_name, arrow_type, true);

                        // Create new schema with added column
                        let mut fields: Vec<Field> = current_schema
                            .fields()
                            .iter()
                            .map(|f| f.as_ref().clone())
                            .collect();
                        fields.push(new_field);
                        let new_schema = Arc::new(Schema::new(fields));

                        // Update schema in memory
                        schemas.insert(table.clone(), new_schema.clone());

                        // Persist updated schema
                        drop(schemas); // Release lock before calling wos
                        self.wos.save_schema_metadata(table, &new_schema)?;
                    }
                    AlterTableOperation::DropColumn { column_name } => {
                        // Get current schema
                        let mut schemas = self.table_schemas.write().unwrap();
                        let current_schema = schemas.get(table).ok_or_else(|| {
                            DbxError::Schema(format!("Table '{}' not found", table))
                        })?;

                        // Find the column to drop
                        let fields: Vec<Field> = current_schema
                            .fields()
                            .iter()
                            .filter(|f| f.name() != column_name)
                            .map(|f| f.as_ref().clone())
                            .collect();

                        // Check if column was found
                        if fields.len() == current_schema.fields().len() {
                            return Err(DbxError::Schema(format!(
                                "Column '{}' not found in table '{}'",
                                column_name, table
                            )));
                        }

                        // Create new schema without the dropped column
                        let new_schema = Arc::new(Schema::new(fields));

                        // Update schema in memory
                        schemas.insert(table.clone(), new_schema.clone());

                        // Persist updated schema
                        drop(schemas); // Release lock
                        self.wos.save_schema_metadata(table, &new_schema)?;
                    }
                    AlterTableOperation::RenameColumn { old_name, new_name } => {
                        // Get current schema
                        let mut schemas = self.table_schemas.write().unwrap();
                        let current_schema = schemas.get(table).ok_or_else(|| {
                            DbxError::Schema(format!("Table '{}' not found", table))
                        })?;

                        // Find and rename the column
                        let mut found = false;
                        let fields: Vec<Field> = current_schema
                            .fields()
                            .iter()
                            .map(|f| {
                                if f.name() == old_name {
                                    found = true;
                                    Field::new(new_name, f.data_type().clone(), f.is_nullable())
                                } else {
                                    f.as_ref().clone()
                                }
                            })
                            .collect();

                        // Check if column was found
                        if !found {
                            return Err(DbxError::Schema(format!(
                                "Column '{}' not found in table '{}'",
                                old_name, table
                            )));
                        }

                        // Create new schema with renamed column
                        let new_schema = Arc::new(Schema::new(fields));

                        // Update schema in memory
                        schemas.insert(table.clone(), new_schema.clone());

                        // Persist updated schema
                        drop(schemas); // Release lock
                        self.wos.save_schema_metadata(table, &new_schema)?;
                    }
                }

                // Return success
                let schema = Arc::new(Schema::new(vec![Field::new(
                    "rows_affected",
                    DataType::Int64,
                    false,
                )]));
                let array = Int64Array::from(vec![1]);
                let batch = RecordBatch::try_new(schema, vec![Arc::new(array)])
                    .map_err(|e| DbxError::from(e))?;

                Ok(vec![batch])
            }
            _ => {
                // Original logic for SELECT queries
                self.execute_select_plan(plan)
            }
        }
    }

    /// Execute SELECT query plans (original logic)
    fn execute_select_plan(&self, plan: &PhysicalPlan) -> DbxResult<Vec<RecordBatch>> {
        // Phase 6.3: Automatic Tier Selection & Loading
        // If the query is analytical (OLAP), ensure the tables are in the Columnar Cache.
        if plan.is_analytical() {
            for table in plan.tables() {
                if !self.columnar_cache.has_table(&table) {
                    // Try to sync from Delta (Tier 1) to Cache (Tier 2) automatically
                    let _ = self.sync_columnar_cache(&table);
                }
            }
        }

        let tables = self.tables.read().unwrap();
        let mut operator = self.build_operator(plan, &tables, &self.columnar_cache)?;
        Self::drain_operator(&mut *operator)
    }

    /// Build an operator tree from a physical plan.
    fn build_operator(
        &self,
        plan: &PhysicalPlan,
        tables: &HashMap<String, Vec<RecordBatch>>,
        columnar_cache: &ColumnarCache,
    ) -> DbxResult<Box<dyn PhysicalOperator>> {
        match plan {
            PhysicalPlan::TableScan {
                table,
                projection,
                filter,
            } => {
                let mut filter_pushed_down = false;

                // Try Columnar Cache first (with projection AND filter pushdown!)
                let cached_results = if let Some(filter_expr) = filter {
                    let filter_expr_clone = filter_expr.clone();
                    // Use pushdown with filter
                    let result = columnar_cache.get_batches_with_filter(
                        table,
                        if projection.is_empty() {
                            None
                        } else {
                            Some(projection)
                        },
                        move |batch| {
                            use crate::sql::executor::evaluate_expr;
                            use arrow::array::BooleanArray;

                            let array = evaluate_expr(&filter_expr_clone, batch)?;
                            let boolean_array = array
                                .as_any()
                                .downcast_ref::<BooleanArray>()
                                .ok_or_else(|| DbxError::TypeMismatch {
                                    expected: "BooleanArray".to_string(),
                                    actual: format!("{:?}", array.data_type()),
                                })?;
                            Ok(boolean_array.clone())
                        },
                    )?;
                    if result.is_some() {
                        filter_pushed_down = true;
                    }
                    result
                } else {
                    columnar_cache.get_batches(
                        table,
                        if projection.is_empty() {
                            None
                        } else {
                            Some(projection)
                        },
                    )?
                };

                let (batches, schema, projection_to_use) =
                    if let Some(cached_batches) = cached_results {
                        if cached_batches.is_empty() {
                            return Err(DbxError::TableNotFound(table.clone()));
                        }
                        // Cache hit! Batches are already projected (and filtered if pushed down).
                        let schema = cached_batches[0].schema();
                        // Projection is already applied, so pass empty projection to operator.
                        (cached_batches, schema, vec![])
                    } else {
                        // Reset flag if cache miss
                        filter_pushed_down = false;

                        // Fallback to HashMap
                        let batches = tables
                            .get(table)
                            .ok_or_else(|| DbxError::TableNotFound(table.clone()))?;

                        if batches.is_empty() {
                            return Err(DbxError::TableNotFound(table.clone()));
                        }

                        let schema = batches[0].schema();
                        (batches.clone(), schema, projection.clone())
                    };

                let mut scan =
                    TableScanOperator::new(table.clone(), Arc::clone(&schema), projection_to_use);
                scan.set_data(batches);

                // Wrap with filter if needed AND NOT pushed down
                if let Some(filter_expr) = filter {
                    if !filter_pushed_down {
                        Ok(Box::new(FilterOperator::new(
                            Box::new(scan),
                            filter_expr.clone(),
                        )))
                    } else {
                        // Filter already applied in scan (via cache)
                        Ok(Box::new(scan))
                    }
                } else {
                    Ok(Box::new(scan))
                }
            }

            PhysicalPlan::Projection {
                input,
                exprs,
                aliases,
            } => {
                let input_op = self.build_operator(input, tables, columnar_cache)?;
                use arrow::datatypes::Field;

                let input_schema = input_op.schema();
                let fields: Vec<Field> = exprs
                    .iter()
                    .enumerate()
                    .map(|(i, expr)| {
                        let data_type = expr.get_type(input_schema);
                        let field_name = if let Some(Some(alias)) = aliases.get(i) {
                            alias.clone()
                        } else {
                            format!("col_{}", i)
                        };
                        Field::new(&field_name, data_type, true)
                    })
                    .collect();

                let output_schema = Arc::new(Schema::new(fields));
                Ok(Box::new(ProjectionOperator::new(
                    input_op,
                    output_schema,
                    exprs.clone(),
                )))
            }

            PhysicalPlan::Limit {
                input,
                count,
                offset,
            } => {
                let input_op = self.build_operator(input, tables, columnar_cache)?;
                Ok(Box::new(LimitOperator::new(input_op, *count, *offset)))
            }

            PhysicalPlan::SortMerge { input, order_by } => {
                let input_op = self.build_operator(input, tables, columnar_cache)?;
                Ok(Box::new(SortOperator::new(input_op, order_by.clone())))
            }

            PhysicalPlan::HashAggregate {
                input,
                group_by,
                aggregates,
            } => {
                let input_op = self.build_operator(input, tables, columnar_cache)?;
                // Build output schema for aggregate (simplified: use input schema)
                let agg_schema = Arc::new(input_op.schema().clone());
                Ok(Box::new(
                    HashAggregateOperator::new(
                        input_op,
                        agg_schema,
                        group_by.clone(),
                        aggregates.clone(),
                    )
                    .with_gpu(self.gpu_manager.clone()),
                ))
            }

            PhysicalPlan::HashJoin {
                left,
                right,
                on,
                join_type,
            } => {
                use arrow::datatypes::Field;

                let left_op = self.build_operator(left, tables, columnar_cache)?;
                let right_op = self.build_operator(right, tables, columnar_cache)?;

                // Build joined schema: left columns + right columns
                let left_schema = left_op.schema();
                let right_schema = right_op.schema();

                let mut joined_fields: Vec<Field> = Vec::new();
                for field in left_schema.fields().iter() {
                    let mut f = field.as_ref().clone();
                    // In RIGHT JOIN, left side can be null
                    if matches!(join_type, crate::sql::planner::JoinType::Right) {
                        f = f.with_nullable(true);
                    }
                    joined_fields.push(f);
                }
                for field in right_schema.fields().iter() {
                    let mut f = field.as_ref().clone();
                    // In LEFT JOIN, right side can be null
                    if matches!(join_type, crate::sql::planner::JoinType::Left) {
                        f = f.with_nullable(true);
                    }
                    joined_fields.push(f);
                }

                let joined_schema = Arc::new(Schema::new(joined_fields));

                Ok(Box::new(HashJoinOperator::new(
                    left_op,
                    right_op,
                    joined_schema,
                    on.clone(),
                    *join_type,
                )))
            }

            PhysicalPlan::Insert { .. } => {
                // INSERT should be handled in execute_physical_plan, not here
                unreachable!("INSERT should not reach build_operator")
            }
            PhysicalPlan::Update { .. } => {
                // UPDATE should be handled in execute_physical_plan, not here
                unreachable!("UPDATE should not reach build_operator")
            }
            PhysicalPlan::Delete { .. } => {
                // DELETE should be handled in execute_physical_plan, not here
                unreachable!("DELETE should not reach build_operator")
            }
            PhysicalPlan::DropTable { .. } => {
                // DROP TABLE should be handled in execute_physical_plan, not here
                unreachable!("DROP TABLE should not reach build_operator")
            }
            PhysicalPlan::CreateTable { .. } => {
                // CREATE TABLE should be handled in execute_physical_plan, not here
                unreachable!("CREATE TABLE should not reach build_operator")
            }
            PhysicalPlan::CreateIndex { .. } => {
                // CREATE INDEX should be handled in execute_physical_plan, not here
                unreachable!("CREATE INDEX should not reach build_operator")
            }
            PhysicalPlan::DropIndex { .. } => {
                // DROP INDEX should be handled in execute_physical_plan, not here
                unreachable!("DROP INDEX should not reach build_operator")
            }
            PhysicalPlan::AlterTable { .. } => {
                // ALTER TABLE should be handled in execute_physical_plan, not here
                unreachable!("ALTER TABLE should not reach build_operator")
            }
        }
    }

    /// Drain all batches from an operator.
    fn drain_operator(op: &mut dyn PhysicalOperator) -> DbxResult<Vec<RecordBatch>> {
        let mut results = Vec::new();
        while let Some(batch) = op.next()? {
            if batch.num_rows() > 0 {
                results.push(batch);
            }
        }
        Ok(results)
    }
}
