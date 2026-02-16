//! SELECT statement planning - includes JOIN, Aggregate, and projection logic

use crate::error::{DbxError, DbxResult};
use crate::sql::planner::types::*;
use crate::storage::columnar::ScalarValue;
use sqlparser::ast::{
    Expr as SqlExpr, GroupByExpr, JoinConstraint, JoinOperator,
    OrderByExpr as SqlOrderByExpr, Query, Select, SelectItem, SetExpr, TableFactor,
    TableWithJoins,
};

use super::helpers::{convert_binary_op, match_scalar_function};
use super::LogicalPlanner;

impl LogicalPlanner {
    /// Query → LogicalPlan 변환
    pub(super) fn plan_query(&self, query: &Query) -> DbxResult<LogicalPlan> {
        let mut plan = match query.body.as_ref() {
            SetExpr::Select(select) => self.plan_select(select)?,
            _ => {
                return Err(DbxError::SqlNotSupported {
                    feature: "Non-SELECT query body".to_string(),
                    hint: "Only SELECT queries are supported".to_string(),
                });
            }
        };

        // ORDER BY
        if let Some(order_by) = &query.order_by {
            let sort_exprs: Vec<SortExpr> = order_by
                .exprs
                .iter()
                .map(|ob| self.plan_order_by_expr(ob))
                .collect::<DbxResult<Vec<_>>>()?;
            plan = LogicalPlan::Sort {
                input: Box::new(plan),
                order_by: sort_exprs,
            };
        }

        // LIMIT
        if let Some(limit_expr) = &query.limit {
            let limit = super::helpers::extract_usize(limit_expr)?;
            plan = LogicalPlan::Limit {
                input: Box::new(plan),
                count: limit,
                offset: 0,
            };
        }

        Ok(plan)
    }

    /// SELECT → LogicalPlan 변환
    pub(super) fn plan_select(&self, select: &Select) -> DbxResult<LogicalPlan> {
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
    pub(super) fn plan_order_by_expr(&self, ob: &SqlOrderByExpr) -> DbxResult<SortExpr> {
        let expr = self.plan_expr(&ob.expr)?;
        Ok(SortExpr {
            expr,
            asc: ob.asc.unwrap_or(true),
            nulls_first: ob.nulls_first.unwrap_or(true),
        })
    }

    /// Extract aggregate function calls from SELECT items.
    pub(super) fn extract_aggregates(&self, projection: &[SelectItem]) -> DbxResult<Vec<AggregateExpr>> {
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
    pub(super) fn try_extract_aggregate(
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
    pub(super) fn plan_function_arg(&self, args: &sqlparser::ast::FunctionArguments) -> DbxResult<Expr> {
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
    pub(super) fn plan_from(&self, from: &[TableWithJoins]) -> DbxResult<LogicalPlan> {
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
    pub(super) fn plan_projection(&self, projection: &[SelectItem]) -> DbxResult<Vec<(Expr, Option<String>)>> {
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
    pub(super) fn plan_expr(&self, expr: &SqlExpr) -> DbxResult<Expr> {
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
