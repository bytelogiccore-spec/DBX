//! SQL 쿼리 옵티마이저 — 규칙 기반 최적화
//!
//! LogicalPlan을 최적화하여 실행 성능을 향상시킵니다.
//! 4가지 핵심 규칙: PredicatePushdown, ProjectionPushdown, ConstantFolding, LimitPushdown

mod constant_folding;
mod limit_pushdown;
mod predicate_pushdown;
mod projection_pushdown;

#[cfg(test)]
mod tests;

use crate::error::DbxResult;
use crate::sql::planner::LogicalPlan;

pub use constant_folding::ConstantFoldingRule;
pub use limit_pushdown::LimitPushdownRule;
pub use predicate_pushdown::PredicatePushdownRule;
pub use projection_pushdown::ProjectionPushdownRule;

/// 최적화 규칙 트레이트
pub trait OptimizationRule: Send + Sync {
    /// 규칙 이름
    fn name(&self) -> &str;

    /// LogicalPlan에 규칙 적용
    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan>;
}

/// 쿼리 옵티마이저
pub struct QueryOptimizer {
    rules: Vec<Box<dyn OptimizationRule>>,
}

impl QueryOptimizer {
    /// 기본 최적화 규칙으로 생성
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(PredicatePushdownRule),
                Box::new(ProjectionPushdownRule),
                Box::new(ConstantFoldingRule),
                Box::new(LimitPushdownRule),
            ],
        }
    }

    /// 모든 규칙 적용
    pub fn optimize(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        let mut optimized = plan;
        for rule in &self.rules {
            optimized = rule.apply(optimized)?;
        }
        Ok(optimized)
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
