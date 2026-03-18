//! DDL API implementation - Schema management convenience methods

use crate::engine::Database;
use crate::error::DbxResult;
use arrow::datatypes::{DataType, Schema};
use std::sync::Arc;

impl Database {
    /// Create a new table with the given Arrow schema
    ///
    /// This is a convenience wrapper around `execute_sql("CREATE TABLE ...")`.
    /// It automatically converts the Arrow schema to SQL DDL.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("name", DataType::Utf8, true),
    ///     Field::new("age", DataType::Int32, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// assert!(db.table_exists("users"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_table(&self, name: &str, schema: Schema) -> DbxResult<()> {
        let schema_arc = Arc::new(schema);

        // Generate CREATE TABLE SQL from Arrow Schema
        let sql = self.generate_create_table_sql(name, &schema_arc);

        // Execute SQL FIRST (this will check if table exists)
        self.execute_sql(&sql)?;

        // THEN store schema (after SQL succeeds)
        self.table_schemas
            .write()
            .unwrap()
            .insert(name.to_string(), Arc::clone(&schema_arc));

        // Initialize empty table data
        self.tables
            .write()
            .unwrap()
            .insert(name.to_string(), vec![]);

        // Initialize row counter
        self.row_counters
            .insert(name.to_string(), std::sync::atomic::AtomicUsize::new(0));

