use crate::error::DbxResult;
use crate::sql::planner::types::PhysicalPlan;
use crate::sql::planner::types::ScalarValue;

/// Prepared SQL statement for efficient repeated execution
/// 
/// Parses SQL once and caches the execution plan.
/// Parameters are bound at execution time using placeholders (?).
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    /// Original SQL string (for debugging)
    pub sql: String,
    
    /// Cached physical execution plan
    pub plan: PhysicalPlan,
    
    /// Number of parameters (placeholders) in the statement
    pub param_count: usize,
}

impl PreparedStatement {
    /// Create a new prepared statement
    pub fn new(sql: String, plan: PhysicalPlan, param_count: usize) -> Self {
        Self {
            sql,
            plan,
            param_count,
        }
    }
    
    /// Validate that the correct number of parameters are provided
    pub fn validate_params(&self, params: &[ScalarValue]) -> DbxResult<()> {
        if params.len() != self.param_count {
            return Err(crate::error::DbxError::SqlExecution {
                message: format!(
                    "Expected {} parameters, got {}",
                    self.param_count,
                    params.len()
                ),
                context: "PreparedStatement::validate_params".to_string(),
            });
        }
        Ok(())
    }
}
