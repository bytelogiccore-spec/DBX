//! DDL statement planning - CREATE TABLE, CREATE INDEX, DROP TABLE, DROP INDEX, ALTER TABLE

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::{AlterTableOperation as SqlAlterOp, ObjectType, Statement};

use super::LogicalPlanner;

impl LogicalPlanner {
    /// DDL statement 처리 (CREATE TABLE, CREATE INDEX, DROP, ALTER TABLE)
    pub(super) fn plan_ddl(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        match statement {
            Statement::Drop {
                names,
                object_type,
                if_exists,
                ..
            } => {
                // DROP TABLE or DROP INDEX parsing
                match object_type {
                    ObjectType::Table => {
                        let table = names[0].to_string();
                        Ok(LogicalPlan::DropTable {
                            table,
                            if_exists: *if_exists,
                        })
                    }
                    ObjectType::Index => {
                        // DROP INDEX
                        if names.is_empty() {
                            return Err(DbxError::Schema(
                                "DROP INDEX requires an index name".to_string(),
                            ));
                        }

                        // Parse index name (may include table name)
                        let index_full_name = names[0].to_string();

                        // Try to split table.index_name format
                        let (table, index_name) = if index_full_name.contains('.') {
                            let parts: Vec<&str> = index_full_name.splitn(2, '.').collect();
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            // If no table specified, we'll need to find it later
                            // For now, use empty string as placeholder
                            ("".to_string(), index_full_name)
                        };

                        Ok(LogicalPlan::DropIndex {
                            table,
                            index_name,
                            if_exists: *if_exists,
                        })
                    }
                    _ => Err(DbxError::SqlNotSupported {
                        feature: format!("DROP {:?}", object_type),
                        hint: "Only DROP TABLE and DROP INDEX are currently supported".to_string(),
                    }),
                }
            }
            Statement::CreateTable(create_table) => {
                // CREATE TABLE parsing
                let table = create_table.name.to_string();
                let columns: Vec<(String, String)> = create_table
                    .columns
                    .iter()
                    .map(|col| {
                        let name = col.name.to_string();
                        let type_str = col.data_type.to_string();
                        (name, type_str)
                    })
                    .collect();

                let mut policy = None;
                if !create_table.with_options.is_empty() {
                    let mut pol = crate::engine::policy::TablePolicy::default();
                    for opt in &create_table.with_options {
                        let opt_str = opt.to_string().replace(" ", "").replace("\"", "").replace("'", "").to_lowercase();
                        let parts: Vec<&str> = opt_str.split('=').collect();
                        if parts.len() == 2 {
                            let key = parts[0];
                            let val = parts[1];
                            match key {
                                "hot_ttl" | "hot_ttl_days" => {
                                    if val == "none" || val == "null" {
                                        pol.hot_ttl_days = None;
                                    } else if let Ok(n) = val.parse::<u32>() {
                                        pol.hot_ttl_days = Some(n);
                                    }
                                }
                                "warm_ttl" | "warm_ttl_days" => {
                                    if val == "none" || val == "null" {
                                        pol.warm_ttl_days = None;
                                    } else if let Ok(n) = val.parse::<u32>() {
                                        pol.warm_ttl_days = Some(n);
                                    }
                                }
                                "cold_ttl" | "cold_ttl_days" => {
                                    if val == "none" || val == "null" {
                                        pol.cold_ttl_days = None;
                                    } else if let Ok(n) = val.parse::<u32>() {
                                        pol.cold_ttl_days = Some(n);
                                    }
                                }
                                "hot_strategy" => {
                                    if val.contains("sharding") { pol.hot_strategy = crate::engine::policy::StorageStrategy::PureSharding; }
                                    else if val.contains("replication") { pol.hot_strategy = crate::engine::policy::StorageStrategy::Replication { factor: 3 }; }
                                }
                                "warm_strategy" => {
                                    if val.contains("erasure") { pol.warm_strategy = crate::engine::policy::StorageStrategy::ErasureCoding { k: 4, m: 2 }; }
                                    else if val.contains("sharding") { pol.warm_strategy = crate::engine::policy::StorageStrategy::PureSharding; }
                                    else if val.contains("replication") { pol.warm_strategy = crate::engine::policy::StorageStrategy::Replication { factor: 3 }; }
                                }
                                "cold_strategy" => {
                                    if val.contains("erasure") { pol.cold_strategy = crate::engine::policy::StorageStrategy::ErasureCoding { k: 4, m: 2 }; }
                                    else if val.contains("sharding") { pol.cold_strategy = crate::engine::policy::StorageStrategy::PureSharding; }
                                }
                                _ => {}
                            }
                        }
                    }
                    policy = Some(pol);
                }

                Ok(LogicalPlan::CreateTable {
                    table,
                    columns,
                    if_not_exists: create_table.if_not_exists,
                    policy,
                })
            }
            Statement::AlterTable {
                name, operations, ..
            } => {
                // ALTER TABLE parsing (simplified - only ADD COLUMN for now)
                let table = name.to_string();

                // Get the first operation
                let operation = operations.first().ok_or_else(|| {
                    DbxError::Schema("ALTER TABLE requires at least one operation".to_string())
                })?;

                let alter_op = match operation {
                    SqlAlterOp::AddColumn { column_def, .. } => {
                        let column_name = column_def.name.to_string();
                        let data_type = column_def.data_type.to_string();
                        AlterTableOperation::AddColumn {
                            column_name,
                            data_type,
                        }
                    }
                    SqlAlterOp::DropColumn { column_name, .. } => {
                        let col_name = column_name.to_string();
                        AlterTableOperation::DropColumn {
                            column_name: col_name,
                        }
                    }
                    SqlAlterOp::RenameColumn {
                        old_column_name,
                        new_column_name,
                    } => {
                        let old_name = old_column_name.to_string();
                        let new_name = new_column_name.to_string();
                        AlterTableOperation::RenameColumn {
                            old_name,
                            new_name,
                        }
                    }
                    _ => {
                        return Err(DbxError::SqlNotSupported {
                            feature: format!("ALTER TABLE operation: {:?}", operation),
                            hint: "Only ADD COLUMN, DROP COLUMN, and RENAME COLUMN are currently supported".to_string(),
                        });
                    }
                };

                Ok(LogicalPlan::AlterTable {
                    table,
                    operation: alter_op,
                })
            }
            Statement::CreateIndex(create_index) => {
                // CREATE INDEX parsing
                let index_name = create_index
                    .name
                    .as_ref()
                    .ok_or_else(|| {
                        DbxError::Schema("CREATE INDEX requires an index name".to_string())
                    })?
                    .to_string();

                // In sqlparser 0.52, table_name is ObjectName (not Option)
                let table = create_index.table_name.to_string();

                // Extract column names from OrderByExpr
                if create_index.columns.is_empty() {
                    return Err(DbxError::Schema(
                        "CREATE INDEX requires at least one column".to_string(),
                    ));
                }

                let columns: Vec<String> = create_index
                    .columns
                    .iter()
                    .map(|order_by_expr| {
                        // Extract column name from the expression
                        match &order_by_expr.expr {
                            sqlparser::ast::Expr::Identifier(ident) => ident.value.clone(),
                            sqlparser::ast::Expr::CompoundIdentifier(idents) => idents
                                .iter()
                                .map(|i| i.value.clone())
                                .collect::<Vec<_>>()
                                .join("."),
                            _ => order_by_expr.expr.to_string(),
                        }
                    })
                    .collect();

                Ok(LogicalPlan::CreateIndex {
                    table,
                    index_name,
                    columns,
                    if_not_exists: create_index.if_not_exists,
                })
            }
            _ => Err(DbxError::SqlNotSupported {
                feature: format!("DDL statement: {:?}", statement),
                hint: "Only CREATE TABLE, CREATE INDEX, DROP TABLE, DROP INDEX, and ALTER TABLE are supported".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::planner::LogicalPlan;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    #[test]
    fn test_parse_create_table_with_options() {
        let sql = "CREATE TABLE users (id INT, name TEXT) WITH (hot_ttl = 30, cold_strategy = 'erasure', warm_strategy='sharding', hot_strategy='replication')";
        let statements = Parser::parse_sql(&GenericDialect {}, sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan_ddl(&statements[0]).unwrap();

        if let LogicalPlan::CreateTable { table, policy, .. } = plan {
            assert_eq!(table, "users");
            assert!(policy.is_some());
            let pol = policy.unwrap();

            assert_eq!(pol.hot_ttl_days, Some(30));

            match pol.hot_strategy {
                crate::engine::policy::StorageStrategy::Replication { .. } => {}
                _ => panic!("Expected Hot Strategy Replication"),
            }

            match pol.warm_strategy {
                crate::engine::policy::StorageStrategy::PureSharding => {}
                _ => panic!("Expected Warm Strategy Sharding"),
            }

            match pol.cold_strategy {
                crate::engine::policy::StorageStrategy::ErasureCoding { .. } => {}
                _ => panic!("Expected Cold Strategy ErasureCoding"),
            }
        } else {
            panic!("Expected CreateTable logical plan");
        }
    }
}
