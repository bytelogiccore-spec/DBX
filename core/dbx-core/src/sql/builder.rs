//! Query Builder — Fluent 스타일 API
//!
//! Dapper 스타일 파라미터 바인딩 지원:
//! - Positional: `$1, $2, ...`
//! - Named: `:name, :age, ...`

use crate::api::FromRow;
use crate::engine::Database;
use crate::error::{DbxError, DbxResult};
use std::marker::PhantomData;

/// 쿼리 파라미터 값
#[derive(Debug, Clone)]
pub enum ScalarValue {
    Null,
    Int32(i32),
    Int64(i64),
    Float64(f64),
    Utf8(String),
    Boolean(bool),
}

impl ScalarValue {
    /// SQL 리터럴 문자열로 변환 (placeholder 치환용)
    pub fn to_sql_literal(&self) -> String {
        match self {
            ScalarValue::Null => "NULL".to_string(),
            ScalarValue::Int32(v) => v.to_string(),
            ScalarValue::Int64(v) => v.to_string(),
            ScalarValue::Float64(v) => format!("{v}"),
            ScalarValue::Utf8(v) => format!("'{}'", v.replace('\'', "''")),
            ScalarValue::Boolean(v) => {
                if *v {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
        }
    }
}

/// Named 파라미터 엔트리
#[derive(Debug, Clone)]
struct NamedParam {
    name: String,
    value: ScalarValue,
}

/// SQL에 파라미터를 적용하여 최종 실행 가능한 SQL 생성
/// 문자열 리터럴('...') 및 식별자("...") 내부에 있는 placeholder는 무시합니다.
fn apply_params(sql: &str, positional: &[ScalarValue], named: &[NamedParam]) -> DbxResult<String> {
    if !positional.is_empty() && !named.is_empty() {
        return Err(DbxError::InvalidOperation {
            message: "positional과 named 파라미터를 동시에 사용할 수 없습니다".to_string(),
            context: "apply_params".to_string(),
        });
    }

    let mut result = String::with_capacity(sql.len() + 64);
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        if in_single_quote {
            result.push(c);
            if c == '\'' {
                in_single_quote = false;
            }
            continue;
        } else if in_double_quote {
            result.push(c);
            if c == '"' {
                in_double_quote = false;
            }
            continue;
        }

        match c {
            '\'' => {
                in_single_quote = true;
                result.push(c);
            }
            '"' => {
                in_double_quote = true;
                result.push(c);
            }
            '$' if !positional.is_empty() => {
                let mut num_str = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_ascii_digit() {
                        num_str.push(next_c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(idx) = num_str.parse::<usize>() {
                    if idx > 0 && idx <= positional.len() {
                        result.push_str(&positional[idx - 1].to_sql_literal());
                    } else {
                        result.push('$');
                        result.push_str(&num_str);
                    }
                } else {
                    result.push('$');
                    result.push_str(&num_str);
                }
            }
            ':' if !named.is_empty() => {
                let mut name = String::new();
                while let Some(&next_c) = chars.peek() {
                    if next_c.is_ascii_alphanumeric() || next_c == '_' {
                        name.push(next_c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !name.is_empty() {
                    let mut found = false;
                    for np in named {
                        if np.name == name {
                            result.push_str(&np.value.to_sql_literal());
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        result.push(':');
                        result.push_str(&name);
                    }
                } else {
                    result.push(':');
                }
            }
            _ => {
                result.push(c);
            }
        }
    }

    Ok(result)
}

/// Query Builder — 여러 행 반환
pub struct Query<'a, T> {
    db: &'a Database,
    sql: String,
    params: Vec<ScalarValue>,
    named_params: Vec<NamedParam>,
    _marker: PhantomData<T>,
}

impl<'a, T: FromRow> Query<'a, T> {
    pub fn new(db: &'a Database, sql: impl Into<String>) -> Self {
        Self {
            db,
            sql: sql.into(),
            params: Vec::new(),
            named_params: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Positional 파라미터 바인딩 ($1, $2, ...)
    pub fn bind<V: IntoParam>(mut self, value: V) -> Self {
        self.params.push(value.into_scalar());
        self
    }

    /// Named 파라미터 바인딩 (:name, :age, ...)
    pub fn param<V: IntoParam>(mut self, name: &str, value: V) -> Self {
        self.named_params.push(NamedParam {
            name: name.to_string(),
            value: value.into_scalar(),
        });
        self
    }

    /// 모든 행 반환
    pub fn fetch_all(self) -> DbxResult<Vec<T>> {
        let final_sql = apply_params(&self.sql, &self.params, &self.named_params)?;
        let batches = self.db.execute_sql(&final_sql)?;
        let mut rows = Vec::new();
        for batch in &batches {
            for row_idx in 0..batch.num_rows() {
                rows.push(T::from_row(batch, row_idx)?);
            }
        }
        Ok(rows)
    }

    /// 첫 번째 행만 반환 (나머지 무시)
    pub fn fetch_first(self) -> DbxResult<Option<T>> {
        let final_sql = apply_params(&self.sql, &self.params, &self.named_params)?;
        let batches = self.db.execute_sql(&final_sql)?;
        for batch in &batches {
            if batch.num_rows() > 0 {
                return Ok(Some(T::from_row(batch, 0)?));
            }
        }
        Ok(None)
    }
}

/// Query Builder — 단일 행 반환 (없으면 에러)
pub struct QueryOne<'a, T> {
    db: &'a Database,
    sql: String,
    params: Vec<ScalarValue>,
    named_params: Vec<NamedParam>,
    _marker: PhantomData<T>,
}

impl<'a, T: FromRow> QueryOne<'a, T> {
    pub fn new(db: &'a Database, sql: impl Into<String>) -> Self {
        Self {
            db,
            sql: sql.into(),
            params: Vec::new(),
            named_params: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn bind<V: IntoParam>(mut self, value: V) -> Self {
        self.params.push(value.into_scalar());
        self
    }

    pub fn param<V: IntoParam>(mut self, name: &str, value: V) -> Self {
        self.named_params.push(NamedParam {
            name: name.to_string(),
            value: value.into_scalar(),
        });
        self
    }

    /// 정확히 1개 행 반환 (0개 또는 2개 이상이면 에러)
    pub fn fetch(self) -> DbxResult<T> {
        let final_sql = apply_params(&self.sql, &self.params, &self.named_params)?;
        let batches = self.db.execute_sql(&final_sql)?;
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        if total_rows == 0 {
            return Err(DbxError::KeyNotFound);
        }
        if total_rows > 1 {
            return Err(DbxError::InvalidOperation {
                message: format!("expected 1 row, got {total_rows}"),
                context: "QueryOne::fetch".to_string(),
            });
        }
        T::from_row(&batches[0], 0)
    }
}

/// Query Builder — 단일 행 반환 (없으면 None)
pub struct QueryOptional<'a, T> {
    db: &'a Database,
    sql: String,
    params: Vec<ScalarValue>,
    named_params: Vec<NamedParam>,
    _marker: PhantomData<T>,
}

impl<'a, T: FromRow> QueryOptional<'a, T> {
    pub fn new(db: &'a Database, sql: impl Into<String>) -> Self {
        Self {
            db,
            sql: sql.into(),
            params: Vec::new(),
            named_params: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn bind<V: IntoParam>(mut self, value: V) -> Self {
        self.params.push(value.into_scalar());
        self
    }

    pub fn param<V: IntoParam>(mut self, name: &str, value: V) -> Self {
        self.named_params.push(NamedParam {
            name: name.to_string(),
            value: value.into_scalar(),
        });
        self
    }

    pub fn fetch(self) -> DbxResult<Option<T>> {
        let final_sql = apply_params(&self.sql, &self.params, &self.named_params)?;
        let batches = self.db.execute_sql(&final_sql)?;
        for batch in &batches {
            if batch.num_rows() > 0 {
                return Ok(Some(T::from_row(batch, 0)?));
            }
        }
        Ok(None)
    }
}

/// Query Builder — Scalar 값 반환
pub struct QueryScalar<'a, T> {
    db: &'a Database,
    sql: String,
    params: Vec<ScalarValue>,
    _marker: PhantomData<T>,
}

impl<'a, T: FromScalar> QueryScalar<'a, T> {
    pub fn new(db: &'a Database, sql: impl Into<String>) -> Self {
        Self {
            db,
            sql: sql.into(),
            params: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn bind<V: IntoParam>(mut self, value: V) -> Self {
        self.params.push(value.into_scalar());
        self
    }

    pub fn fetch(self) -> DbxResult<T> {
        let final_sql = apply_params(&self.sql, &self.params, &[])?;
        let batches = self.db.execute_sql(&final_sql)?;
        for batch in &batches {
            if batch.num_rows() > 0 && batch.num_columns() > 0 {
                let col = batch.column(0);
                let sv = crate::storage::columnar::ScalarValue::from_array(col, 0)?;
                let qsv = scalar_to_query_scalar(&sv);
                return T::from_scalar(&qsv);
            }
        }
        Err(DbxError::KeyNotFound)
    }
}

/// Execute Builder — INSERT/UPDATE/DELETE
pub struct Execute<'a> {
    db: &'a Database,
    sql: String,
    params: Vec<ScalarValue>,
    named_params: Vec<NamedParam>,
}

impl<'a> Execute<'a> {
    pub fn new(db: &'a Database, sql: impl Into<String>) -> Self {
        Self {
            db,
            sql: sql.into(),
            params: Vec::new(),
            named_params: Vec::new(),
        }
    }

    pub fn bind<V: IntoParam>(mut self, value: V) -> Self {
        self.params.push(value.into_scalar());
        self
    }

    pub fn param<V: IntoParam>(mut self, name: &str, value: V) -> Self {
        self.named_params.push(NamedParam {
            name: name.to_string(),
            value: value.into_scalar(),
        });
        self
    }

    /// INSERT/UPDATE/DELETE 실행 → 영향받은 행 수
    pub fn run(self) -> DbxResult<usize> {
        let final_sql = apply_params(&self.sql, &self.params, &self.named_params)?;
        let batches = self.db.execute_sql(&final_sql)?;
        Ok(batches.iter().map(|b| b.num_rows()).sum())
    }
}

/// 파라미터 변환 트레이트
pub trait IntoParam {
    fn into_scalar(self) -> ScalarValue;
}

/// Scalar 값 추출 트레이트
pub trait FromScalar: Sized {
    fn from_scalar(value: &ScalarValue) -> DbxResult<Self>;
}

/// Convert columnar::ScalarValue to query::ScalarValue
fn scalar_to_query_scalar(sv: &crate::storage::columnar::ScalarValue) -> ScalarValue {
    use crate::storage::columnar::ScalarValue as CSV;
    match sv {
        CSV::Null => ScalarValue::Null,
        CSV::Int32(v) => ScalarValue::Int32(*v),
        CSV::Int64(v) => ScalarValue::Int64(*v),
        CSV::Float64(v) => ScalarValue::Float64(*v),
        CSV::Utf8(v) => ScalarValue::Utf8(v.clone()),
        CSV::Boolean(v) => ScalarValue::Boolean(*v),
        CSV::Binary(_) => {
            // Binary type is not supported in query builder ScalarValue
            // This should not happen in normal query operations
            ScalarValue::Null
        }
    }
}

// 기본 타입 구현
impl IntoParam for i32 {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Int32(self)
    }
}

impl IntoParam for i64 {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Int64(self)
    }
}

impl IntoParam for f64 {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Float64(self)
    }
}

impl IntoParam for &str {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Utf8(self.to_string())
    }
}

impl IntoParam for String {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Utf8(self)
    }
}

impl IntoParam for bool {
    fn into_scalar(self) -> ScalarValue {
        ScalarValue::Boolean(self)
    }
}

impl<T: IntoParam> IntoParam for Option<T> {
    fn into_scalar(self) -> ScalarValue {
        match self {
            Some(v) => v.into_scalar(),
            None => ScalarValue::Null,
        }
    }
}

// FromScalar 구현
impl FromScalar for i64 {
    fn from_scalar(value: &ScalarValue) -> DbxResult<Self> {
        match value {
            ScalarValue::Int64(v) => Ok(*v),
            _ => Err(crate::error::DbxError::TypeMismatch {
                expected: "Int64".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl FromScalar for i32 {
    fn from_scalar(value: &ScalarValue) -> DbxResult<Self> {
        match value {
            ScalarValue::Int32(v) => Ok(*v),
            _ => Err(crate::error::DbxError::TypeMismatch {
                expected: "Int32".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

impl FromScalar for f64 {
    fn from_scalar(value: &ScalarValue) -> DbxResult<Self> {
        match value {
            ScalarValue::Float64(v) => Ok(*v),
            _ => Err(crate::error::DbxError::TypeMismatch {
                expected: "Float64".to_string(),
                actual: format!("{:?}", value),
            }),
        }
    }
}

// Database에 Query Builder 메서드 추가
impl Database {
    /// SELECT 쿼리 — 여러 행 반환
    pub fn query<T: FromRow>(&self, sql: impl Into<String>) -> Query<'_, T> {
        Query::new(self, sql)
    }

    /// SELECT 쿼리 — 단일 행 반환 (없으면 에러)
    pub fn query_one<T: FromRow>(&self, sql: impl Into<String>) -> QueryOne<'_, T> {
        QueryOne::new(self, sql)
    }

    /// SELECT 쿼리 — 단일 행 반환 (없으면 None)
    pub fn query_optional<T: FromRow>(&self, sql: impl Into<String>) -> QueryOptional<'_, T> {
        QueryOptional::new(self, sql)
    }

    /// SELECT 쿼리 — 단일 스칼라 값 반환
    pub fn query_scalar<T: FromScalar>(&self, sql: impl Into<String>) -> QueryScalar<'_, T> {
        QueryScalar::new(self, sql)
    }

    /// INSERT/UPDATE/DELETE — 영향받은 행 수 반환
    pub fn execute(&self, sql: impl Into<String>) -> Execute<'_> {
        Execute::new(self, sql)
    }
}

// ════════════════════════════════════════════
// DatabaseQuery Trait Implementation
// ════════════════════════════════════════════

impl crate::traits::DatabaseQuery for Database {
    // Query Builder methods are already implemented in impl Database block above
    // No additional implementation needed - Trait is satisfied by existing methods
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_to_sql_literal() {
        assert_eq!(ScalarValue::Null.to_sql_literal(), "NULL");
        assert_eq!(ScalarValue::Int32(42).to_sql_literal(), "42");
        assert_eq!(ScalarValue::Int64(100).to_sql_literal(), "100");
        assert_eq!(ScalarValue::Float64(3.1).to_sql_literal(), "3.1");
        assert_eq!(
            ScalarValue::Utf8("hello".into()).to_sql_literal(),
            "'hello'"
        );
        assert_eq!(ScalarValue::Boolean(true).to_sql_literal(), "TRUE");
        assert_eq!(ScalarValue::Boolean(false).to_sql_literal(), "FALSE");
    }

    #[test]
    fn test_sql_literal_single_quote_escape() {
        // SQL injection 방어: single quote → 이스케이프
        assert_eq!(
            ScalarValue::Utf8("O'Brien".into()).to_sql_literal(),
            "'O''Brien'"
        );
    }

    #[test]
    fn test_apply_params_positional() {
        let sql = "SELECT * FROM users WHERE id = $1 AND age > $2";
        let params = vec![ScalarValue::Int32(42), ScalarValue::Int64(18)];
        let result = apply_params(sql, &params, &[]).unwrap();
        assert_eq!(result, "SELECT * FROM users WHERE id = 42 AND age > 18");
    }

    #[test]
    fn test_apply_params_string() {
        let sql = "SELECT * FROM users WHERE name = $1";
        let params = vec![ScalarValue::Utf8("Alice".into())];
        let result = apply_params(sql, &params, &[]).unwrap();
        assert_eq!(result, "SELECT * FROM users WHERE name = 'Alice'");
    }

    #[test]
    fn test_apply_params_null_bool() {
        let sql = "SELECT * FROM t WHERE a = $1 AND b = $2 AND c = $3";
        let params = vec![
            ScalarValue::Null,
            ScalarValue::Boolean(true),
            ScalarValue::Boolean(false),
        ];
        let result = apply_params(sql, &params, &[]).unwrap();
        assert_eq!(
            result,
            "SELECT * FROM t WHERE a = NULL AND b = TRUE AND c = FALSE"
        );
    }

    #[test]
    fn test_apply_params_reverse_order_safety() {
        // $10이 $1로 잘못 매치되지 않아야 함
        let sql = "SELECT $1, $10";
        let mut params = vec![ScalarValue::Int32(1)];
        for i in 2..=10 {
            params.push(ScalarValue::Int32(i));
        }
        let result = apply_params(sql, &params, &[]).unwrap();
        assert_eq!(result, "SELECT 1, 10");
    }

    #[test]
    fn test_apply_named_params() {
        let sql = "SELECT * FROM users WHERE name = :name AND age > :age";
        let named = vec![
            NamedParam {
                name: "name".into(),
                value: ScalarValue::Utf8("Alice".into()),
            },
            NamedParam {
                name: "age".into(),
                value: ScalarValue::Int32(18),
            },
        ];
        let result = apply_params(sql, &[], &named).unwrap();
        assert_eq!(
            result,
            "SELECT * FROM users WHERE name = 'Alice' AND age > 18"
        );
    }

    #[test]
    fn test_apply_named_params_ignores_strings() {
        let sql = "SELECT * FROM users WHERE txt = 'cost: $1, name: :name' AND id = $1";
        let positional = vec![ScalarValue::Int32(5)];
        let result = apply_params(sql, &positional, &[]).unwrap();
        assert_eq!(result, "SELECT * FROM users WHERE txt = 'cost: $1, name: :name' AND id = 5");
    }

    #[test]
    fn test_mixed_params_error() {
        let sql = "SELECT * FROM users";
        let positional = vec![ScalarValue::Int32(1)];
        let named = vec![NamedParam {
            name: "a".into(),
            value: ScalarValue::Int32(2),
        }];
        let result = apply_params(sql, &positional, &named);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_params_named_full() {
        let sql = "SELECT * FROM t WHERE x = :x AND y = :y";
        let named = vec![
            NamedParam {
                name: "x".into(),
                value: ScalarValue::Int32(10),
            },
            NamedParam {
                name: "y".into(),
                value: ScalarValue::Utf8("hello".into()),
            },
        ];
        let result = apply_params(sql, &[], &named).unwrap();
        assert_eq!(result, "SELECT * FROM t WHERE x = 10 AND y = 'hello'");
    }

    #[test]
    fn test_apply_params_positional_full() {
        let sql = "INSERT INTO t VALUES ($1, $2, $3)";
        let params = vec![
            ScalarValue::Int32(1),
            ScalarValue::Utf8("test".into()),
            ScalarValue::Float64(3.1),
        ];
        let result = apply_params(sql, &params, &[]).unwrap();
        assert_eq!(result, "INSERT INTO t VALUES (1, 'test', 3.1)");
    }

    #[test]
    fn test_into_param_trait() {
        assert!(matches!(42i32.into_scalar(), ScalarValue::Int32(42)));
        assert!(matches!(100i64.into_scalar(), ScalarValue::Int64(100)));
        assert!(matches!(3.1f64.into_scalar(), ScalarValue::Float64(_)));
        assert!(matches!("hello".into_scalar(), ScalarValue::Utf8(_)));
        assert!(matches!(true.into_scalar(), ScalarValue::Boolean(true)));
        assert!(matches!(
            Option::<i32>::None.into_scalar(),
            ScalarValue::Null
        ));
        assert!(matches!(Some(10i32).into_scalar(), ScalarValue::Int32(10)));
    }
}
