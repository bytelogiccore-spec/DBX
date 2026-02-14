//! SQL 플래너 타입 정의
//!
//! LogicalPlan, PhysicalPlan, Expr 등의 핵심 타입들을 정의합니다.

use crate::storage::columnar::ScalarValue;

/// 논리 플랜 — SQL 쿼리의 논리적 표현
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalPlan {
    /// 테이블 스캔
    Scan {
        table: String,
        columns: Vec<String>,
        filter: Option<Expr>,
    },
    /// 컬럼 선택/계산
    Project {
        input: Box<LogicalPlan>,
        projections: Vec<(Expr, Option<String>)>,
    },
    /// WHERE 조건 필터
    Filter {
        input: Box<LogicalPlan>,
        predicate: Expr,
    },
    /// GROUP BY + 집계
    Aggregate {
        input: Box<LogicalPlan>,
        group_by: Vec<Expr>,
        aggregates: Vec<AggregateExpr>,
    },
    /// JOIN
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        join_type: JoinType,
        on: Expr,
    },
    /// ORDER BY
    Sort {
        input: Box<LogicalPlan>,
        order_by: Vec<SortExpr>,
    },
    /// LIMIT/OFFSET
    Limit {
        input: Box<LogicalPlan>,
        count: usize,
        offset: usize,
    },
    /// INSERT INTO
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<Expr>>,
    },
    /// UPDATE
    Update {
        table: String,
        assignments: Vec<(String, Expr)>,
        filter: Option<Expr>,
    },
    /// DELETE
    Delete { table: String, filter: Option<Expr> },
    /// DROP TABLE
    DropTable { table: String, if_exists: bool },
    /// CREATE TABLE
    CreateTable {
        table: String,
        columns: Vec<(String, String)>, // (name, type_str)
        if_not_exists: bool,
    },
    /// CREATE INDEX
    CreateIndex {
        table: String,
        index_name: String,
        columns: Vec<String>,
        if_not_exists: bool,
    },
    /// DROP INDEX
    DropIndex {
        table: String,
        index_name: String,
        if_exists: bool,
    },
    /// ALTER TABLE
    AlterTable {
        table: String,
        operation: AlterTableOperation,
    },
}

/// ALTER TABLE operations
#[derive(Debug, Clone, PartialEq)]
pub enum AlterTableOperation {
    /// ADD COLUMN
    AddColumn {
        column_name: String,
        data_type: String,
    },
    /// DROP COLUMN (future)
    DropColumn { column_name: String },
    /// RENAME COLUMN (future)
    RenameColumn { old_name: String, new_name: String },
}

/// 표현식 — 컬럼, 리터럴, 연산자, 함수
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// 컬럼 참조
    Column(String),
    /// 리터럴 값
    Literal(ScalarValue),
    /// 이항 연산 (+, -, *, /, =, !=, <, >, AND, OR)
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    /// 함수 호출 (COUNT, SUM, AVG, MIN, MAX)
    Function { name: String, args: Vec<Expr> },
    /// 스칼라 함수 호출 (UPPER, LOWER, ABS, NOW 등)
    ScalarFunc {
        func: ScalarFunction,
        args: Vec<Expr>,
    },
    /// IS NULL
    IsNull(Box<Expr>),
    /// IS NOT NULL
    IsNotNull(Box<Expr>),
    /// IN (...)
    InList {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
}

/// 이항 연산자
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // 산술
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    // 비교
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    // 논리
    And,
    Or,
}

/// 집계 표현식
#[derive(Debug, Clone, PartialEq)]
pub struct AggregateExpr {
    pub function: AggregateFunction,
    pub expr: Expr,
    pub alias: Option<String>,
}

/// 집계 함수
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

/// 스칼라 함수 (행별 처리)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarFunction {
    // 문자열 함수
    Upper,
    Lower,
    Length,
    Substring,
    Concat,
    Trim,

    // 수학 함수
    Abs,
    Round,
    Ceil,
    Floor,
    Sqrt,
    Power,

    // 날짜/시간 함수
    Now,
    CurrentDate,
    CurrentTime,
    Year,
    Month,
    Day,
    Hour,
    Minute,
    Second,
}

/// JOIN 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Cross,
}

/// 정렬 표현식
#[derive(Debug, Clone, PartialEq)]
pub struct SortExpr {
    pub expr: Expr,
    pub asc: bool,
    pub nulls_first: bool,
}

// ===== Physical Plan =====

/// 물리 플랜 — 실행 가능한 쿼리 플랜
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicalPlan {
    /// 테이블 스캔
    TableScan {
        table: String,
        projection: Vec<usize>,
        filter: Option<PhysicalExpr>,
    },
    /// Hash Join
    HashJoin {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        on: Vec<(usize, usize)>,
        join_type: JoinType,
    },
    /// Hash Aggregate
    HashAggregate {
        input: Box<PhysicalPlan>,
        group_by: Vec<usize>,
        aggregates: Vec<PhysicalAggExpr>,
    },
    /// Sort Merge
    SortMerge {
        input: Box<PhysicalPlan>,
        order_by: Vec<(usize, bool)>,
    },
    /// Projection
    Projection {
        input: Box<PhysicalPlan>,
        exprs: Vec<PhysicalExpr>,
        aliases: Vec<Option<String>>,
    },
    /// Limit
    Limit {
        input: Box<PhysicalPlan>,
        count: usize,
        offset: usize,
    },
    /// Insert
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<PhysicalExpr>>,
    },
    /// Update
    Update {
        table: String,
        assignments: Vec<(String, PhysicalExpr)>,
        filter: Option<PhysicalExpr>,
    },
    /// Delete
    Delete {
        table: String,
        filter: Option<PhysicalExpr>,
    },
    /// Drop Table
    DropTable { table: String, if_exists: bool },
    /// Create Table
    CreateTable {
        table: String,
        columns: Vec<(String, String)>,
        if_not_exists: bool,
    },
    /// Create Index
    CreateIndex {
        table: String,
        index_name: String,
        columns: Vec<String>,
        if_not_exists: bool,
    },
    /// Drop Index
    DropIndex {
        table: String,
        index_name: String,
        if_exists: bool,
    },
    /// Alter Table
    AlterTable {
        table: String,
        operation: crate::sql::planner::types::AlterTableOperation,
    },
}

