// Query Builder - Fluent API for building SQL queries
//
// This module provides a type-safe, fluent API for building SQL queries
// without writing raw SQL strings.

use crate::{Database, DbxResult};
use arrow::record_batch::RecordBatch;

/// Connector type for WHERE clauses
#[derive(Debug, Clone, PartialEq)]
enum Connector {
    And,
    Or,
}

impl Connector {
    fn to_sql(&self) -> &str {
        match self {
            Connector::And => "AND",
            Connector::Or => "OR",
        }
    }
}

/// JOIN type
#[derive(Debug, Clone, PartialEq)]
enum JoinType {
    Inner,
    Left,
    Right,
}

impl JoinType {
    fn to_sql(&self) -> &str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
        }
    }
}

/// JOIN clause representation
#[derive(Debug, Clone)]
struct JoinClause {
    join_type: JoinType,
    table: String,
    on_conditions: Vec<(String, String)>, // (left_column, right_column)
}

/// WHERE clause representation
#[derive(Debug, Clone)]
struct WhereClause {
    column: String,
    operator: String,
    value: String,
    connector: Connector,
}

/// ORDER BY clause representation
#[derive(Debug, Clone)]
struct OrderByClause {
    column: String,
    direction: String,
}

/// Aggregate function types
#[derive(Debug, Clone)]
enum AggregateFunction {
    Count(String),
    Sum(String),
    Avg(String),
    Min(String),
    Max(String),
}

impl AggregateFunction {
    fn to_sql(&self) -> String {
        match self {
            AggregateFunction::Count(col) => format!("COUNT({})", col),
            AggregateFunction::Sum(col) => format!("SUM({})", col),
            AggregateFunction::Avg(col) => format!("AVG({})", col),
            AggregateFunction::Min(col) => format!("MIN({})", col),
            AggregateFunction::Max(col) => format!("MAX({})", col),
        }
    }
}

/// Query Builder for constructing SQL queries using a fluent API
pub struct QueryBuilder<'a> {
    db: &'a Database,
    select_columns: Vec<String>,
    from_table: Option<String>,
    join_clauses: Vec<JoinClause>,
    where_clauses: Vec<WhereClause>,
    order_by_clauses: Vec<OrderByClause>,
    limit_value: Option<usize>,
    offset_value: Option<usize>,
    aggregate: Option<AggregateFunction>,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new QueryBuilder
    pub(crate) fn new(db: &'a Database) -> Self {
        Self {
            db,
            select_columns: Vec::new(),
            from_table: None,
            join_clauses: Vec::new(),
            where_clauses: Vec::new(),
            order_by_clauses: Vec::new(),
            limit_value: None,
            offset_value: None,
            aggregate: None,
        }
    }

    /// Select specific columns
    ///
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select_columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Specify the table to query from
    ///
    pub fn from(mut self, table: &str) -> Self {
        self.from_table = Some(table.to_string());
        self
    }

    /// Add a WHERE clause
    ///
    pub fn where_(mut self, column: &str, operator: &str, value: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            connector: Connector::And, // First clause uses AND (will be ignored)
        });
        self
    }

    /// Add an AND condition to the WHERE clause
    ///
    pub fn and(mut self, column: &str, operator: &str, value: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            connector: Connector::And,
        });
        self
    }

    /// Add an OR condition to the WHERE clause
    ///
    pub fn or(mut self, column: &str, operator: &str, value: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.to_string(),
            connector: Connector::Or,
        });
        self
    }

    /// Add an ORDER BY clause
    ///
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by_clauses.push(OrderByClause {
            column: column.to_string(),
            direction: direction.to_uppercase(),
        });
        self
    }

    /// Set the LIMIT clause
    ///
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Set the OFFSET clause
    ///
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Add an INNER JOIN clause
    ///
    pub fn inner_join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.join_clauses.push(JoinClause {
            join_type: JoinType::Inner,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }

    /// Add a LEFT JOIN clause
    ///
    pub fn left_join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.join_clauses.push(JoinClause {
            join_type: JoinType::Left,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }

    /// Add a RIGHT JOIN clause
    ///
    pub fn right_join(mut self, table: &str, left_col: &str, right_col: &str) -> Self {
        self.join_clauses.push(JoinClause {
            join_type: JoinType::Right,
            table: table.to_string(),
            on_conditions: vec![(left_col.to_string(), right_col.to_string())],
        });
        self
    }

    /// Count rows
    ///
    pub fn count(mut self, column: &str) -> Self {
        self.aggregate = Some(AggregateFunction::Count(column.to_string()));
        self
    }

    /// Sum values
    pub fn sum(mut self, column: &str) -> Self {
        self.aggregate = Some(AggregateFunction::Sum(column.to_string()));
        self
    }

    /// Average values
    pub fn avg(mut self, column: &str) -> Self {
        self.aggregate = Some(AggregateFunction::Avg(column.to_string()));
        self
    }

    /// Minimum value
    pub fn min(mut self, column: &str) -> Self {
        self.aggregate = Some(AggregateFunction::Min(column.to_string()));
        self
    }

    /// Maximum value
    pub fn max(mut self, column: &str) -> Self {
        self.aggregate = Some(AggregateFunction::Max(column.to_string()));
        self
    }

    /// Build the SQL query string
    fn build_sql(&self) -> String {
        let mut sql = String::new();

        // SELECT clause
        if let Some(agg) = &self.aggregate {
            sql.push_str(&format!("SELECT {}", agg.to_sql()));
        } else {
            let columns = if self.select_columns.is_empty() {
                "*".to_string()
            } else {
                self.select_columns.join(", ")
            };
            sql.push_str(&format!("SELECT {}", columns));
        }

        // FROM clause
        if let Some(table) = &self.from_table {
            sql.push_str(&format!(" FROM {}", table));
        }

        // JOIN clauses
        for join in &self.join_clauses {
            sql.push_str(&format!(" {} {}", join.join_type.to_sql(), join.table));

            if !join.on_conditions.is_empty() {
                sql.push_str(" ON ");
                let conditions: Vec<String> = join
                    .on_conditions
                    .iter()
                    .map(|(left, right)| format!("{} = {}", left, right))
                    .collect();
                sql.push_str(&conditions.join(" AND "));
            }
        }

        // WHERE clause
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (i, clause) in self.where_clauses.iter().enumerate() {
                if i > 0 {
                    sql.push_str(&format!(" {} ", clause.connector.to_sql()));
                }
                sql.push_str(&format!(
                    "{} {} {}",
                    clause.column, clause.operator, clause.value
                ));
            }
        }

        // ORDER BY clause
        if !self.order_by_clauses.is_empty() {
            sql.push_str(" ORDER BY ");
            let orders: Vec<String> = self
                .order_by_clauses
                .iter()
                .map(|o| format!("{} {}", o.column, o.direction))
                .collect();
            sql.push_str(&orders.join(", "));
        }

        // LIMIT clause
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // OFFSET clause
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    /// Execute the query and return results
    ///
    pub fn execute(self) -> DbxResult<Vec<RecordBatch>> {
        let sql = self.build_sql();
        self.db.execute_sql(&sql)
    }
}
