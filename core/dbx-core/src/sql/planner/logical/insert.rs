//! INSERT statement planning

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::{Insert, SetExpr};

use super::LogicalPlanner;

impl LogicalPlanner {
    /// INSERT INTO → LogicalPlan 변환
    pub(super) fn plan_insert(&self, insert: &Insert) -> DbxResult<LogicalPlan> {
        let table = insert.table_name.to_string();

        // Extract column names
        let column_names: Vec<String> = insert.columns.iter().map(|c| c.value.clone()).collect();

        // Parse VALUES clause
        let values = if let Some(source) = &insert.source {
            match source.body.as_ref() {
                SetExpr::Values(values_set) => {
                    let mut rows = Vec::new();
                    for row in &values_set.rows {
                        let mut row_exprs = Vec::new();
                        for expr in row {
                            row_exprs.push(self.plan_expr(expr)?);
                        }
                        rows.push(row_exprs);
                    }
                    rows
                }
                _ => {
                    return Err(DbxError::SqlNotSupported {
                        feature: "INSERT with SELECT".to_string(),
                        hint: "Only INSERT INTO ... VALUES (...) is supported".to_string(),
                    });
                }
            }
        } else {
            return Err(DbxError::SqlNotSupported {
                feature: "INSERT without VALUES".to_string(),
                hint: "INSERT INTO ... VALUES (...) is required".to_string(),
            });
        };

        Ok(LogicalPlan::Insert {
            table,
            columns: column_names,
            values,
        })
    }
}
