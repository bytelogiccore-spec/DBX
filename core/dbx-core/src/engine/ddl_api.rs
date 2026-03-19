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

    // ════════════════════════════════════════════
    // Phase 3 Synergy: PartitionStats API
    // ════════════════════════════════════════════

    /// 파티션 통계를 갱신합니다.
    ///
    /// # Example
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::partition::PartitionStats;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.update_partition_stats("orders", "orders__p_part_0", PartitionStats {
    ///     row_count: 1000, min_value: 0, max_value: 999,
    ///     null_count: 0, distinct_count: 1000,
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_partition_stats(
        &self,
        table: &str,
        partition_name: &str,
        stats: crate::storage::partition::PartitionStats,
    ) -> DbxResult<()> {
        let key = format!("{}__{}", table, partition_name);
        self.partition_stats.insert(key, stats);
        Ok(())
    }

    /// 특정 파티션의 통계를 조회합니다.
    pub fn get_partition_stats(
        &self,
        table: &str,
        partition_name: &str,
    ) -> DbxResult<crate::storage::partition::PartitionStats> {
        let key = format!("{}__{}", table, partition_name);
        self.partition_stats
            .get(&key)
            .map(|r| r.clone())
            .ok_or_else(|| crate::error::DbxError::InvalidOperation {
                message: format!("No stats for partition '{}'", partition_name),
                context: format!("Call update_partition_stats first for table '{}'", table),
            })
    }

    /// 테이블의 모든 파티션 통계를 반환합니다.
    pub fn all_partition_stats(
        &self,
        table: &str,
    ) -> DbxResult<std::collections::HashMap<String, crate::storage::partition::PartitionStats>> {
        let prefix = format!("{}__", table);
        let result = self
            .partition_stats
            .iter()
            .filter(|r| r.key().starts_with(&prefix))
            .map(|r| (r.key().clone(), r.value().clone()))
            .collect();
        Ok(result)
    }

    // ════════════════════════════════════════════
    // Phase 3 Synergy: Per-Partition Compression API
    // ════════════════════════════════════════════

    /// 파티션별 압축 설정을 지정합니다.
    ///
    /// # Example
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::compression::CompressionConfig;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// // 오래된 파티션 → 고압축
    /// db.set_partition_compression("orders", "orders__p_part_0", CompressionConfig::zstd_level(9))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_partition_compression(
        &self,
        table: &str,
        partition_name: &str,
        config: crate::storage::compression::CompressionConfig,
    ) -> DbxResult<()> {
        let key = format!("{}__{}", table, partition_name);
        self.partition_compression.insert(key, config);
        Ok(())
    }

    /// 파티션 압축 설정 조회 (미설정 시 기본값 Snappy 반환).
    pub fn get_partition_compression(
        &self,
        table: &str,
        partition_name: &str,
    ) -> DbxResult<crate::storage::compression::CompressionConfig> {
        let key = format!("{}__{}", table, partition_name);
        Ok(self
            .partition_compression
            .get(&key)
            .map(|r| *r)
            .unwrap_or_default())
    }

    // ════════════════════════════════════════════
    // Phase 3 Synergy: PartitionLifecycle API
    // ════════════════════════════════════════════

    /// 테이블의 파티션 수명 주기 정책을 설정합니다.
    ///
    /// # Example
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::partition::PartitionLifecycle;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.enable_auto_archive("logs", PartitionLifecycle {
    ///     archive_after_days: 90,
    ///     delete_after_days: 365,
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn enable_auto_archive(
        &self,
        table: &str,
        lifecycle: crate::storage::partition::PartitionLifecycle,
    ) -> DbxResult<()> {
        self.partition_lifecycle
            .insert(table.to_string(), lifecycle);
        Ok(())
    }

    /// 테이블의 파티션 수명 주기 정책을 조회합니다.
    pub fn get_partition_lifecycle(
        &self,
        table: &str,
    ) -> DbxResult<crate::storage::partition::PartitionLifecycle> {
        self.partition_lifecycle
            .get(table)
            .map(|r| r.clone())
            .ok_or_else(|| crate::error::DbxError::InvalidOperation {
                message: format!("No lifecycle policy for table '{}'", table),
                context: "Call enable_auto_archive first".to_string(),
            })
    }

    /// 파티션이 아카이브 시점이 되었는지 확인합니다.
    ///
    /// `partition_created_at`: UNIX timestamp (초 단위)
    pub fn partition_needs_archive(
        &self,
        table: &str,
        partition_created_at: u64,
    ) -> DbxResult<bool> {
        let lc = self.get_partition_lifecycle(table)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let threshold_secs = lc.archive_after_days as u64 * 24 * 3600;
        Ok(now.saturating_sub(partition_created_at) >= threshold_secs)
    }

    /// 파티션이 삭제 시점이 되었는지 확인합니다.
    ///
    /// `partition_created_at`: UNIX timestamp (초 단위)
    pub fn partition_needs_delete(
        &self,
        table: &str,
        partition_created_at: u64,
    ) -> DbxResult<bool> {
        let lc = self.get_partition_lifecycle(table)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let threshold_secs = lc.delete_after_days as u64 * 24 * 3600;
        Ok(now.saturating_sub(partition_created_at) >= threshold_secs)
    }

    /// 테이블의 파티션 수명 주기 정책을 실제로 실행합니다.
    ///
    /// `partition_creation_times`에 기록된 타임스탬프를 기반으로:
    /// - 아카이브 시점이 된 파티션 → ZSTD 레벨 9 압축 자동 적용
    /// - 삭제 시점이 된 파티션 → sub-table 드롭
    ///
    /// # Returns
    /// `(archived_count, deleted_count)` — 처리된 파티션 수
    ///
    /// # Example
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::partition::PartitionLifecycle;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.enable_auto_archive("logs", PartitionLifecycle {
    ///     archive_after_days: 90,
    ///     delete_after_days: 365,
    /// })?;
    /// let (archived, deleted) = db.run_partition_lifecycle("logs")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn run_partition_lifecycle(&self, table: &str) -> DbxResult<(usize, usize)> {
        let _lc = self.get_partition_lifecycle(table)?;
        let prefix = format!("{}__", table);

        let mut candidates: Vec<(String, u64)> = self
            .partition_creation_times
            .iter()
            .filter(|r| r.key().starts_with(&prefix))
            .map(|r| (r.key().clone(), *r.value()))
            .collect();

        // delete 우선: 삭제 대상은 archive도 하지 않음
        candidates.sort_by_key(|(_, ts)| *ts); // 오래된 것 먼저

        let mut archived = 0usize;
        let mut deleted = 0usize;

        for (sub_table, created_at) in candidates {
            if self.partition_needs_delete(table, created_at)? {
                // 실제 sub-table 드롭
                let drop_sql = format!("DROP TABLE IF EXISTS \"{}\"", sub_table);
                let _ = self.execute_sql(&drop_sql);
                // 관련 메타데이터 정리
                self.partition_stats.remove(&sub_table);
                self.partition_compression.remove(&sub_table);
                self.partition_tier_hints.remove(&sub_table);
                self.partition_creation_times.remove(&sub_table);
                deleted += 1;
            } else if self.partition_needs_archive(table, created_at)? {
                // ZSTD 레벨 9 고압축 자동 적용
                // ⚠️ sub_table은 이미 "table__p_part_N" 형식이므로
                //    set_partition_compression(table, sub_table) 사용 시 이중 prefix 발생.
                //    DashMap에 직접 삽입하여 올바른 키("table__p_part_N")를 사용.
                self.partition_compression.insert(
                    sub_table.clone(),
                    crate::storage::compression::CompressionConfig::zstd_level(9),
                );
                self.partition_tier_hints.insert(
                    sub_table.clone(),
                    crate::storage::partition::PartitionTierHint::Cold,
                );
                archived += 1;
            }
        }

        Ok((archived, deleted))
    }

    /// 라이프사이클 정책이 있는 모든 테이블에 대해 일괄 실행합니다.
    ///
    /// # Returns
    /// 전체 `(archived_count, deleted_count)`
    pub fn run_all_partition_lifecycles(&self) -> DbxResult<(usize, usize)> {
        let tables: Vec<String> = self
            .partition_lifecycle
            .iter()
            .map(|r| r.key().clone())
            .collect();

        let mut total_archived = 0usize;
        let mut total_deleted = 0usize;

        for table in tables {
            let (a, d) = self.run_partition_lifecycle(&table)?;
            total_archived += a;
            total_deleted += d;
        }

        Ok((total_archived, total_deleted))
    }

    /// 파티션의 생성 시각을 조회합니다 (INSERT 시 자동 기록).
    pub fn get_partition_creation_time(
        &self,
        partition_name: &str,
    ) -> Option<u64> {
        self.partition_creation_times
            .get(partition_name)
            .map(|r| *r)
    }

    // ════════════════════════════════════════════
    // Phase 3 Synergy: PartitionTierHint API
    // ════════════════════════════════════════════

    /// 파티션의 스토리지 티어 힌트를 설정합니다.
    ///
    /// # Example
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::partition::PartitionTierHint;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.set_partition_tier("orders", "orders__p_part_0", PartitionTierHint::Hot)?;
    /// db.set_partition_tier("orders", "orders__p_part_1", PartitionTierHint::Cold)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_partition_tier(
        &self,
        table: &str,
        partition_name: &str,
        hint: crate::storage::partition::PartitionTierHint,
    ) -> DbxResult<()> {
        let key = format!("{}__{}", table, partition_name);
        self.partition_tier_hints.insert(key, hint);
        Ok(())
    }

    /// 파티션 티어 힌트를 조회합니다 (미설정 시 Hot 반환).
    pub fn get_partition_tier(
        &self,
        table: &str,
        partition_name: &str,
    ) -> DbxResult<crate::storage::partition::PartitionTierHint> {
        let key = format!("{}__{}", table, partition_name);
        Ok(self
            .partition_tier_hints
            .get(&key)
            .map(|r| *r)
            .unwrap_or_default())
    }

    /// 특정 티어에 속하는 파티션 목록을 반환합니다.
    pub fn list_partitions_by_tier(
        &self,
        table: &str,
        hint: crate::storage::partition::PartitionTierHint,
    ) -> DbxResult<Vec<String>> {
        let prefix = format!("{}__", table);
        let result = self
            .partition_tier_hints
            .iter()
            .filter(|r| r.key().starts_with(&prefix) && *r.value() == hint)
            .map(|r| r.key().clone())
            .collect();
        Ok(result)
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
