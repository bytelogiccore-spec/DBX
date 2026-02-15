//! Rule 1: Predicate Pushdown
//!
//! Filter를 Scan에 가까이 이동하여 I/O 감소

use crate::error::DbxResult;
use crate::sql::planner::{BinaryOperator, Expr, LogicalPlan};

use super::OptimizationRule;

/// Filter를 Scan에 가까이 이동하여 I/O 감소
pub struct PredicatePushdownRule;

impl OptimizationRule for PredicatePushdownRule {
    fn name(&self) -> &str {
        "PredicatePushdown"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.push_down(plan)
    }
}

impl PredicatePushdownRule {
    fn push_down(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Filter { input, predicate } => {
                let optimized_input = self.push_down(*input)?;
                match optimized_input {
                    LogicalPlan::Project {
                        input: project_input,
                        projections: columns,
                    } if self.can_push_through_project(&predicate, &columns) => {
                        let pushed = self.push_down(LogicalPlan::Filter {
                            input: project_input,
                            predicate,
                        })?;
                        Ok(LogicalPlan::Project {
                            input: Box::new(pushed),
                            projections: columns,
                        })
                    }
                    LogicalPlan::Scan {
                        table,
                        columns,
                        filter: None,
                    } => Ok(LogicalPlan::Scan {
                        table,
                        columns,
                        filter: Some(predicate),
                    }),
                    LogicalPlan::Scan {
                        table,
                        columns,
                        filter: Some(existing),
                    } => Ok(LogicalPlan::Scan {
                        table,
                        columns,
                        filter: Some(Expr::BinaryOp {
                            left: Box::new(existing),
                            op: BinaryOperator::And,
                            right: Box::new(predicate),
                        }),
                    }),
                    other => Ok(LogicalPlan::Filter {
                        input: Box::new(other),
                        predicate,
                    }),
                }
            }
            LogicalPlan::Project { input, projections } => Ok(LogicalPlan::Project {
                input: Box::new(self.push_down(*input)?),
                projections,
            }),
            LogicalPlan::Sort { input, order_by } => Ok(LogicalPlan::Sort {
                input: Box::new(self.push_down(*input)?),
                order_by,
            }),
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => Ok(LogicalPlan::Limit {
                input: Box::new(self.push_down(*input)?),
                count,
                offset,
            }),
            LogicalPlan::Aggregate {
                input,
                group_by,
                aggregates,
            } => Ok(LogicalPlan::Aggregate {
                input: Box::new(self.push_down(*input)?),
                group_by,
                aggregates,
            }),
            other => Ok(other),
        }
    }

    /// Check if a predicate references only columns available before projection.
    fn can_push_through_project(
        &self,
        predicate: &Expr,
        _projections: &[(Expr, Option<String>)],
    ) -> bool {
        self.is_simple_column_predicate(predicate)
    }

    fn is_simple_column_predicate(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Column(_) | Expr::Literal(_) => true,
            Expr::BinaryOp { left, right, .. } => {
                self.is_simple_column_predicate(left) && self.is_simple_column_predicate(right)
            }
            Expr::IsNull(inner) | Expr::IsNotNull(inner) => self.is_simple_column_predicate(inner),
            _ => false,
        }
    }
}