        Ok(())
    }

    /// Drop a table
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    /// ]);
    ///
    /// db.create_table("temp", schema)?;
    /// db.drop_table("temp")?;
    /// assert!(!db.table_exists("temp"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn drop_table(&self, name: &str) -> DbxResult<()> {
        self.execute_sql(&format!("DROP TABLE {}", name))?;
        self.table_schemas.write().unwrap().remove(name);
        Ok(())
    }

    /// Check if a table exists
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// assert!(!db.table_exists("users"));
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// assert!(db.table_exists("users"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn table_exists(&self, name: &str) -> bool {
        self.table_schemas.read().unwrap().contains_key(name)
    }

    /// Get the schema of a table
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("name", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema.clone())?;
    /// let retrieved_schema = db.get_table_schema("users")?;
    /// assert_eq!(retrieved_schema.fields().len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_table_schema(&self, name: &str) -> DbxResult<Schema> {
        self.table_schemas
            .read()
            .unwrap()
            .get(name)
            .map(|s| (**s).clone())
            .ok_or_else(|| crate::DbxError::Schema(format!("Table '{}' not found", name)))
    }

    /// List all tables
    pub fn list_tables(&self) -> Vec<String> {
        self.table_schemas.read().unwrap().keys().cloned().collect()
    }

    /// Helper: Generate CREATE TABLE SQL from Arrow Schema
    fn generate_create_table_sql(&self, name: &str, schema: &Schema) -> String {
        let columns: Vec<String> = schema
            .fields()
            .iter()
            .map(|field| {
                let sql_type = match field.data_type() {
                    DataType::Int8 | DataType::Int16 | DataType::Int32 | DataType::Int64 => "INT",
                    DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => {
                        "INT"
                    }
                    DataType::Float32 | DataType::Float64 => "FLOAT",
                    DataType::Utf8 | DataType::LargeUtf8 => "TEXT",
                    DataType::Boolean => "BOOLEAN",
                    DataType::Binary | DataType::LargeBinary => "BLOB",
                    DataType::Date32 | DataType::Date64 => "DATE",
                    DataType::Timestamp(_, _) => "TIMESTAMP",
                    _ => "TEXT", // Default to TEXT for unsupported types
                };
                format!("{} {}", field.name(), sql_type)
            })
            .collect();

        format!("CREATE TABLE {} ({})", name, columns.join(", "))
    }

    /// Create a SQL index on table columns
    ///
    /// This is a convenience wrapper around `execute_sql("CREATE INDEX ...")`.
    /// For Hash Index (O(1) lookup), use `create_index(table, column)` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("email", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    /// assert!(db.sql_index_exists("idx_email"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_sql_index(
        &self,
        table: &str,
        index_name: &str,
        columns: Vec<String>,
    ) -> DbxResult<()> {
        // Generate CREATE INDEX SQL
        let columns_str = columns.join(", ");
        let sql = format!("CREATE INDEX {} ON {} ({})", index_name, table, columns_str);

        // Execute SQL
        self.execute_sql(&sql)?;
        Ok(())
    }

    /// Drop a SQL index
    ///
    /// This is a convenience wrapper around `execute_sql("DROP INDEX ...")`.
    /// For Hash Index, use `drop_index(table, column)` instead.
    ///
    /// Note: The index must have been created with `create_sql_index` to be tracked properly.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("email", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    /// db.drop_sql_index("users", "idx_email")?;
    /// assert!(!db.sql_index_exists("idx_email"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn drop_sql_index(&self, table: &str, index_name: &str) -> DbxResult<()> {
        // Use table.index_name format for DROP INDEX
        let sql = format!("DROP INDEX {}.{}", table, index_name);
        self.execute_sql(&sql)?;
        Ok(())
    }

    /// Check if a SQL index exists
    ///
    /// For Hash Index, use `has_index(table, column)` instead.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("email", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// assert!(!db.sql_index_exists("idx_email"));
    ///
    /// db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    /// assert!(db.sql_index_exists("idx_email"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn sql_index_exists(&self, index_name: &str) -> bool {
        self.index_registry.read().unwrap().contains_key(index_name)
    }

    /// List all SQL indexes for a table
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("email", DataType::Utf8, true),
    ///     Field::new("name", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    /// db.create_sql_index("users", "idx_name", vec!["name".to_string()])?;
    ///
    /// let indexes = db.list_sql_indexes("users");
    /// assert!(indexes.contains(&"idx_email".to_string()));
    /// assert!(indexes.contains(&"idx_name".to_string()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn list_sql_indexes(&self, table: &str) -> Vec<String> {
        self.index_registry
            .read()
            .unwrap()
            .iter()
            .filter_map(|(index_name, (tbl, _col))| {
                if tbl == table {
                    Some(index_name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add a column to an existing table
    ///
    /// This is a convenience wrapper around `execute_sql("ALTER TABLE ... ADD COLUMN ...")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("name", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.add_column("users", "email", "TEXT")?;
    ///
    /// let updated_schema = db.get_table_schema("users")?;
    /// assert_eq!(updated_schema.fields().len(), 3);
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_column(&self, table: &str, column_name: &str, data_type: &str) -> DbxResult<()> {
        let sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column_name, data_type
        );
        self.execute_sql(&sql)?;
        Ok(())
    }

    /// Drop a column from an existing table
    ///
    /// This is a convenience wrapper around `execute_sql("ALTER TABLE ... DROP COLUMN ...")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("name", DataType::Utf8, true),
    ///     Field::new("email", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.drop_column("users", "email")?;
    ///
    /// let updated_schema = db.get_table_schema("users")?;
    /// assert_eq!(updated_schema.fields().len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn drop_column(&self, table: &str, column_name: &str) -> DbxResult<()> {
        let sql = format!("ALTER TABLE {} DROP COLUMN {}", table, column_name);
        self.execute_sql(&sql)?;
        Ok(())
    }

    /// Rename a column in an existing table
    ///
    /// This is a convenience wrapper around `execute_sql("ALTER TABLE ... RENAME COLUMN ...")`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("user_name", DataType::Utf8, true),
    /// ]);
    ///
    /// db.create_table("users", schema)?;
    /// db.rename_column("users", "user_name", "name")?;
    ///
    /// let updated_schema = db.get_table_schema("users")?;
    /// assert_eq!(updated_schema.field(1).name(), "name");
    /// # Ok(())
    /// # }
    /// ```
    pub fn rename_column(&self, table: &str, old_name: &str, new_name: &str) -> DbxResult<()> {
        let sql = format!(
            "ALTER TABLE {} RENAME COLUMN {} TO {}",
            table, old_name, new_name
        );
        self.execute_sql(&sql)?;
        Ok(())
    }

    // ════════════════════════════════════════════
    // Phase 3: 파티셔닝 API (Partitioning API)
    // ════════════════════════════════════════════

    /// 파티셔닝 규칙을 생성합니다.
    ///
    /// 이후 해당 `table`로 들어오는 INSERT는 키 값에 따라
    /// `route_key()`가 반환하는 내부 sub-table로 라우팅됩니다.
    pub fn create_partition(&self, map: crate::storage::partition::PartitionMap) -> DbxResult<()> {
        let table_name = map.table.clone();
        self.partition_maps.write().unwrap().insert(table_name, map);
        Ok(())
    }

    /// 자동 확장(Auto-Expand)을 지원하는 범위 파티션을 생성합니다 (Phase 3.4).
    ///
    /// 설정된 범위를 초과하는 키 값이 인입되면 `interval` 크기만큼
    /// 새로운 파티션 구획을 자동 생성하며 지속적으로 확장됩니다.
    pub fn create_auto_range_partition(
        &self,
        table: &str,
        column: &str,
        initial_low: i64,
        interval: i64,
        max_partitions: usize,
    ) -> DbxResult<()> {
        use crate::storage::partition::{PartitionMap, PartitionType};
        let map = PartitionMap {
            table: table.to_string(),
            partition_type: PartitionType::Range {
                column: column.to_string(),
                bounds: vec![(initial_low, initial_low + interval)],
                auto_expand_interval: Some((interval, max_partitions)),
            },
            num_partitions: 1, // 초기 파티션 크기
        };

        self.partition_maps
            .write()
            .unwrap()
            .insert(table.to_string(), map);
        Ok(())
    }

    /// 파티셔닝 규칙을 제거합니다.
    pub fn drop_partition(&self, table: &str) -> DbxResult<()> {
        self.partition_maps.write().unwrap().remove(table);
        Ok(())
    }

    /// 뷰를 생성합니다 (Phase 5.1).
    pub fn create_view(&self, name: &str, sql: &str) -> DbxResult<()> {
        self.view_registry.create(name, sql)
    }

    /// 뷰를 삭제합니다 (Phase 5.1).
    pub fn drop_view(&self, name: &str) -> DbxResult<()> {
        self.view_registry.drop(name)
    }
}
