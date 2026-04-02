//! Rule: Distributed Pushdown
//!
//! 분산 환경(Grid)에 최적화된 실행 계획을 생성합니다.
//! - Partial Aggregate Pushdown: `Aggregate` 노드를 Shard 수준의 `LocalAggregate`와 Coordinator 수준의 `GlobalAggregate`로 분리
//! - Limit Pushdown: 분산 실행 환경에서 불필요한 네트워크 전송을 줄이기 위해 부분 Limit 적용 반영 (현재는 기본 LimitPushdown과 연계)

use super::OptimizationRule;
use crate::error::DbxResult;
use crate::sql::planner::{AggregateMode, LogicalPlan};

pub struct DistributedPushdownRule;

impl OptimizationRule for DistributedPushdownRule {
    fn name(&self) -> &str {
        "DistributedPushdown"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        self.push_down(plan)
    }
}

impl DistributedPushdownRule {
    fn push_down(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            // Traverse down
            LogicalPlan::Project { input, projections } => Ok(LogicalPlan::Project {
                input: Box::new(self.push_down(*input)?),
                projections,
            }),
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
                mode,
            } => {
                let pushed_input = self.push_down(*input)?;

                if mode == AggregateMode::Simple {
                    // Create Partial Aggregate for Shard level
                    let partial_agg = LogicalPlan::Aggregate {
                        input: Box::new(pushed_input),
                        group_by: group_by.clone(),
                        aggregates: aggregates.clone(),
                        mode: AggregateMode::Partial,
                    };

                    // Create Final Aggregate for Coordinator level
                    // For now, Final uses the same expressions, but HashAggregateOperator
                    // will handle the "Merge" logic based on its mode.
                    Ok(LogicalPlan::Aggregate {
                        input: Box::new(partial_agg),
                        group_by,
                        aggregates,
                        mode: AggregateMode::Final,
                    })
                } else {
                    Ok(LogicalPlan::Aggregate {
                        input: Box::new(pushed_input),
                        group_by,
                        aggregates,
                        mode,
                    })
                }
            }
            // Join도 양측 traverse
            LogicalPlan::Join {
                left,
                right,
                join_type,
                on,
            } => Ok(LogicalPlan::Join {
                left: Box::new(self.push_down(*left)?),
                right: Box::new(self.push_down(*right)?),
                join_type,
                on,
            }),
            other => Ok(other),
        }
    }
}
