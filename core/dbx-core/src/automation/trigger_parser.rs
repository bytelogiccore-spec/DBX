//! SQL Trigger Parser
//!
//! CREATE TRIGGER, DROP TRIGGER SQL 파싱

use crate::automation::{ForEachType, Trigger, TriggerOperation, TriggerTiming};
use crate::error::{DbxError, DbxResult};
use crate::sql::StringCaseExt;

/// CREATE TRIGGER 파싱
///
/// # 문법
/// ```sql
/// CREATE TRIGGER trigger_name
/// { BEFORE | AFTER } { INSERT | UPDATE | DELETE }
/// ON table_name
/// [ FOR EACH { ROW | STATEMENT } ]
/// [ WHEN ( condition ) ]
/// BEGIN
///   sql_statement;
///   ...
/// END;
/// ```
pub fn parse_create_trigger(sql: &str) -> DbxResult<Trigger> {
    // 간단한 수동 파싱 (sqlparser는 CREATE TRIGGER를 완전히 지원하지 않음)
    let sql = sql.trim();

    // CREATE TRIGGER 확인
    if !sql.starts_with_ignore_ascii_case("CREATE TRIGGER") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected CREATE TRIGGER, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    // 토큰 분리
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    if tokens.len() < 7 {
        return Err(DbxError::InvalidOperation {
            message: "Invalid CREATE TRIGGER syntax".to_string(),
            context: "SQL parsing".to_string(),
        });
    }

    // Trigger 이름 추출 (CREATE TRIGGER name)
    let name = tokens[2].to_string();

    // Timing 추출 (BEFORE/AFTER)
    let timing = match tokens[3].to_uppercase().as_str() {
        "BEFORE" => TriggerTiming::Before,
        "AFTER" => TriggerTiming::After,
        _ => {
            return Err(DbxError::InvalidOperation {
                message: format!("Invalid timing: {}", tokens[3]),
                context: "CREATE TRIGGER parsing".to_string(),
            });
        }
    };

    // Operation 추출 (INSERT/UPDATE/DELETE)
    let operation = match tokens[4].to_uppercase().as_str() {
        "INSERT" => TriggerOperation::Insert,
        "UPDATE" => TriggerOperation::Update,
        "DELETE" => TriggerOperation::Delete,
        _ => {
            return Err(DbxError::InvalidOperation {
                message: format!("Invalid operation: {}", tokens[4]),
                context: "CREATE TRIGGER parsing".to_string(),
            });
        }
    };

    // ON 확인
    if tokens[5].to_uppercase() != "ON" {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected ON, got: {}", tokens[5]),
            context: "CREATE TRIGGER parsing".to_string(),
        });
    }

    // 테이블 이름 추출
    let table = tokens[6].to_string();

    // FOR EACH 추출 (선택적)
    let mut for_each = ForEachType::Row; // 기본값
    let mut body_start_idx = 7;

    if tokens.len() > 9 && tokens[7].to_uppercase() == "FOR" && tokens[8].to_uppercase() == "EACH" {
        for_each = match tokens[9].to_uppercase().as_str() {
            "ROW" => ForEachType::Row,
            "STATEMENT" => ForEachType::Statement,
            _ => {
                return Err(DbxError::InvalidOperation {
                    message: format!("Invalid FOR EACH type: {}", tokens[9]),
                    context: "CREATE TRIGGER parsing".to_string(),
                });
            }
        };
        body_start_idx = 10;
    }

    // WHEN 조건 추출 (선택적)
    let mut condition = None;
    if tokens.len() > body_start_idx && tokens[body_start_idx].to_uppercase() == "WHEN" {
        // WHEN (condition) 형태 파싱
        let when_start = sql.find("WHEN").unwrap();
        let begin_pos = sql
            .find("BEGIN")
            .ok_or_else(|| DbxError::InvalidOperation {
                message: "Missing BEGIN".to_string(),
                context: "CREATE TRIGGER parsing".to_string(),
            })?;
        let condition_str = sql[when_start + 4..begin_pos].trim();

        // 괄호 제거
        let condition_str = condition_str
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim();
        condition = Some(condition_str.to_string());
    }

    // BEGIN ... END 사이의 SQL 문장들 추출
    let begin_pos = sql
        .find("BEGIN")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing BEGIN".to_string(),
            context: "CREATE TRIGGER parsing".to_string(),
        })?;
    let end_pos = sql.rfind("END").ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing END".to_string(),
        context: "CREATE TRIGGER parsing".to_string(),
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
            message: "Trigger body is empty".to_string(),
            context: "CREATE TRIGGER parsing".to_string(),
        });
    }

    Ok(Trigger::new(
        name, timing, operation, table, for_each, condition, body,
    ))
}

/// DROP TRIGGER 파싱
///
/// # 문법
/// ```sql
/// DROP TRIGGER trigger_name;
/// ```
pub fn parse_drop_trigger(sql: &str) -> DbxResult<String> {
    let sql = sql.trim();

    if !sql.starts_with_ignore_ascii_case("DROP TRIGGER") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected DROP TRIGGER, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    let tokens: Vec<&str> = sql.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(DbxError::InvalidOperation {
            message: "Invalid DROP TRIGGER syntax".to_string(),
            context: "SQL parsing".to_string(),
        });
    }

    let name = tokens[2].trim_end_matches(';').to_string();
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_trigger_basic() {
        let sql = r#"
            CREATE TRIGGER audit_trigger
            AFTER INSERT ON users
            BEGIN
                INSERT INTO audit_logs VALUES (NEW.id, 'INSERT');
            END;
        "#;

        let trigger = parse_create_trigger(sql).unwrap();
        assert_eq!(trigger.name, "audit_trigger");
        assert_eq!(trigger.timing, TriggerTiming::After);
        assert_eq!(trigger.operation, TriggerOperation::Insert);
        assert_eq!(trigger.table, "users");
        assert_eq!(trigger.for_each, ForEachType::Row);
        assert!(trigger.condition.is_none());
        assert_eq!(trigger.body.len(), 1);
    }

    #[test]
    fn test_parse_create_trigger_with_condition() {
        let sql = r#"
            CREATE TRIGGER check_age
            BEFORE INSERT ON users
            FOR EACH ROW
            WHEN (NEW.age > 18)
            BEGIN
                INSERT INTO adult_users VALUES (NEW.id);
            END;
        "#;

        let trigger = parse_create_trigger(sql).unwrap();
        assert_eq!(trigger.name, "check_age");
        assert_eq!(trigger.timing, TriggerTiming::Before);
        assert_eq!(trigger.for_each, ForEachType::Row);
        assert!(trigger.condition.is_some());
        assert_eq!(trigger.condition.unwrap(), "NEW.age > 18");
    }

    #[test]
    fn test_parse_create_trigger_multiple_statements() {
        let sql = r#"
            CREATE TRIGGER multi_action
            AFTER UPDATE ON products
            BEGIN
                UPDATE logs SET count = count + 1;
                INSERT INTO history VALUES (OLD.id, NEW.id);
            END;
        "#;

        let trigger = parse_create_trigger(sql).unwrap();
        assert_eq!(trigger.body.len(), 2);
    }

    #[test]
    fn test_parse_drop_trigger() {
        let sql = "DROP TRIGGER audit_trigger;";
        let name = parse_drop_trigger(sql).unwrap();
        assert_eq!(name, "audit_trigger");
    }

    #[test]
    fn test_parse_create_trigger_invalid() {
        let sql = "CREATE TABLE users (id INT);";
        assert!(parse_create_trigger(sql).is_err());
    }
}
