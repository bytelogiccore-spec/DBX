//! SQL 논리 플래너 — AST → LogicalPlan 변환
//!
//! 이 모듈은 SQL AST를 논리 실행 계획으로 변환합니다.

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::Statement;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// 서브 모듈
pub mod custom;
pub mod ddl;
pub mod delete;
pub mod helpers;
pub mod insert;
pub mod select;
pub mod update;

// Re-export helper functions
pub use helpers::{convert_binary_op, extract_usize, match_scalar_function};

/// 논리 플랜 빌더 — AST → LogicalPlan 변환
pub struct LogicalPlanner {
    pub(crate) alias_map: Arc<RwLock<HashMap<String, Expr>>>,
}

impl LogicalPlanner {
    pub fn new() -> Self {
        Self {
            alias_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// SQL Statement → LogicalPlan 변환
    pub fn plan(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        match statement {
            Statement::Query(query) => self.plan_query(query),
            Statement::Insert { .. } => {
                // Extract Insert struct from Statement
                if let Statement::Insert(insert) = statement {
                    self.plan_insert(insert)
                } else {
                    unreachable!()
                }
            }
            Statement::Update { .. } => self.plan_update(statement),
            Statement::Delete { .. } => self.plan_delete(statement),
            Statement::Drop { .. }
            | Statement::CreateTable(_)
            | Statement::AlterTable { .. }
            | Statement::CreateIndex(_) => self.plan_ddl(statement),
            _ => {
                // Try to parse custom statements (CREATE FUNCTION, CREATE TRIGGER, CREATE JOB)
                let sql = format!("{:?}", statement);
                if sql.contains("CREATE FUNCTION")
                    || sql.contains("CREATE TRIGGER")
                    || sql.contains("CREATE JOB")
                {
                    self.parse_custom_statement(statement)
                } else {
                    Err(DbxError::SqlNotSupported {
                        feature: format!("Statement type: {:?}", statement),
                        hint: "Only SELECT, INSERT, UPDATE, DELETE, DROP TABLE, CREATE TABLE, ALTER TABLE, CREATE INDEX, DROP INDEX, CREATE FUNCTION, CREATE TRIGGER, and CREATE JOB are currently supported".to_string(),
                    })
                }
            }
        }
    }
}

impl Default for LogicalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    #[test]
    fn test_simple_select() {
        let sql = "SELECT id, name FROM users WHERE age > 18";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::Project { .. }));
    }

    #[test]
    fn test_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::Insert { .. }));
    }

    #[test]
    fn test_update() {
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::Update { .. }));
    }

    #[test]
    fn test_delete() {
        let sql = "DELETE FROM users WHERE id = 1";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::Delete { .. }));
    }

    #[test]
    fn test_create_table() {
        let sql = "CREATE TABLE users (id INT, name TEXT)";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::CreateTable { .. }));
    }

    #[test]
    fn test_drop_table() {
        let sql = "DROP TABLE users";
        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, sql).unwrap();
        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();
        assert!(matches!(plan, LogicalPlan::DropTable { .. }));
    }
}
