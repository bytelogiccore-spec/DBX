//! Stored Procedure Execution Engine
//!
//! Procedure 실행 엔진: CALL 명령으로 Procedure 실행

use crate::automation::StoredProcedure;
use crate::engine::Database;
use crate::error::{DbxError, DbxResult};
use std::collections::HashMap;

/// Procedure 실행 엔진
pub struct ProcedureExecutor {
    /// 등록된 Procedure 목록 (name -> procedure)
    procedures: HashMap<String, StoredProcedure>,
}

impl ProcedureExecutor {
    /// 새 Procedure 실행 엔진 생성
    pub fn new() -> Self {
        Self {
            procedures: HashMap::new(),
        }
    }

    /// Procedure 등록
    pub fn register(&mut self, procedure: StoredProcedure) {
        self.procedures.insert(procedure.name.clone(), procedure);
    }

    /// 모든 Procedure 등록
    pub fn register_all(&mut self, procedures: Vec<StoredProcedure>) {
        for proc in procedures {
            self.register(proc);
        }
    }

    /// Procedure 제거
    pub fn unregister(&mut self, name: &str) -> bool {
        self.procedures.remove(name).is_some()
    }

    /// 등록된 모든 Procedure 조회
    pub fn list_procedures(&self) -> Vec<&StoredProcedure> {
        self.procedures.values().collect()
    }

    /// Procedure 조회
    pub fn get(&self, name: &str) -> Option<&StoredProcedure> {
        self.procedures.get(name)
    }

    /// 파라미터 타입 검증
    ///
    /// SQL 타입과 인자 값이 호환되는지 검증합니다.
    fn validate_parameter_type(sql_type: &str, value: &str) -> Result<(), String> {
        let sql_type_upper = sql_type.to_uppercase();

        match sql_type_upper.as_str() {
            "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" => value
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| format!("Expected integer, got '{}'", value)),
            "REAL" | "FLOAT" | "DOUBLE" | "DECIMAL" | "NUMERIC" => value
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| format!("Expected number, got '{}'", value)),
            "TEXT" | "VARCHAR" | "CHAR" | "STRING" => {
                // 문자열은 항상 허용
                Ok(())
            }
            "BOOLEAN" | "BOOL" => {
                let val_lower = value.to_lowercase();
                if val_lower == "true"
                    || val_lower == "false"
                    || val_lower == "1"
                    || val_lower == "0"
                {
                    Ok(())
                } else {
                    Err(format!(
                        "Expected boolean (true/false/1/0), got '{}'",
                        value
                    ))
                }
            }
            _ => {
                // 알 수 없는 타입은 경고만 출력하고 통과
                #[cfg(debug_assertions)]
                eprintln!(
                    "[Procedure] Unknown parameter type '{}', skipping validation",
                    sql_type
                );
                Ok(())
            }
        }
    }

    /// Procedure 실행
    pub fn execute(&self, db: &Database, name: &str, arguments: &[String]) -> DbxResult<()> {
        // Procedure 조회
        let procedure = self
            .procedures
            .get(name)
            .ok_or_else(|| DbxError::InvalidOperation {
                message: format!("Procedure '{}' not found", name),
                context: "CALL PROCEDURE".to_string(),
            })?;

        // 파라미터 개수 검증
        if arguments.len() != procedure.parameters.len() {
            return Err(DbxError::InvalidOperation {
                message: format!(
                    "Procedure '{}' expects {} arguments, got {}",
                    name,
                    procedure.parameters.len(),
                    arguments.len()
                ),
                context: format!("CALL {}", name),
            });
        }

        // 파라미터 타입 검증
        for (i, param) in procedure.parameters.iter().enumerate() {
            let arg_value = &arguments[i];
            if let Err(e) = Self::validate_parameter_type(&param.data_type, arg_value) {
                return Err(DbxError::InvalidOperation {
                    message: format!(
                        "Procedure '{}' parameter '{}' type mismatch: {}",
                        name, param.name, e
                    ),
                    context: format!("CALL {}", name),
                });
            }
        }

        // Procedure body의 각 SQL 문장 실행
        for sql in &procedure.body {
            // 파라미터 바인딩 (Phase 11 - 향후 개선)
            // TODO: SQL 파라미터 바인딩 (prepared statement 방식)
            // 현재는 단순 문자열 치환
            let mut bound_sql = sql.clone();
            for (i, param) in procedure.parameters.iter().enumerate() {
                bound_sql = bound_sql.replace(&param.name, &arguments[i]);
            }

            // 실제 SQL 실행
            match db.execute_sql(&bound_sql) {
                Ok(_) => {
                    #[cfg(debug_assertions)]
                    println!(
                        "[Procedure] Successfully executed '{}': {}",
                        name, bound_sql
                    );
                }
                Err(e) => {
                    // Procedure 실행 실패 시 에러 반환
                    return Err(DbxError::InvalidOperation {
                        message: format!("Failed to execute procedure '{}': {}", name, e),
                        context: bound_sql,
                    });
                }
            }
        }

        Ok(())
    }
}

impl Default for ProcedureExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::ProcedureParameter;

    #[test]
    fn test_procedure_executor_register() {
        let mut executor = ProcedureExecutor::new();

        let proc = StoredProcedure::new("test_proc", vec![], vec!["SELECT 1".to_string()]);

        executor.register(proc);
        assert_eq!(executor.list_procedures().len(), 1);
    }

    #[test]
    fn test_procedure_executor_unregister() {
        let mut executor = ProcedureExecutor::new();

        let proc = StoredProcedure::new("test_proc", vec![], vec!["SELECT 1".to_string()]);

        executor.register(proc);
        assert_eq!(executor.list_procedures().len(), 1);

        let removed = executor.unregister("test_proc");
        assert!(removed);
        assert_eq!(executor.list_procedures().len(), 0);
    }

    #[test]
    fn test_procedure_executor_get() {
        let mut executor = ProcedureExecutor::new();

        let proc = StoredProcedure::new(
            "test_proc",
            vec![ProcedureParameter {
                name: "user_id".to_string(),
                data_type: "INT".to_string(),
            }],
            vec!["UPDATE users SET active = 1 WHERE id = user_id".to_string()],
        );

        executor.register(proc);

        let found = executor.get("test_proc");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test_proc");
    }
}
