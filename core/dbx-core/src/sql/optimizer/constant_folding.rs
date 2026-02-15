//! Rule 3: Constant Folding
//!
//! 상수 표현식을 컴파일 타임에 평가 (1 + 2 → 3)

use crate::error::DbxResult;
use crate::sql::planner::{BinaryOperator, Expr, LogicalPlan};
use crate::storage::columnar::ScalarValue;

use super::OptimizationRule;

/// 상수 표현식을 컴파일 타임에 평가 (1 + 2 → 3)
pub struct ConstantFoldingRule;

impl OptimizationRule for ConstantFoldingRule {
    fn name(&self) -> &str {
        "ConstantFolding"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.fold(plan)
    }
}

impl ConstantFoldingRule {
    fn fold(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Filter { input, predicate } => {
                let folded_pred = self.fold_expr(predicate);
                // If predicate folded to TRUE, eliminate filter entirely
                if let Expr::Literal(ScalarValue::Boolean(true)) = &folded_pred {
                    return self.fold(*input);
                }
                Ok(LogicalPlan::Filter {
                    input: Box::new(self.fold(*input)?),
                    predicate: folded_pred,
                })
            }
            LogicalPlan::Project {
                input,
                projections: columns,
            } => {
                let folded_cols = columns
                    .into_iter()
                    .map(|(c, a)| (self.fold_expr(c), a))
                    .collect();
                Ok(LogicalPlan::Project {
                    input: Box::new(self.fold(*input)?),
                    projections: folded_cols,
                })
            }
            LogicalPlan::Sort { input, order_by } => Ok(LogicalPlan::Sort {
                input: Box::new(self.fold(*input)?),
                order_by,
            }),
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => Ok(LogicalPlan::Limit {
                input: Box::new(self.fold(*input)?),
                count,
                offset,
            }),
            LogicalPlan::Aggregate {
                input,
                group_by,
                aggregates,
            } => Ok(LogicalPlan::Aggregate {
                input: Box::new(self.fold(*input)?),
                group_by,
                aggregates,
            }),
            LogicalPlan::Scan {
                table,
                columns,
                filter,
            } => Ok(LogicalPlan::Scan {
                table,
                columns,
                filter: filter.map(|f| self.fold_expr(f)),
            }),
            other => Ok(other),
        }
    }

    /// Fold constant expressions: Literal op Literal → Literal
    fn fold_expr(&self, expr: Expr) -> Expr {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                let left = self.fold_expr(*left);
                let right = self.fold_expr(*right);

                // Both sides are literals → evaluate at plan time
                if let (Expr::Literal(lv), Expr::Literal(rv)) = (&left, &right)
                    && let Some(result) = self.eval_const(lv, &op, rv)
                {
                    return Expr::Literal(result);
                }

                Expr::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                }
            }
            other => other,
        }
    }

    /// Evaluate constant binary operations.
    fn eval_const(
        &self,
        left: &ScalarValue,
        op: &BinaryOperator,
        right: &ScalarValue,
    ) -> Option<ScalarValue> {
        match (left, op, right) {
            // Integer arithmetic
            (ScalarValue::Int32(a), BinaryOperator::Plus, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Int32(a + b))
            }
            (ScalarValue::Int32(a), BinaryOperator::Minus, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Int32(a - b))
            }
            (ScalarValue::Int32(a), BinaryOperator::Multiply, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Int32(a * b))
            }
            (ScalarValue::Int32(a), BinaryOperator::Divide, ScalarValue::Int32(b)) if *b != 0 => {
                Some(ScalarValue::Int32(a / b))
            }
            // Integer comparison
            (ScalarValue::Int32(a), BinaryOperator::Eq, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Boolean(a == b))
            }
            (ScalarValue::Int32(a), BinaryOperator::NotEq, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Boolean(a != b))
            }
            (ScalarValue::Int32(a), BinaryOperator::Lt, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Boolean(a < b))
            }
            (ScalarValue::Int32(a), BinaryOperator::Gt, ScalarValue::Int32(b)) => {
                Some(ScalarValue::Boolean(a > b))
            }
            // Boolean logic
            (ScalarValue::Boolean(a), BinaryOperator::And, ScalarValue::Boolean(b)) => {
                Some(ScalarValue::Boolean(*a && *b))
            }
            (ScalarValue::Boolean(a), BinaryOperator::Or, ScalarValue::Boolean(b)) => {
                Some(ScalarValue::Boolean(*a || *b))
            }
            // Float arithmetic
            (ScalarValue::Float64(a), BinaryOperator::Plus, ScalarValue::Float64(b)) => {
                Some(ScalarValue::Float64(a + b))
            }
            (ScalarValue::Float64(a), BinaryOperator::Minus, ScalarValue::Float64(b)) => {
                Some(ScalarValue::Float64(a - b))
            }
            (ScalarValue::Float64(a), BinaryOperator::Multiply, ScalarValue::Float64(b)) => {
                Some(ScalarValue::Float64(a * b))
            }
            _ => None,
        }
    }
}
