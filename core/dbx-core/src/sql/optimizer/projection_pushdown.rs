//! Rule 2: Projection Pushdown
//!
//! 불필요한 컬럼을 조기 제거하여 메모리 절감

use crate::error::DbxResult;
use crate::sql::planner::{Expr, LogicalPlan};

use super::OptimizationRule;

/// 불필요한 컬럼을 조기 제거하여 메모리 절감
pub struct ProjectionPushdownRule;

impl OptimizationRule for ProjectionPushdownRule {
    fn name(&self) -> &str {
        "ProjectionPushdown"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.push_down(plan)
    }
}

impl ProjectionPushdownRule {
    fn push_down(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Project {
                input,
                projections: columns,
            } => {
                let optimized_input = self.push_down(*input)?;
                match optimized_input {
                    LogicalPlan::Scan {
                        table,
                        columns: scan_cols,
                        filter,
                    } if !columns.is_empty() => {
                        let needed = self.extract_column_names(&columns);
                        let final_cols = if scan_cols.is_empty() {
                            needed
                        } else {
                            scan_cols
                                .into_iter()
                                .filter(|c| needed.contains(c))
                                .collect()
                        };
                        Ok(LogicalPlan::Project {
                            input: Box::new(LogicalPlan::Scan {
                                table,
                                columns: final_cols,
                                filter,
                            }),
                            projections: columns,
                        })
                    }
                    other => Ok(LogicalPlan::Project {
                        input: Box::new(other),
                        projections: columns,
                    }),
                }
            }
            LogicalPlan::Filter { input, predicate } => Ok(LogicalPlan::Filter {
                input: Box::new(self.push_down(*input)?),
                predicate,
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

    /// Extract column names referenced in expressions.
    fn extract_column_names(&self, projections: &[(Expr, Option<String>)]) -> Vec<String> {
        let mut names = Vec::new();
        for (expr, _) in projections {
            self.collect_columns(expr, &mut names);
        }
        names.sort();
        names.dedup();
        names
    }

    fn collect_columns(&self, expr: &Expr, out: &mut Vec<String>) {
        match expr {
            Expr::Column(name) => out.push(name.clone()),
            Expr::BinaryOp { left, right, .. } => {
                self.collect_columns(left, out);
                self.collect_columns(right, out);
            }
            Expr::Function { args, .. } => {
                for arg in args {
                    self.collect_columns(arg, out);
                }
            }
            Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
                self.collect_columns(inner, out);
            }
            Expr::InList { expr, list, .. } => {
                self.collect_columns(expr, out);
                for item in list {
                    self.collect_columns(item, out);
                }
            }
            _ => {}
        }
    }
}
