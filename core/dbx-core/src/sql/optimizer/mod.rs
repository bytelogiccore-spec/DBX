//! SQL 쿼리 옵티마이저 — 규칙 기반 최적화
//!
//! LogicalPlan을 최적화하여 실행 성능을 향상시킵니다.
//! 4가지 핵심 규칙: PredicatePushdown, ProjectionPushdown, ConstantFolding, LimitPushdown

mod constant_folding;
mod distributed_pushdown;
mod limit_pushdown;
mod predicate_pushdown;
mod projection_pushdown;
mod subquery_unnesting;
mod tier_pruning;

#[cfg(test)]
mod tests;

use crate::error::DbxResult;
use crate::sql::planner::LogicalPlan;

pub use constant_folding::ConstantFoldingRule;
pub use distributed_pushdown::DistributedPushdownRule;
pub use limit_pushdown::LimitPushdownRule;
pub use predicate_pushdown::PredicatePushdownRule;
pub use projection_pushdown::ProjectionPushdownRule;
pub use subquery_unnesting::SubqueryUnnestingRule;
pub use tier_pruning::TierPruningRule;

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
                Box::new(SubqueryUnnestingRule), // 가장 먼저 서브쿼리 해제
                Box::new(PredicatePushdownRule),
                Box::new(ProjectionPushdownRule),
                Box::new(ConstantFoldingRule),
                Box::new(DistributedPushdownRule),
                Box::new(LimitPushdownRule),
            ],
        }
    }

    /// 외부 구성 규칙 등록 (Phase 6 MetadataRegistry 연계 등)
    pub fn register_rule(&mut self, rule: Box<dyn OptimizationRule>) {
        // 푸루닝 규칙은 보통 푸시다운 이전/이후에 실행. 여기서는 맨 마지막(또는 첫번째)에 추가
        self.rules.push(rule);
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
