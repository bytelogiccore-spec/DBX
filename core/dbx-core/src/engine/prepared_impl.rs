// Prepared Statement Methods - Simplified Version
use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::ScalarValue;

impl Database {
    /// Prepare a SQL statement for efficient repeated execution
    /// 
    /// Stores the SQL template with ? placeholders.
    /// 
    /// # Example
    /// ```ignore
    /// let stmt = db.prepare("SELECT * FROM users WHERE id = ?")?;
    /// let result = db.execute_prepared(&stmt, &[ScalarValue::Int32(1)])?;
    /// ```
    pub fn prepare(&self, sql: &str) -> DbxResult<crate::engine::prepared::PreparedStatement> {
        // Count placeholders
        let placeholder_count = sql.matches('?').count();
        
        // Replace ? with NULL for parsing
        let mut temp_sql = sql.to_string();
        for _ in 0..placeholder_count {
            temp_sql = temp_sql.replacen('?', "NULL", 1);
        }
        
        // Parse and generate PhysicalPlan (once!)
        let ast = self.sql_parser.parse(&temp_sql)?;
        let logical_plan = self.sql_planner.plan(&ast)?;
        let physical_plan = self.sql_optimizer.optimize(logical_plan)?;
        
        // Store the actual PhysicalPlan
        Ok(crate::engine::prepared::PreparedStatement::new(
            sql.to_string(),
            physical_plan,
            placeholder_count,
        ))
    }
    
    /// Execute a prepared statement with bound parameters
    /// 
    /// Replaces ? placeholders with actual values and executes the SQL.
    /// 
    /// # Example
    /// ```ignore
    /// let stmt = db.prepare("INSERT INTO users VALUES (?, ?)");
    /// db.execute_prepared(&stmt, &[
    ///     ScalarValue::Int32(1),
    ///     ScalarValue::Utf8("Alice".to_string()),
    /// ])?;
    /// ```
    pub fn execute_prepared(
        &self,
        stmt: &crate::engine::prepared::PreparedStatement,
        params: &[ScalarValue],
    ) -> DbxResult<Vec<arrow::record_batch::RecordBatch>> {
        // Validate parameter count
        stmt.validate_params(params)?;
        
        // Bind parameters to the cached PhysicalPlan
        let bound_plan = bind_parameters_to_plan(&stmt.plan, params)?;
        
        // Execute using SQL interface (includes Columnar Cache!)
        self.sql_interface.execute_plan(&bound_plan)
    }
}

/// Bind parameters to NULL literals in PhysicalPlan
fn bind_parameters_to_plan(
    plan: &crate::sql::planner::types::PhysicalPlan,
    params: &[ScalarValue],
) -> DbxResult<crate::sql::planner::types::PhysicalPlan> {
    use crate::sql::planner::types::PhysicalPlan;
    
    let mut param_index = 0;
    
    match plan {
        PhysicalPlan::TableScan { table, projection, filter } => {
            let bound_filter = if let Some(f) = filter {
                Some(bind_expr(f, params, &mut param_index)?)
            } else {
                None
            };
            
            Ok(PhysicalPlan::TableScan {
                table: table.clone(),
                projection: projection.clone(),
                filter: bound_filter,
            })
        }
        other => Ok(other.clone()),
    }
}

/// Bind parameters to NULL literals in Expr
fn bind_expr(
    expr: &crate::sql::planner::types::Expr,
    params: &[ScalarValue],
    param_index: &mut usize,
) -> DbxResult<crate::sql::planner::types::Expr> {
    use crate::sql::planner::types::Expr;
    
    match expr {
        Expr::Literal(ScalarValue::Null) => {
            // Replace NULL with actual parameter
            if *param_index >= params.len() {
                return Err(DbxError::Schema(
                    format!("Not enough parameters: need {}, got {}", param_index + 1, params.len())
                ));
            }
            let result = Expr::Literal(params[*param_index].clone());
            *param_index += 1;
            Ok(result)
        }
        Expr::BinaryOp { left, op, right } => {
            let bound_left = bind_expr(left, params, param_index)?;
            let bound_right = bind_expr(right, params, param_index)?;
            
            Ok(Expr::BinaryOp {
                left: Box::new(bound_left),
                op: *op,
                right: Box::new(bound_right),
            })
        }
        Expr::Function { name, args } => {
            let bound_args: Result<Vec<_>, _> = args
                .iter()
                .map(|arg| bind_expr(arg, params, param_index))
                .collect();
            
            Ok(Expr::Function {
                name: name.clone(),
                args: bound_args?,
            })
        }
        Expr::InList { expr: inner, list, negated } => {
            let bound_inner = bind_expr(inner, params, param_index)?;
            let bound_list: Result<Vec<_>, _> = list
                .iter()
                .map(|item| bind_expr(item, params, param_index))
                .collect();
            
            Ok(Expr::InList {
                expr: Box::new(bound_inner),
                list: bound_list?,
                negated: *negated,
            })
        }
        Expr::IsNull(inner) => {
            let bound_inner = bind_expr(inner, params, param_index)?;
            Ok(Expr::IsNull(Box::new(bound_inner)))
        }
        Expr::IsNotNull(inner) => {
            let bound_inner = bind_expr(inner, params, param_index)?;
            Ok(Expr::IsNotNull(Box::new(bound_inner)))
        }
        other => Ok(other.clone()),
    }
}

/// Convert ScalarValue to SQL string representation
fn scalar_to_sql_string(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Boolean(b) => b.to_string(),
        ScalarValue::Int32(i) => i.to_string(),
        ScalarValue::Int64(i) => i.to_string(),
        ScalarValue::Float64(f) => f.to_string(),
        ScalarValue::Utf8(s) => format!("'{}'", s.replace('\'', "''")), // Escape single quotes
        ScalarValue::Null => "NULL".to_string(),
    }
}
