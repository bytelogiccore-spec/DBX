//! Stored Procedure Parser
//!
//! CREATE PROCEDURE, DROP PROCEDURE, CALL PROCEDURE SQL 파싱

use crate::automation::{ProcedureParameter, StoredProcedure};
use crate::error::{DbxError, DbxResult};
use crate::sql::StringCaseExt;

/// CREATE PROCEDURE 파싱
///
/// # 문법
/// ```sql
/// CREATE PROCEDURE procedure_name (param1 TYPE1, param2 TYPE2, ...)
/// BEGIN
///   sql_statement;
///   ...
/// END;
/// ```
pub fn parse_create_procedure(sql: &str) -> DbxResult<StoredProcedure> {
    let sql = sql.trim();

    if !sql.starts_with_ignore_ascii_case("CREATE PROCEDURE") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected CREATE PROCEDURE, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    // Procedure 이름 추출
    let name_start = sql.find("PROCEDURE").unwrap() + 9;
    let paren_pos = sql.find('(').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing parameter list".to_string(),
        context: "CREATE PROCEDURE parsing".to_string(),
    })?;
    let name = sql[name_start..paren_pos].trim().to_string();

    // 파라미터 추출
    let params_end = sql.find(')').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing closing parenthesis".to_string(),
        context: "CREATE PROCEDURE parsing".to_string(),
    })?;
    let params_str = sql[paren_pos + 1..params_end].trim();

    let mut parameters = Vec::new();
    if !params_str.is_empty() {
        for param in params_str.split(',') {
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.len() != 2 {
                return Err(DbxError::InvalidOperation {
                    message: format!("Invalid parameter: {}", param),
                    context: "CREATE PROCEDURE parameter parsing".to_string(),
                });
            }
            parameters.push(ProcedureParameter {
                name: parts[0].to_string(),
                data_type: parts[1].to_string(),
            });
        }
    }

    // BEGIN ... END 사이의 SQL 문장들 추출
    let begin_pos = sql
        .find("BEGIN")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing BEGIN".to_string(),
            context: "CREATE PROCEDURE parsing".to_string(),
        })?;
    let end_pos = sql.rfind("END").ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing END".to_string(),
        context: "CREATE PROCEDURE parsing".to_string(),
    })?;

    let body_sql = sql[begin_pos + 5..end_pos].trim();

    // 세미콜론으로 분리
    let body: Vec<String> = body_sql
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if body.is_empty() {
        return Err(DbxError::InvalidOperation {
            message: "Procedure body is empty".to_string(),
            context: "CREATE PROCEDURE parsing".to_string(),
        });
    }

    Ok(StoredProcedure::new(name, parameters, body))
}

/// DROP PROCEDURE 파싱
///
/// # 문법
/// ```sql
/// DROP PROCEDURE procedure_name;
/// ```
pub fn parse_drop_procedure(sql: &str) -> DbxResult<String> {
    let sql = sql.trim();

    if !sql.starts_with_ignore_ascii_case("DROP PROCEDURE") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected DROP PROCEDURE, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    let tokens: Vec<&str> = sql.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(DbxError::InvalidOperation {
            message: "Invalid DROP PROCEDURE syntax".to_string(),
            context: "SQL parsing".to_string(),
        });
    }

    let name = tokens[2].trim_end_matches(';').to_string();
    Ok(name)
}

/// CALL PROCEDURE 파싱
///
/// # 문법
/// ```sql
/// CALL procedure_name(arg1, arg2, ...);
/// ```
pub fn parse_call_procedure(sql: &str) -> DbxResult<(String, Vec<String>)> {
    let sql = sql.trim();

    if !sql.starts_with_ignore_ascii_case("CALL") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected CALL, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    // Procedure 이름 추출
    let name_start = 4; // "CALL".len()
    let paren_pos = sql.find('(').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing argument list".to_string(),
        context: "CALL PROCEDURE parsing".to_string(),
    })?;
    let name = sql[name_start..paren_pos].trim().to_string();

    // 인자 추출
    let args_end = sql.find(')').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing closing parenthesis".to_string(),
        context: "CALL PROCEDURE parsing".to_string(),
    })?;
    let args_str = sql[paren_pos + 1..args_end].trim();

    let mut arguments = Vec::new();
    if !args_str.is_empty() {
        for arg in args_str.split(',') {
            arguments.push(arg.trim().to_string());
        }
    }

    Ok((name, arguments))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_procedure_no_params() {
        let sql = r#"
            CREATE PROCEDURE reset_counter ()
            BEGIN
                UPDATE counters SET value = 0;
            END;
        "#;

        let proc = parse_create_procedure(sql).unwrap();
        assert_eq!(proc.name, "reset_counter");
        assert_eq!(proc.parameters.len(), 0);
        assert_eq!(proc.body.len(), 1);
    }

    #[test]
    fn test_parse_create_procedure_with_params() {
        let sql = r#"
            CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
            BEGIN
                UPDATE accounts SET balance = balance + amount WHERE id = user_id;
                INSERT INTO transactions VALUES (user_id, amount);
            END;
        "#;

        let proc = parse_create_procedure(sql).unwrap();
        assert_eq!(proc.name, "update_balance");
        assert_eq!(proc.parameters.len(), 2);
        assert_eq!(proc.parameters[0].name, "user_id");
        assert_eq!(proc.parameters[0].data_type, "INT");
        assert_eq!(proc.body.len(), 2);
    }

    #[test]
    fn test_parse_drop_procedure() {
        let sql = "DROP PROCEDURE update_balance;";
        let name = parse_drop_procedure(sql).unwrap();
        assert_eq!(name, "update_balance");
    }

    #[test]
    fn test_parse_call_procedure_no_args() {
        let sql = "CALL reset_counter();";
        let (name, args) = parse_call_procedure(sql).unwrap();
        assert_eq!(name, "reset_counter");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_parse_call_procedure_with_args() {
        let sql = "CALL update_balance(123, 100.50);";
        let (name, args) = parse_call_procedure(sql).unwrap();
        assert_eq!(name, "update_balance");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], "123");
        assert_eq!(args[1], "100.50");
    }
}
