//! Rule 4: Limit Pushdown
//!
//! LIMIT를 하위 노드에 적용하여 조기 종료

use crate::error::DbxResult;
use crate::sql::planner::LogicalPlan;

use super::OptimizationRule;

/// LIMIT를 하위 노드에 적용하여 조기 종료
pub struct LimitPushdownRule;

impl OptimizationRule for LimitPushdownRule {
    fn name(&self) -> &str {
        "LimitPushdown"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.push_down(plan)
    }
}

impl LimitPushdownRule {
    fn push_down(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => {
                let optimized_input = self.push_down(*input)?;
                match optimized_input {
                    LogicalPlan::Project {
                        input: project_input,
                        projections: columns,
                    } if offset == 0 => {
                        let pushed = LogicalPlan::Limit {
                            input: project_input,
                            count,
                            offset: 0,
                        };
                        Ok(LogicalPlan::Project {
                            input: Box::new(pushed),
                            projections: columns,
                        })
                    }
                    LogicalPlan::Limit {
                        input: inner_input,
                        count: inner_count,
                        offset: inner_offset,
                    } => {
                        let final_count = count.min(inner_count);
                        let final_offset = offset + inner_offset;
                        Ok(LogicalPlan::Limit {
                            input: inner_input,
                            count: final_count,
                            offset: final_offset,
                        })
                    }
                    other => Ok(LogicalPlan::Limit {
                        input: Box::new(other),
                        count,
                        offset,
                    }),
                }
            }
            LogicalPlan::Project {
                input,
                projections: columns,
            } => Ok(LogicalPlan::Project {
                input: Box::new(self.push_down(*input)?),
                projections: columns,
            }),
            LogicalPlan::Filter { input, predicate } => Ok(LogicalPlan::Filter {
                input: Box::new(self.push_down(*input)?),
                predicate,
            }),
            LogicalPlan::Sort { input, order_by } => Ok(LogicalPlan::Sort {
                input: Box::new(self.push_down(*input)?),
                order_by,
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
}
