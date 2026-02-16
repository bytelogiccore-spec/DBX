//! UPDATE statement planning

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::Statement;

use super::LogicalPlanner;

impl LogicalPlanner {
    /// UPDATE → LogicalPlan 변환
    pub(super) fn plan_update(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        if let Statement::Update {
            table,
            assignments,
            selection,
            ..
        } = statement
        {
            let table_name = table.relation.to_string();

            // Parse SET assignments
            let mut parsed_assignments = Vec::new();
            for assignment in assignments {
                let column = assignment.target.to_string();
                let value = self.plan_expr(&assignment.value)?;
                parsed_assignments.push((column, value));
            }

            // Parse WHERE clause (optional)
            let filter = if let Some(sel) = selection {
                Some(self.plan_expr(sel)?)
            } else {
                None
            };

            Ok(LogicalPlan::Update {
                table: table_name,
                assignments: parsed_assignments,
                filter,
            })
        } else {
            Err(DbxError::SqlNotSupported {
                feature: "UPDATE statement".to_string(),
                hint: "Expected UPDATE statement".to_string(),
            })
        }
    }
}
