//! Function Parser
//!
//! SQL CREATE FUNCTION, DROP FUNCTION 파싱

use crate::automation::UdfMetadata;
use crate::error::{DbxError, DbxResult};

/// CREATE FUNCTION 파싱
///
/// # 예제
/// ```sql
/// CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
/// BEGIN
///     RETURN a + b;
/// END;
/// ```
pub fn parse_create_function(sql: &str) -> DbxResult<UdfMetadata> {
    let sql = sql.trim();

    // CREATE FUNCTION 확인
    if !sql.to_uppercase().starts_with("CREATE FUNCTION") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected CREATE FUNCTION, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    // 함수 이름 추출
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(DbxError::InvalidOperation {
            message: "Invalid CREATE FUNCTION syntax".to_string(),
            context: "SQL parsing".to_string(),
        });
    }

    let name = tokens[2].trim_end_matches('(').to_string();

    // 파라미터 추출
    let params_start = sql.find('(').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing opening parenthesis".to_string(),
        context: "CREATE FUNCTION parsing".to_string(),
    })?;

    let params_end = sql.find(')').ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing closing parenthesis".to_string(),
        context: "CREATE FUNCTION parsing".to_string(),
    })?;

    let params_str = &sql[params_start + 1..params_end];
    let mut param_types = Vec::new();

    if !params_str.trim().is_empty() {
        for param in params_str.split(',') {
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.len() >= 2 {
                param_types.push(parts[1].to_string());
            }
        }
    }

    // RETURNS 타입 추출
    let returns_idx =
        sql.to_uppercase()
            .find("RETURNS")
            .ok_or_else(|| DbxError::InvalidOperation {
                message: "Missing RETURNS clause".to_string(),
                context: "CREATE FUNCTION parsing".to_string(),
            })?;

    let after_returns = &sql[returns_idx + 7..].trim();
    let return_type = after_returns
        .split_whitespace()
        .next()
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing return type".to_string(),
            context: "CREATE FUNCTION parsing".to_string(),
        })?
        .to_string();

    // UdfMetadata 생성
    Ok(UdfMetadata::new(
        &name,
        crate::automation::UdfType::Scalar, // 기본값으로 Scalar 사용
        param_types,
        return_type,
        false,
    ))
}

/// DROP FUNCTION 파싱
///
/// # 예제
/// ```sql
/// DROP FUNCTION add_numbers;
/// ```
pub fn parse_drop_function(sql: &str) -> DbxResult<String> {
    let sql = sql.trim();

    // DROP FUNCTION 확인
    if !sql.to_uppercase().starts_with("DROP FUNCTION") {
        return Err(DbxError::InvalidOperation {
            message: format!("Expected DROP FUNCTION, got: {}", sql),
            context: "SQL parsing".to_string(),
        });
    }

    // 함수 이름 추출
    let tokens: Vec<&str> = sql.split_whitespace().collect();
    if tokens.len() < 3 {
        return Err(DbxError::InvalidOperation {
            message: "Invalid DROP FUNCTION syntax".to_string(),
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
    fn test_parse_create_function() {
        let sql = r#"
            CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
            BEGIN
                RETURN a + b;
            END;
        "#;

        let metadata = parse_create_function(sql).unwrap();
        assert_eq!(metadata.name, "add_numbers");
        assert_eq!(metadata.param_types.len(), 2);
        assert_eq!(metadata.return_type, "INT");
    }

    #[test]
    fn test_parse_create_function_no_params() {
        let sql = r#"
            CREATE FUNCTION get_version () RETURNS VARCHAR
            BEGIN
                RETURN '1.0.0';
            END;
        "#;

        let metadata = parse_create_function(sql).unwrap();
        assert_eq!(metadata.name, "get_version");
        assert_eq!(metadata.param_types.len(), 0);
        assert_eq!(metadata.return_type, "VARCHAR");
    }

    #[test]
    fn test_parse_drop_function() {
        let sql = "DROP FUNCTION add_numbers;";
        let name = parse_drop_function(sql).unwrap();
        assert_eq!(name, "add_numbers");
    }

    #[test]
    fn test_parse_create_function_multiple_params() {
        let sql = r#"
            CREATE FUNCTION calculate (x DECIMAL, y DECIMAL, z INT) RETURNS DECIMAL
            BEGIN
                RETURN x + y * z;
            END;
        "#;

        let metadata = parse_create_function(sql).unwrap();
        assert_eq!(metadata.name, "calculate");
        assert_eq!(metadata.param_types.len(), 3);
        assert_eq!(metadata.param_types[0], "DECIMAL");
        assert_eq!(metadata.param_types[1], "DECIMAL");
        assert_eq!(metadata.param_types[2], "INT");
        assert_eq!(metadata.return_type, "DECIMAL");
    }
}
