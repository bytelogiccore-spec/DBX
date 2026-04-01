//! Rule: Subquery Unnesting
//!
//! 서브쿼리를 Join 연산으로 평탄화(Unnest)하여 실행 성능을 높이는 규칙.
//! 최우선적으로 `col IN (SELECT ...)` 패턴을 파악하여 Semi-Join 또는 Inner-Join 형태로 풀어냅니다.

use crate::error::DbxResult;
use crate::sql::planner::LogicalPlan;
use super::OptimizationRule;

pub struct SubqueryUnnestingRule;

impl OptimizationRule for SubqueryUnnestingRule {
    fn name(&self) -> &str {
        "SubqueryUnnesting"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.unnest(plan)
    }
}

impl SubqueryUnnestingRule {
    fn unnest(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Filter { input, predicate } => {
                let optimized_input = self.unnest(*input)?;
                
                // IN 서브쿼리 패턴 감지 (현재 MVP Expr 트리에서는 IN 리스트만 존재, 
                // 차후 Expr::InSubquery가 추가되면 여기에 매칭)
                // 만약 Expr::InSubquery(col_expr, subquery_plan) 이라면
                // LogicalPlan::Join (LeftSemi) 로 변환하는 로직 수행.
                
                Ok(LogicalPlan::Filter {
                    input: Box::new(optimized_input),
                    predicate,
                })
            }
            LogicalPlan::Project { input, projections } => Ok(LogicalPlan::Project {
                input: Box::new(self.unnest(*input)?),
                projections,
            }),
            LogicalPlan::Aggregate {
                input,
                group_by,
                aggregates,
                mode,
            } => Ok(LogicalPlan::Aggregate {
                input: Box::new(self.unnest(*input)?),
                group_by,
                aggregates,
                mode,
            }),
            LogicalPlan::Sort { input, order_by } => Ok(LogicalPlan::Sort {
                input: Box::new(self.unnest(*input)?),
                order_by,
            }),
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => Ok(LogicalPlan::Limit {
                input: Box::new(self.unnest(*input)?),
                count,
                offset,
            }),
            LogicalPlan::Join {
                left,
                right,
                join_type,
                on,
            } => Ok(LogicalPlan::Join {
                left: Box::new(self.unnest(*left)?),
                right: Box::new(self.unnest(*right)?),
                join_type,
                on,
            }),
            other => Ok(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Unit tests will simulate Expr::InSubquery unwrapping when it's added to types.rs
}