impl PhysicalPlan {
    /// Returns true if the query plan is considered "analytical" (OLAP).
    pub fn is_analytical(&self) -> bool {
        match self {
            PhysicalPlan::HashJoin { .. }
            | PhysicalPlan::HashAggregate { .. }
            | PhysicalPlan::SortMerge { .. } => true,
            PhysicalPlan::TableScan { filter, .. } => filter.is_some(),
            PhysicalPlan::Projection { input, .. } | PhysicalPlan::Limit { input, .. } => {
                input.is_analytical()
            }
            PhysicalPlan::Insert { .. } => false, // INSERT is not analytical
            PhysicalPlan::Update { .. } => false, // UPDATE is not analytical
            PhysicalPlan::Delete { .. } => false, // DELETE is not analytical
            PhysicalPlan::DropTable { .. } => false, // DROP TABLE is not analytical
            PhysicalPlan::CreateTable { .. } => false, // CREATE TABLE is not analytical
            PhysicalPlan::CreateIndex { .. } => false, // CREATE INDEX is not analytical
            PhysicalPlan::DropIndex { .. } => false, // DROP INDEX is not analytical
            PhysicalPlan::AlterTable { .. } => false, // ALTER TABLE is not analytical
        }
    }

    /// Returns a list of all tables involved in this plan.
    pub fn tables(&self) -> Vec<String> {
        match self {
            PhysicalPlan::TableScan { table, .. } => vec![table.clone()],
            PhysicalPlan::HashJoin { left, right, .. } => {
                let mut v = left.tables();
                v.extend(right.tables());
                v
            }
            PhysicalPlan::HashAggregate { input, .. }
            | PhysicalPlan::SortMerge { input, .. }
            | PhysicalPlan::Projection { input, .. }
            | PhysicalPlan::Limit { input, .. } => input.tables(),
            PhysicalPlan::Insert { table, .. } => vec![table.clone()],
            PhysicalPlan::Update { table, .. } => vec![table.clone()],
            PhysicalPlan::Delete { table, .. } => vec![table.clone()],
            PhysicalPlan::DropTable { table, .. } => vec![table.clone()],
            PhysicalPlan::CreateTable { table, .. } => vec![table.clone()],
            PhysicalPlan::CreateIndex { table, .. } => vec![table.clone()],
            PhysicalPlan::DropIndex { table, .. } => vec![table.clone()],
            PhysicalPlan::AlterTable { table, .. } => vec![table.clone()],
        }
    }
}

/// 물리 표현식
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicalExpr {
    Column(usize),
    Literal(ScalarValue),
    BinaryOp {
        left: Box<PhysicalExpr>,
        op: BinaryOperator,
        right: Box<PhysicalExpr>,
    },
    IsNull(Box<PhysicalExpr>),
    IsNotNull(Box<PhysicalExpr>),
    /// 스칼라 함수
    ScalarFunc {
        func: ScalarFunction,
        args: Vec<PhysicalExpr>,
    },
}

impl PhysicalExpr {
    pub fn get_type(&self, input_schema: &arrow::datatypes::Schema) -> arrow::datatypes::DataType {
        use arrow::datatypes::DataType;
        match self {
            Self::Column(idx) => input_schema.field(*idx).data_type().clone(),
            Self::Literal(scalar) => scalar.data_type(),
            Self::BinaryOp { left, op, .. } => match op {
                BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq
                | BinaryOperator::And
                | BinaryOperator::Or => DataType::Boolean,
                _ => left.get_type(input_schema),
            },
            Self::IsNull(_) | Self::IsNotNull(_) => DataType::Boolean,
            Self::ScalarFunc { func, .. } => match func {
                ScalarFunction::Length => DataType::Int32,
                ScalarFunction::Abs
                | ScalarFunction::Round
                | ScalarFunction::Ceil
                | ScalarFunction::Floor
                | ScalarFunction::Sqrt
                | ScalarFunction::Power => DataType::Float64,
                ScalarFunction::Now | ScalarFunction::CurrentDate | ScalarFunction::CurrentTime => {
                    DataType::Int64
                }
                ScalarFunction::Year
                | ScalarFunction::Month
                | ScalarFunction::Day
                | ScalarFunction::Hour
                | ScalarFunction::Minute
                | ScalarFunction::Second => DataType::Int32,
                _ => DataType::Utf8, // String functions
            },
        }
    }
}

/// 물리 집계 표현식
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalAggExpr {
    pub function: AggregateFunction,
    pub input: usize,
    pub alias: Option<String>,
}
