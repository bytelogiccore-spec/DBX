//! SQL 논리 플래너 유틸리티 함수
//!
//! LogicalPlanner를 위한 헬퍼 함수들

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use crate::storage::columnar::ScalarValue;
use sqlparser::ast::{
    BinaryOperator as SqlBinaryOp, Expr as SqlExpr, GroupByExpr, JoinConstraint, JoinOperator,
    OrderByExpr as SqlOrderByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
    TableWithJoins,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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

/// 논리 플랜 빌더 — AST → LogicalPlan 변환
pub struct LogicalPlanner {
    alias_map: Arc<RwLock<HashMap<String, Expr>>>,
}

impl LogicalPlanner {
    pub fn new() -> Self {
        Self {
            alias_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// SQL Statement → LogicalPlan 변환
    pub fn plan(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        match statement {
            Statement::Query(query) => self.plan_query(query),
            Statement::Insert { .. } => {
                // Extract Insert struct from Statement
                if let Statement::Insert(insert) = statement {
                    self.plan_insert(insert)
                } else {
                    unreachable!()
                }
            }
            Statement::Update { .. } => {
                self.plan_update(statement)
            }
            Statement::Delete { .. } => {
                self.plan_delete(statement)
            }
            Statement::Drop { names, object_type, if_exists, .. } => {
                // DROP TABLE or DROP INDEX parsing
                use sqlparser::ast::ObjectType;
                match object_type {
                    ObjectType::Table => {
                        let table = names[0].to_string();
                        Ok(LogicalPlan::DropTable {
                            table,
                            if_exists: *if_exists,
                        })
                    }
                    ObjectType::Index => {
                        // DROP INDEX
                        if names.is_empty() {
                            return Err(DbxError::Schema("DROP INDEX requires an index name".to_string()));
                        }
                        
                        // Parse index name (may include table name)
                        let index_full_name = names[0].to_string();
                        
                        // Try to split table.index_name format
                        let (table, index_name) = if index_full_name.contains('.') {
                            let parts: Vec<&str> = index_full_name.splitn(2, '.').collect();
                            (parts[0].to_string(), parts[1].to_string())
                        } else {
                            // If no table specified, we'll need to find it later
                            // For now, use empty string as placeholder
                            ("".to_string(), index_full_name)
                        };
                        
                        Ok(LogicalPlan::DropIndex {
                            table,
                            index_name,
                            if_exists: *if_exists,
                        })
                    }
                    _ => {
                        Err(DbxError::SqlNotSupported {
                            feature: format!("DROP {:?}", object_type),
                            hint: "Only DROP TABLE and DROP INDEX are currently supported".to_string(),
                        })
                    }
                }
            }
            Statement::CreateTable(create_table) => {
                // CREATE TABLE parsing
                let table = create_table.name.to_string();
                let columns: Vec<(String, String)> = create_table.columns.iter().map(|col| {
                    let name = col.name.to_string();
                    let type_str = col.data_type.to_string();
                    (name, type_str)
                }).collect();
                
                Ok(LogicalPlan::CreateTable {
                    table,
                    columns,
                    if_not_exists: create_table.if_not_exists,
                })
            }
            Statement::AlterTable { name, operations, .. } => {
                // ALTER TABLE parsing (simplified - only ADD COLUMN for now)
                let table = name.to_string();
                
                // Get the first operation
                let operation = operations.get(0).ok_or_else(|| {
                    DbxError::Schema("ALTER TABLE requires at least one operation".to_string())
                })?;
                
                use sqlparser::ast::AlterTableOperation as SqlAlterOp;
                let alter_op = match operation {
                    SqlAlterOp::AddColumn { column_def, .. } => {
                        let column_name = column_def.name.to_string();
                        let data_type = column_def.data_type.to_string();
                        crate::sql::planner::types::AlterTableOperation::AddColumn {
                            column_name,
                            data_type,
                        }
                    }
                    SqlAlterOp::DropColumn { column_name, .. } => {
                        let col_name = column_name.to_string();
                        crate::sql::planner::types::AlterTableOperation::DropColumn {
                            column_name: col_name,
                        }
                    }
                    SqlAlterOp::RenameColumn { old_column_name, new_column_name } => {
                        let old_name = old_column_name.to_string();
                        let new_name = new_column_name.to_string();
                        crate::sql::planner::types::AlterTableOperation::RenameColumn {
                            old_name,
                            new_name,
                        }
                    }
                    _ => {
                        return Err(DbxError::SqlNotSupported {
                            feature: format!("ALTER TABLE operation: {:?}", operation),
                            hint: "Only ADD COLUMN, DROP COLUMN, and RENAME COLUMN are currently supported".to_string(),
                        });
                    }
                };
                
                Ok(LogicalPlan::AlterTable {
                    table,
                    operation: alter_op,
                })
            }
            Statement::CreateIndex(create_index) => {
                // CREATE INDEX parsing
                let index_name = create_index.name.as_ref()
                    .ok_or_else(|| DbxError::Schema("CREATE INDEX requires an index name".to_string()))?
                    .to_string();
                
                // In sqlparser 0.52, table_name is ObjectName (not Option)
                let table = create_index.table_name.to_string();
                
                // Extract column names from OrderByExpr
                if create_index.columns.is_empty() {
                    return Err(DbxError::Schema("CREATE INDEX requires at least one column".to_string()));
                }
                
                let columns: Vec<String> = create_index.columns.iter()
                    .map(|order_by_expr| {
                        // Extract column name from the expression
                        match &order_by_expr.expr {
                            sqlparser::ast::Expr::Identifier(ident) => ident.value.clone(),
                            sqlparser::ast::Expr::CompoundIdentifier(idents) => {
                                idents.iter().map(|i| i.value.clone()).collect::<Vec<_>>().join(".")
                            }
                            _ => order_by_expr.expr.to_string(),
                        }
                    })
                    .collect();
                
                Ok(LogicalPlan::CreateIndex {
                    table,
                    index_name,
                    columns,
                    if_not_exists: create_index.if_not_exists,
                })
            }
            _ => Err(DbxError::SqlNotSupported {
                feature: format!("Statement type: {:?}", statement),
                hint: "Only SELECT, INSERT, UPDATE, DELETE, DROP TABLE, CREATE TABLE, ALTER TABLE, CREATE INDEX, and DROP INDEX are currently supported".to_string(),
            }),
        }
    }

    /// Query → LogicalPlan 변환
    fn plan_query(&self, query: &Query) -> DbxResult<LogicalPlan> {
        let mut plan = match query.body.as_ref() {
            SetExpr::Select(select) => self.plan_select(select)?,
            _ => {
                return Err(DbxError::SqlNotSupported {
                    feature: "Non-SELECT queries".to_string(),
                    hint: "Only SELECT queries are currently supported".to_string(),
                });
            }
        };

        // ORDER BY (lives on Query, not Select in sqlparser 0.52)
        if let Some(ref order_by) = query.order_by {
            let sort_exprs: Vec<SortExpr> = order_by
                .exprs
                .iter()
                .map(|ob| self.plan_order_by_expr(ob))
                .collect::<DbxResult<_>>()?;
            if !sort_exprs.is_empty() {
                plan = LogicalPlan::Sort {
                    input: Box::new(plan),
                    order_by: sort_exprs,
                };
            }
        }

        // LIMIT / OFFSET
        if query.limit.is_some() || query.offset.is_some() {
            let count = match &query.limit {
                Some(expr) => extract_usize(expr)?,
                None => usize::MAX,
            };
            let offset = match &query.offset {
                Some(offset) => extract_usize(&offset.value)?,
                None => 0,
            };
            plan = LogicalPlan::Limit {
                input: Box::new(plan),
                count,
                offset,
            };
        }

        Ok(plan)
    }

    /// INSERT INTO → LogicalPlan 변환
    fn plan_insert(&self, insert: &sqlparser::ast::Insert) -> DbxResult<LogicalPlan> {
        let table = insert.table_name.to_string();
        
        // Extract column names
        let column_names: Vec<String> = insert
            .columns
            .iter()
            .map(|c| c.value.clone())
            .collect();
        
        // Parse VALUES clause
        let values = if let Some(source) = &insert.source {
            match source.body.as_ref() {
                SetExpr::Values(values_set) => {
                    let mut rows = Vec::new();
                    for row in &values_set.rows {
                        let mut row_exprs = Vec::new();
                        for expr in row {
                            row_exprs.push(self.plan_expr(expr)?);
                        }
                        rows.push(row_exprs);
                    }
                    rows
                }
                _ => {
                    return Err(DbxError::SqlNotSupported {
                        feature: "INSERT with SELECT".to_string(),
                        hint: "Only INSERT INTO ... VALUES (...) is supported".to_string(),
                    });
                }
            }
        } else {
            return Err(DbxError::SqlNotSupported {
                feature: "INSERT without VALUES".to_string(),
                hint: "INSERT INTO ... VALUES (...) is required".to_string(),
            });
        };
        
        Ok(LogicalPlan::Insert {
            table,
            columns: column_names,
            values,
        })
    }

    /// UPDATE → LogicalPlan 변환
    fn plan_update(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        if let Statement::Update { table, assignments, selection, .. } = statement {
            let table_name = table.relation.to_string();
            
            // Parse SET assignments
            let mut parsed_assignments = Vec::new();
            for assignment in assignments {
                let column = assignment.target.to_string();
                let value = self.plan_expr(&assignment.value)?;
                parsed_assignments.push((column, value));
            }
            
            // Parse WHERE clause (optional)
            let filter = if let Some(sel) = selection {
                Some(self.plan_expr(sel)?)
            } else {
                None
            };
            
            Ok(LogicalPlan::Update {
                table: table_name,
                assignments: parsed_assignments,
                filter,
            })
        } else {
            Err(DbxError::SqlNotSupported {
                feature: "UPDATE statement".to_string(),
                hint: "Expected UPDATE statement".to_string(),
            })
        }
    }

    /// DELETE → LogicalPlan 변환
    fn plan_delete(&self, statement: &Statement) -> DbxResult<LogicalPlan> {
        if let Statement::Delete(delete) = statement {
            // Extract tables from FromTable enum
            let tables = match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(t) => t,
                sqlparser::ast::FromTable::WithoutKeyword(t) => t,
            };
            let table_name = tables.first()
                .map(|t| t.relation.to_string())
                .unwrap_or_default();

            // Parse WHERE clause (optional)
            let filter = if let Some(sel) = &delete.selection {
                Some(self.plan_expr(sel)?)
            } else {
                None
            };

            Ok(LogicalPlan::Delete {
                table: table_name,
                filter,
            })
        } else {
            Err(DbxError::SqlNotSupported {
                feature: "DELETE statement".to_string(),
                hint: "Expected DELETE statement".to_string(),
            })
        }
    }

    /// SELECT → LogicalPlan 변환
    fn plan_select(&self, select: &Select) -> DbxResult<LogicalPlan> {
        // Clear alias map for new query
        self.alias_map.write().unwrap().clear();

        // 0. Pre-scan projections for aliases to support WHERE/ORDER BY
        for item in &select.projection {
            if let SelectItem::ExprWithAlias { expr, alias } = item {
                let planned_expr = self.plan_expr(expr)?;
                self.alias_map
                    .write()
                    .unwrap()
                    .insert(alias.value.clone(), planned_expr);
            }
        }

        // 1. FROM 절 → Scan
        let mut plan = self.plan_from(&select.from)?;

        // 2. WHERE 절 → Filter
        if let Some(ref selection) = select.selection {
            let predicate = self.plan_expr(selection)?;
            plan = LogicalPlan::Filter {
                input: Box::new(plan),
                predicate,
            };
        }

        // 3. GROUP BY 절 → Aggregate
        let group_by_exprs = match &select.group_by {
            GroupByExpr::Expressions(exprs, _) => exprs
                .iter()
                .map(|e| self.plan_expr(e))
                .collect::<DbxResult<Vec<_>>>()?,
            GroupByExpr::All(_) => vec![], // GROUP BY ALL — treat as empty
        };

        // Extract aggregate functions from SELECT items
        let aggregates = self.extract_aggregates(&select.projection)?;

        if !group_by_exprs.is_empty() || !aggregates.is_empty() {
            plan = LogicalPlan::Aggregate {
                input: Box::new(plan),
                group_by: group_by_exprs,
                aggregates,
            };
        }

        // 4. SELECT 절 → Project
        let projections = self.plan_projection(&select.projection)?;
        if !projections.is_empty() {
            plan = LogicalPlan::Project {
                input: Box::new(plan),
                projections,
            };
        }

        Ok(plan)
    }

    /// Convert sqlparser OrderByExpr → our SortExpr
    fn plan_order_by_expr(&self, ob: &SqlOrderByExpr) -> DbxResult<SortExpr> {
        let expr = self.plan_expr(&ob.expr)?;
        Ok(SortExpr {
            expr,
            asc: ob.asc.unwrap_or(true),
            nulls_first: ob.nulls_first.unwrap_or(true),
        })
    }

    /// Extract aggregate function calls from SELECT items.
    fn extract_aggregates(&self, projection: &[SelectItem]) -> DbxResult<Vec<AggregateExpr>> {
        let mut aggregates = Vec::new();
        for item in projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    if let Some(agg) = self.try_extract_aggregate(expr, None)? {
                        aggregates.push(agg);
                    }
                }
                SelectItem::ExprWithAlias { expr, alias } => {
                    if let Some(agg) =
                        self.try_extract_aggregate(expr, Some(alias.value.clone()))?
                    {
                        aggregates.push(agg);
                    }
                }
                _ => {}
            }
        }
        Ok(aggregates)
    }

    /// Try to extract an aggregate expression from a SQL expression.
    fn try_extract_aggregate(
        &self,
        expr: &SqlExpr,
        alias: Option<String>,
    ) -> DbxResult<Option<AggregateExpr>> {
        match expr {
            SqlExpr::Function(func) => {
                let func_name = func.name.to_string().to_uppercase();
                let agg_func = match func_name.as_str() {
                    "COUNT" => Some(AggregateFunction::Count),
                    "SUM" => Some(AggregateFunction::Sum),
                    "AVG" => Some(AggregateFunction::Avg),
                    "MIN" => Some(AggregateFunction::Min),
                    "MAX" => Some(AggregateFunction::Max),
                    _ => None,
                };

                if let Some(function) = agg_func {
                    let arg_expr = match &func.args {
                        sqlparser::ast::FunctionArguments::None => {
                            // COUNT(*)
                            Expr::Literal(ScalarValue::Int32(1))
                        }
                        _ => self.plan_function_arg(&func.args)?,
                    };
                    Ok(Some(AggregateExpr {
                        function,
                        expr: arg_expr,
                        alias,
                    }))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    /// Plan function arguments (take first arg).
    fn plan_function_arg(&self, args: &sqlparser::ast::FunctionArguments) -> DbxResult<Expr> {
        match args {
            sqlparser::ast::FunctionArguments::List(arg_list) => {
                if arg_list.args.is_empty() {
                    return Ok(Expr::Literal(ScalarValue::Int32(1))); // COUNT(*)
                }
                match &arg_list.args[0] {
                    sqlparser::ast::FunctionArg::Unnamed(arg_expr) => {
                        match arg_expr {
                            sqlparser::ast::FunctionArgExpr::Expr(e) => self.plan_expr(e),
                            sqlparser::ast::FunctionArgExpr::Wildcard => {
                                Ok(Expr::Literal(ScalarValue::Int32(1))) // COUNT(*)
                            }
                            sqlparser::ast::FunctionArgExpr::QualifiedWildcard(_) => {
                                Ok(Expr::Literal(ScalarValue::Int32(1)))
                            }
                        }
                    }
                    sqlparser::ast::FunctionArg::Named { arg, .. } => match arg {
                        sqlparser::ast::FunctionArgExpr::Expr(e) => self.plan_expr(e),
                        _ => Ok(Expr::Literal(ScalarValue::Int32(1))),
                    },
                }
            }
            sqlparser::ast::FunctionArguments::None => Ok(Expr::Literal(ScalarValue::Int32(1))),
            sqlparser::ast::FunctionArguments::Subquery(_) => Err(DbxError::NotImplemented(
                "Subquery function arguments".to_string(),
            )),
        }
    }

    /// FROM 절 → Scan (with JOIN support)
    fn plan_from(&self, from: &[TableWithJoins]) -> DbxResult<LogicalPlan> {
        if from.is_empty() {
            return Err(DbxError::Schema("FROM clause is required".to_string()));
        }

        if from.len() > 1 {
            return Err(DbxError::SqlNotSupported {
                feature: "Multiple tables in FROM clause".to_string(),
                hint: "Use JOIN syntax or separate queries".to_string(),
            });
        }

        let table_with_joins = &from[0];
        let table_name = match &table_with_joins.relation {
            TableFactor::Table { name, .. } => name.to_string(),
            _ => {
                return Err(DbxError::SqlNotSupported {
                    feature: "Complex table expressions".to_string(),
                    hint: "Use simple table names only".to_string(),
                });
            }
        };

        // Start with base table scan
        let mut plan = LogicalPlan::Scan {
            table: table_name,
            columns: vec![], // All columns (optimized later by projection pushdown)
            filter: None,
        };

        // Process JOINs
        for join in &table_with_joins.joins {
            let right_table = match &join.relation {
                TableFactor::Table { name, .. } => name.to_string(),
                _ => {
                    return Err(DbxError::SqlNotSupported {
                        feature: "Complex JOIN table expressions".to_string(),
                        hint: "Use simple table names in JOIN clauses".to_string(),
                    });
                }
            };

            let right_plan = LogicalPlan::Scan {
                table: right_table,
                columns: vec![],
                filter: None,
            };

            // Determine JOIN type
            let join_type = match &join.join_operator {
                JoinOperator::Inner(_) => JoinType::Inner,
                JoinOperator::LeftOuter(_) => JoinType::Left,
                JoinOperator::RightOuter(_) => JoinType::Right,
                JoinOperator::CrossJoin => JoinType::Cross,
                _ => {
                    return Err(DbxError::SqlNotSupported {
                        feature: format!("JOIN type: {:?}", join.join_operator),
                        hint: "Supported: INNER, LEFT, RIGHT, CROSS JOIN".to_string(),
                    });
                }
            };

            // Extract JOIN condition
            let on_expr = match &join.join_operator {
                JoinOperator::Inner(constraint)
                | JoinOperator::LeftOuter(constraint)
                | JoinOperator::RightOuter(constraint) => match constraint {
                    JoinConstraint::On(expr) => self.plan_expr(expr)?,
                    JoinConstraint::Using(_) => {
                        return Err(DbxError::SqlNotSupported {
                            feature: "JOIN USING clause".to_string(),
                            hint: "Use ON clause instead (e.g., ON a.id = b.id)".to_string(),
                        });
                    }
                    JoinConstraint::Natural => {
                        return Err(DbxError::SqlNotSupported {
                            feature: "NATURAL JOIN".to_string(),
                            hint: "Use explicit ON clause instead".to_string(),
                        });
                    }
                    JoinConstraint::None => {
                        return Err(DbxError::Schema("JOIN requires ON condition".to_string()));
                    }
                },
                JoinOperator::CrossJoin => {
                    // CROSS JOIN has no condition (Cartesian product)
                    Expr::Literal(ScalarValue::Boolean(true))
                }
                _ => {
                    return Err(DbxError::SqlNotSupported {
                        feature: "Unsupported JOIN operator".to_string(),
                        hint: "Use INNER, LEFT, RIGHT, or CROSS JOIN".to_string(),
                    });
                }
            };

            plan = LogicalPlan::Join {
                left: Box::new(plan),
                right: Box::new(right_plan),
                join_type,
                on: on_expr,
            };
        }

        Ok(plan)
    }

    /// SELECT 절 → Vec<(Expr, Option<String>)>
    fn plan_projection(&self, projection: &[SelectItem]) -> DbxResult<Vec<(Expr, Option<String>)>> {
        let mut projections = Vec::new();

        for item in projection {
            match item {
                SelectItem::Wildcard(_) => {
                    // SELECT * -> empty projections means all columns
                }
                SelectItem::UnnamedExpr(expr) => {
                    projections.push((self.plan_expr(expr)?, None));
                }
                SelectItem::ExprWithAlias { expr, alias } => {
                    projections.push((self.plan_expr(expr)?, Some(alias.value.clone())));
                }
                _ => {
                    return Err(DbxError::NotImplemented(format!(
                        "Unsupported SELECT item: {:?}",
                        item
                    )));
                }
            }
        }

        Ok(projections)
    }

    /// SQL Expr → Logical Expr 변환
    fn plan_expr(&self, expr: &SqlExpr) -> DbxResult<Expr> {
        match expr {
            SqlExpr::Identifier(ident) => {
                let name = ident.value.clone();
                // Check if this identifier is an alias defined in SELECT
                if let Some(aliased_expr) = self.alias_map.read().unwrap().get(&name) {
                    return Ok(aliased_expr.clone());
                }
                Ok(Expr::Column(name))
            }
            SqlExpr::Value(value) => {
                let scalar = match value {
                    sqlparser::ast::Value::Number(n, _) => {
                        if let Ok(i) = n.parse::<i32>() {
                            ScalarValue::Int32(i)
                        } else if let Ok(i) = n.parse::<i64>() {
                            ScalarValue::Int64(i)
                        } else if let Ok(f) = n.parse::<f64>() {
                            ScalarValue::Float64(f)
                        } else {
                            return Err(DbxError::Schema(format!("Invalid number: {}", n)));
                        }
                    }
                    sqlparser::ast::Value::SingleQuotedString(s) => ScalarValue::Utf8(s.clone()),
                    sqlparser::ast::Value::Boolean(b) => ScalarValue::Boolean(*b),
                    sqlparser::ast::Value::Null => ScalarValue::Null,
                    _ => {
                        return Err(DbxError::NotImplemented(format!(
                            "Unsupported value: {:?}",
                            value
                        )));
                    }
                };
                Ok(Expr::Literal(scalar))
            }
            SqlExpr::BinaryOp { left, op, right } => {
                let left_expr = self.plan_expr(left)?;
                let right_expr = self.plan_expr(right)?;
                let binary_op = convert_binary_op(op)?;
                Ok(Expr::BinaryOp {
                    left: Box::new(left_expr),
                    op: binary_op,
                    right: Box::new(right_expr),
                })
            }
            SqlExpr::IsNull(expr) => {
                let inner = self.plan_expr(expr)?;
                Ok(Expr::IsNull(Box::new(inner)))
            }
            SqlExpr::IsNotNull(expr) => {
                let inner = self.plan_expr(expr)?;
                Ok(Expr::IsNotNull(Box::new(inner)))
            }
            SqlExpr::Function(func) => {
                let name = func.name.to_string().to_uppercase();
                let args: Vec<Expr> = match &func.args {
                    sqlparser::ast::FunctionArguments::List(arg_list) => {
                        let mut planned_args = Vec::new();
                        for arg in &arg_list.args {
                            if let sqlparser::ast::FunctionArg::Unnamed(
                                sqlparser::ast::FunctionArgExpr::Expr(e),
                            ) = arg
                            {
                                planned_args.push(self.plan_expr(e)?)
                            }
                        }
                        planned_args
                    }
                    _ => vec![],
                };

                // 스칼라 함수 매핑 시도
                if let Some(scalar_func) = match_scalar_function(&name) {
                    Ok(Expr::ScalarFunc {
                        func: scalar_func,
                        args,
                    })
                } else {
                    // 집계 함수로 처리 (실제 집계 여부는 추후 Optimizer/Planner에서 검증)
                    Ok(Expr::Function { name, args })
                }
            }
            SqlExpr::Nested(expr) => self.plan_expr(expr),
            SqlExpr::CompoundIdentifier(idents) => {
                // table.column → just use the column name
                let col_name = idents.last().map(|i| i.value.clone()).unwrap_or_default();
                Ok(Expr::Column(col_name))
            }
            _ => Err(DbxError::NotImplemented(format!(
                "Unsupported expression: {:?}",
                expr
            ))),
        }
    }
}

impl Default for LogicalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_binary_op() {
        assert_eq!(
            convert_binary_op(&SqlBinaryOp::Plus).unwrap(),
            BinaryOperator::Plus
        );
        assert_eq!(
            convert_binary_op(&SqlBinaryOp::Eq).unwrap(),
            BinaryOperator::Eq
        );
    }

    #[test]
    fn test_match_scalar_function() {
        assert_eq!(match_scalar_function("UPPER"), Some(ScalarFunction::Upper));
        assert_eq!(
            match_scalar_function("SUBSTRING"),
            Some(ScalarFunction::Substring)
        );
        assert_eq!(match_scalar_function("UNKNOWN"), None);
    }

    // LogicalPlanner integration tests
    use crate::sql::SqlParser;

    #[test]
    fn test_plan_simple_select() {
        let parser = SqlParser::new();
        let statements = parser.parse("SELECT * FROM users").unwrap();
        assert_eq!(statements.len(), 1);

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Project {
                input,
                projections: columns,
            } => {
                assert!(columns.is_empty()); // SELECT *
                match input.as_ref() {
                    LogicalPlan::Scan { table, .. } => {
                        assert_eq!(table, "users");
                    }
                    _ => panic!("Expected Scan inside Project"),
                }
            }
            LogicalPlan::Scan { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected Project or Scan, got: {:?}", plan),
        }
    }

    #[test]
    fn test_plan_select_with_where() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT id, name FROM users WHERE id = 1")
            .unwrap();
        assert_eq!(statements.len(), 1);

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Project {
                input,
                projections: columns,
            } => {
                assert_eq!(columns.len(), 2);
                match input.as_ref() {
                    LogicalPlan::Filter { input, predicate } => {
                        // predicate: id = 1
                        match predicate {
                            Expr::BinaryOp { op, .. } => {
                                assert_eq!(*op, BinaryOperator::Eq);
                            }
                            _ => panic!("Expected BinaryOp"),
                        }
                        match input.as_ref() {
                            LogicalPlan::Scan { table, .. } => {
                                assert_eq!(table, "users");
                            }
                            _ => panic!("Expected Scan"),
                        }
                    }
                    _ => panic!("Expected Filter"),
                }
            }
            _ => panic!("Expected Project"),
        }
    }

    #[test]
    fn test_plan_select_columns() {
        let parser = SqlParser::new();
        let statements = parser.parse("SELECT id, name FROM users").unwrap();
        assert_eq!(statements.len(), 1);

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Project {
                projections: columns,
                ..
            } => {
                assert_eq!(columns.len(), 2);
                match &columns[0].0 {
                    Expr::Column(name) => assert_eq!(name, "id"),
                    _ => panic!("Expected Column"),
                }
                match &columns[1].0 {
                    Expr::Column(name) => assert_eq!(name, "name"),
                    _ => panic!("Expected Column"),
                }
            }
            _ => panic!("Expected Project"),
        }
    }

    #[test]
    fn test_plan_binary_operators() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT * FROM users WHERE age > 18 AND active = true")
            .unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Project { input, .. } => match input.as_ref() {
                LogicalPlan::Filter { predicate, .. } => {
                    // predicate: age > 18 AND active = true
                    match predicate {
                        Expr::BinaryOp { op, .. } => {
                            assert!(matches!(op, BinaryOperator::And));
                        }
                        _ => panic!("Expected BinaryOp"),
                    }
                }
                _ => panic!("Expected Filter"),
            },
            LogicalPlan::Filter { predicate, .. } => match predicate {
                Expr::BinaryOp { op, .. } => {
                    assert!(matches!(op, BinaryOperator::And));
                }
                _ => panic!("Expected BinaryOp"),
            },
            _ => panic!("Expected Project or Filter"),
        }
    }

    #[test]
    fn test_plan_is_null() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT * FROM users WHERE email IS NULL")
            .unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Project { input, .. } => match input.as_ref() {
                LogicalPlan::Filter { predicate, .. } => match predicate {
                    Expr::IsNull(_) => {}
                    _ => panic!("Expected IsNull"),
                },
                _ => panic!("Expected Filter"),
            },
            LogicalPlan::Filter { predicate, .. } => match predicate {
                Expr::IsNull(_) => {}
                _ => panic!("Expected IsNull"),
            },
            _ => panic!("Expected Project or Filter"),
        }
    }

    #[test]
    fn test_extract_usize() {
        let expr = SqlExpr::Value(sqlparser::ast::Value::Number("42".to_string(), false));
        assert_eq!(extract_usize(&expr).unwrap(), 42);
    }

    #[test]
    fn test_insert_single_row() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert {
                table,
                columns,
                values,
            } => {
                assert_eq!(table, "users");
                assert_eq!(columns, vec!["id", "name"]);
                assert_eq!(values.len(), 1);
                assert_eq!(values[0].len(), 2);
            }
            _ => panic!("Expected INSERT plan"),
        }
    }

    #[test]
    fn test_insert_multiple_rows() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob')";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert { values, .. } => {
                assert_eq!(values.len(), 2);
            }
            _ => panic!("Expected INSERT plan"),
        }
    }
}
