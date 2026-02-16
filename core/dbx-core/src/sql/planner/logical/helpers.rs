//! SQL 논리 플래너 유틸리티 함수
//!
//! LogicalPlanner를 위한 헬퍼 함수들

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use sqlparser::ast::{BinaryOperator as SqlBinaryOp, Expr as SqlExpr};

/// SQL BinaryOperator → Logical BinaryOperator 변환
pub fn convert_binary_op(op: &SqlBinaryOp) -> DbxResult<BinaryOperator> {
    match op {
        SqlBinaryOp::Plus => Ok(BinaryOperator::Plus),
        SqlBinaryOp::Minus => Ok(BinaryOperator::Minus),
        SqlBinaryOp::Multiply => Ok(BinaryOperator::Multiply),
        SqlBinaryOp::Divide => Ok(BinaryOperator::Divide),
        SqlBinaryOp::Modulo => Ok(BinaryOperator::Modulo),
        SqlBinaryOp::Eq => Ok(BinaryOperator::Eq),
        SqlBinaryOp::NotEq => Ok(BinaryOperator::NotEq),
        SqlBinaryOp::Lt => Ok(BinaryOperator::Lt),
        SqlBinaryOp::LtEq => Ok(BinaryOperator::LtEq),
        SqlBinaryOp::Gt => Ok(BinaryOperator::Gt),
        SqlBinaryOp::GtEq => Ok(BinaryOperator::GtEq),
        SqlBinaryOp::And => Ok(BinaryOperator::And),
        SqlBinaryOp::Or => Ok(BinaryOperator::Or),
        _ => Err(DbxError::NotImplemented(format!(
            "Unsupported binary operator: {:?}",
            op
        ))),
    }
}

/// 함수 이름을 ScalarFunction enum으로 변환
pub fn match_scalar_function(name: &str) -> Option<ScalarFunction> {
    match name {
        // 문자열
        "UPPER" => Some(ScalarFunction::Upper),
        "LOWER" => Some(ScalarFunction::Lower),
        "LENGTH" => Some(ScalarFunction::Length),
        "SUBSTR" | "SUBSTRING" => Some(ScalarFunction::Substring),
        "CONCAT" => Some(ScalarFunction::Concat),
        "TRIM" => Some(ScalarFunction::Trim),

        // 수학
        "ABS" => Some(ScalarFunction::Abs),
        "ROUND" => Some(ScalarFunction::Round),
        "CEIL" => Some(ScalarFunction::Ceil),
        "FLOOR" => Some(ScalarFunction::Floor),
        "SQRT" => Some(ScalarFunction::Sqrt),
        "POWER" => Some(ScalarFunction::Power),

        // 날짜/시간
        "NOW" => Some(ScalarFunction::Now),
        "CURRENT_DATE" => Some(ScalarFunction::CurrentDate),
        "CURRENT_TIME" => Some(ScalarFunction::CurrentTime),
        "YEAR" => Some(ScalarFunction::Year),
        "MONTH" => Some(ScalarFunction::Month),
        "DAY" => Some(ScalarFunction::Day),
        "HOUR" => Some(ScalarFunction::Hour),
        "MINUTE" => Some(ScalarFunction::Minute),
        "SECOND" => Some(ScalarFunction::Second),

        _ => None,
    }
}

/// Extract a usize from a SQL literal expression (for LIMIT/OFFSET).
pub fn extract_usize(expr: &SqlExpr) -> DbxResult<usize> {
    match expr {
        SqlExpr::Value(sqlparser::ast::Value::Number(n, _)) => n.parse::<usize>().map_err(|_| {
            DbxError::Schema(format!(
                "LIMIT/OFFSET value must be a positive integer, got: {}",
                n
            ))
        }),
        _ => Err(DbxError::NotImplemented(format!(
            "Non-literal LIMIT/OFFSET expression: {:?}",
            expr
        ))),
    }
}
