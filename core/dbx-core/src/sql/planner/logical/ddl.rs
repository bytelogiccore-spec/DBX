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

                Ok(LogicalPlan::CreateTable {
                    table,
                    columns,
                    if_not_exists: create_table.if_not_exists,
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
