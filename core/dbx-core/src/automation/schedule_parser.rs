//! SQL Schedule Parser
//!
//! CREATE SCHEDULE 및 DROP SCHEDULE SQL 파싱

use crate::automation::Schedule;
use crate::error::{DbxError, DbxResult};

/// Parse CREATE SCHEDULE statement
///
/// # Syntax
/// ```sql
/// CREATE SCHEDULE schedule_name
/// EVERY 'cron_expression'
/// BEGIN
///     sql_statement;
///     ...
/// END;
/// ```
///
/// # Example
/// ```
/// use dbx_core::automation::parse_create_schedule;
///
/// let sql = r#"
///     CREATE SCHEDULE cleanup_job
///     EVERY '0 0 * * *'
///     BEGIN
///         DELETE FROM logs WHERE created_at < NOW() - INTERVAL 30 DAY;
///     END;
/// "#;
///
/// let schedule = parse_create_schedule(sql).unwrap();
/// assert_eq!(schedule.name, "cleanup_job");
/// assert_eq!(schedule.cron_expr, "0 0 * * *");
/// ```
pub fn parse_create_schedule(sql: &str) -> DbxResult<Schedule> {
    let sql = sql.trim();

    // Extract schedule name
    let name_start = sql
        .find("CREATE SCHEDULE")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing CREATE SCHEDULE".to_string(),
            context: sql.to_string(),
        })?
        + "CREATE SCHEDULE".len();

    let name_end = sql[name_start..]
        .find("EVERY")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing EVERY clause".to_string(),
            context: sql.to_string(),
        })?
        + name_start;

    let name = sql[name_start..name_end].trim().to_string();

    // Extract cron expression
    let cron_start = sql
        .find("EVERY")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing EVERY clause".to_string(),
            context: sql.to_string(),
        })?
        + "EVERY".len();

    let cron_end = sql[cron_start..]
        .find("BEGIN")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing BEGIN".to_string(),
            context: sql.to_string(),
        })?
        + cron_start;

    let cron_expr = sql[cron_start..cron_end]
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string();

    // Extract SQL body
    let body_start = sql
        .find("BEGIN")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing BEGIN".to_string(),
            context: sql.to_string(),
        })?
        + "BEGIN".len();

    let body_end = sql.rfind("END").ok_or_else(|| DbxError::InvalidOperation {
        message: "Missing END".to_string(),
        context: sql.to_string(),
    })?;

    let body_str = sql[body_start..body_end].trim();

    // Split by semicolon
    let sql_body: Vec<String> = body_str
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(Schedule::new(name, cron_expr, sql_body))
}

/// Parse DROP SCHEDULE statement
///
/// # Syntax
/// ```sql
/// DROP SCHEDULE schedule_name;
/// ```
///
/// # Example
/// ```
/// use dbx_core::automation::parse_drop_schedule;
///
/// let sql = "DROP SCHEDULE cleanup_job;";
/// let name = parse_drop_schedule(sql).unwrap();
/// assert_eq!(name, "cleanup_job");
/// ```
pub fn parse_drop_schedule(sql: &str) -> DbxResult<String> {
    let sql = sql.trim();

    let name_start = sql
        .find("DROP SCHEDULE")
        .ok_or_else(|| DbxError::InvalidOperation {
            message: "Missing DROP SCHEDULE".to_string(),
            context: sql.to_string(),
        })?
        + "DROP SCHEDULE".len();

    let name = sql[name_start..]
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_schedule() {
        let sql = r#"
            CREATE SCHEDULE cleanup_job
            EVERY '0 0 * * *'
            BEGIN
                DELETE FROM logs WHERE created_at < NOW() - INTERVAL 30 DAY;
                UPDATE stats SET last_cleanup = NOW();
            END;
        "#;

        let schedule = parse_create_schedule(sql).unwrap();

        assert_eq!(schedule.name, "cleanup_job");
        assert_eq!(schedule.cron_expr, "0 0 * * *");
        assert_eq!(schedule.sql_body.len(), 2);
        assert!(schedule.enabled);
    }

    #[test]
    fn test_parse_create_schedule_single_statement() {
        let sql = r#"
            CREATE SCHEDULE refresh_stats
            EVERY '*/5 * * * *'
            BEGIN
                UPDATE stats SET count = count + 1;
            END;
        "#;

        let schedule = parse_create_schedule(sql).unwrap();

        assert_eq!(schedule.name, "refresh_stats");
        assert_eq!(schedule.cron_expr, "*/5 * * * *");
        assert_eq!(schedule.sql_body.len(), 1);
    }

    #[test]
    fn test_parse_drop_schedule() {
        let sql = "DROP SCHEDULE cleanup_job;";
        let name = parse_drop_schedule(sql).unwrap();
        assert_eq!(name, "cleanup_job");
    }

    #[test]
    fn test_parse_drop_schedule_no_semicolon() {
        let sql = "DROP SCHEDULE refresh_stats";
        let name = parse_drop_schedule(sql).unwrap();
        assert_eq!(name, "refresh_stats");
    }
}
