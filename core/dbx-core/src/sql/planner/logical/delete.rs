//! DELETE statement planning

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::Statement;

use super::LogicalPlanner;

impl LogicalPlanner {
    /// DELETE → LogicalPlan 변환
    pub(super) fn plan_delete(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        if let Statement::Delete(delete) = statement {
            // Extract tables from FromTable enum
            let tables = match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(t) => t,
                sqlparser::ast::FromTable::WithoutKeyword(t) => t,
            };
            let table_name = tables
                .first()
                .map(|t| t.relation.to_string())
                .unwrap_or_default();

            // Parse WHERE clause (optional)
            let filter = if let Some(sel) = &delete.selection {
                Some(self.plan_expr(sel)?)
            } else {
                None
            };

            Ok(LogicalPlan::Delete {
                table: table_name,
                filter,
            })
        } else {
            Err(DbxError::SqlNotSupported {
                feature: "DELETE statement".to_string(),
                hint: "Expected DELETE statement".to_string(),
            })
        }
    }
}
