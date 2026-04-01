//! Custom statement planning - CREATE FUNCTION, CREATE TRIGGER, CREATE JOB

use crate::error::{DbxError, DbxResult};
use crate::sql::StringCaseExt;
use crate::sql::planner::types::*;
use sqlparser::ast::Statement;

use super::LogicalPlanner;

impl LogicalPlanner {
    /// Custom statement 파싱 (CREATE FUNCTION, CREATE TRIGGER, CREATE JOB)
    pub(super) fn parse_custom_statement(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        // Statement를 문자열로 변환
        let sql = format!("{}", statement);
        let sql_upper = sql.to_uppercase();

        // CREATE FUNCTION 파싱
        if sql_upper.contains("CREATE FUNCTION") {
            return self.parse_create_function(&sql);
        }

        // CREATE TRIGGER 파싱
        if sql_upper.contains("CREATE TRIGGER") {
            return self.parse_create_trigger(&sql);
        }

        // CREATE JOB 파싱
        if sql_upper.contains("CREATE JOB") {
            return self.parse_create_job(&sql);
        }

        Err(DbxError::SqlNotSupported {
            feature: "Custom statement".to_string(),
            hint: "Only CREATE FUNCTION, CREATE TRIGGER, and CREATE JOB are supported".to_string(),
        })
    }

    /// CREATE FUNCTION 파싱
    fn parse_create_function(&self, sql: &str) -> DbxResult<LogicalPlan> {
        // 간단한 문자열 파싱 (실제로는 더 정교한 파서 필요)
        // 예: CREATE FUNCTION add(a INT, b INT) RETURNS INT LANGUAGE rust AS 'fn add(a: i32, b: i32) -> i32 { a + b }'

        let sql_upper = sql.to_uppercase();

        // 함수 이름 추출 - FUNCTION 다음의 첫 단어
        let name = sql_upper
            .split("FUNCTION")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(ToString::to_string)
            .ok_or_else(|| DbxError::SqlParse {
                message: "Failed to parse function name".to_string(),
                sql: sql.to_string(),
            })?;

        // 파라미터 추출: FUNCTION name(a INT, b INT) 형식에서 괄호 내 파라미터 파싱
        let params = if let (Some(open), Some(close)) = (sql.find('('), sql.find(')')) {
            let param_str = &sql[open + 1..close];
            param_str
                .split(',')
                .filter_map(|p| {
                    let parts: Vec<&str> = p.split_whitespace().collect();
                    if parts.len() >= 2 {
                        Some((parts[0].to_string(), parts[1].to_uppercase()))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // 반환 타입 추출
        let return_type = sql_upper
            .split("RETURNS")
            .nth(1)
            .and_then(|s| s.split("LANGUAGE").next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "VOID".to_string());

        // 언어 추출
        let language = sql_upper
            .split("LANGUAGE")
            .nth(1)
            .and_then(|s| s.split("AS").next())
            .map(|s| s.trim().to_lowercase())
            .unwrap_or_else(|| "sql".to_string());

        // 함수 본문 추출
        let body = sql
            .split("AS")
            .nth(1)
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .unwrap_or_default();

        Ok(LogicalPlan::CreateFunction {
            name,
            params,
            return_type,
            language,
            body,
        })
    }

    /// CREATE TRIGGER 파싱
    fn parse_create_trigger(&self, sql: &str) -> DbxResult<LogicalPlan> {
        // 예: CREATE TRIGGER audit_log AFTER INSERT ON users FOR EACH ROW EXECUTE FUNCTION log_insert()

        // 트리거 이름 추출
        let name = sql
            .split("TRIGGER")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.trim().to_string())
            .ok_or_else(|| DbxError::SqlParse {
                message: "Failed to parse trigger name".to_string(),
                sql: sql.to_string(),
            })?;

        // 타이밍 추출 (BEFORE/AFTER)
        let timing = if sql.contains_ignore_ascii_case("BEFORE") {
            TriggerTiming::Before
        } else {
            TriggerTiming::After
        };

        // 이벤트 추출 (INSERT/UPDATE/DELETE)
        let event = if sql.contains_ignore_ascii_case("INSERT") {
            TriggerEventKind::Insert
        } else if sql.contains_ignore_ascii_case("UPDATE") {
            TriggerEventKind::Update
        } else {
            TriggerEventKind::Delete
        };

        // 테이블 이름 추출
        let table = sql
            .split("ON")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // FOR EACH 추출
        let for_each = if sql.contains_ignore_ascii_case("FOR EACH ROW") {
            ForEachKind::Row
        } else {
            ForEachKind::Statement
        };

        // 함수 이름 추출
        let function = sql
            .split("FUNCTION")
            .nth(1)
            .and_then(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Ok(LogicalPlan::CreateTrigger {
            name,
            timing,
            event,
            table,
            for_each,
            function,
        })
    }

    /// CREATE JOB 파싱
    fn parse_create_job(&self, sql: &str) -> DbxResult<LogicalPlan> {
        // 예: CREATE JOB cleanup_old_data SCHEDULE '0 0 * * *' EXECUTE FUNCTION cleanup()

        // 작업 이름 추출
        let name = sql
            .split("JOB")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .map(|s| s.trim().to_string())
            .ok_or_else(|| DbxError::SqlParse {
                message: "Failed to parse job name".to_string(),
                sql: sql.to_string(),
            })?;

        // 스케줄 추출
        let schedule = sql
            .split("SCHEDULE")
            .nth(1)
            .and_then(|s| s.split("EXECUTE").next())
            .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
            .unwrap_or_default();

        // 함수 이름 추출
        let function = sql
            .split("FUNCTION")
            .nth(1)
            .and_then(|s| s.split('(').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        Ok(LogicalPlan::CreateJob {
            name,
            schedule,
            function,
        })
    }
}
