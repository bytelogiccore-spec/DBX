// SQL 모듈 진입점
pub mod builder;
pub mod executor;
pub mod interface;
pub mod optimizer;
pub mod parallel_parser;
pub mod parser;
pub mod planner;
pub mod view;

pub use builder::{Execute, Query, QueryOne, QueryOptional, QueryScalar, ScalarValue};
pub use executor::{
    FilterOperator, HashAggregateOperator, HashJoinOperator, LimitOperator, PhysicalOperator,
    ProjectionOperator, SortOperator, TableScanOperator, evaluate_expr,
};
pub use optimizer::{OptimizationRule, QueryOptimizer};
pub use parallel_parser::ParallelSqlParser;
pub use parser::SqlParser;
pub use planner::{
    AggregateExpr, AggregateFunction, BinaryOperator, Expr, JoinType, LogicalPlan, LogicalPlanner,
    PhysicalAggExpr, PhysicalExpr, PhysicalPlan, PhysicalPlanner, SortExpr,
};
pub trait StringCaseExt {
    fn starts_with_ignore_ascii_case(&self, prefix: &str) -> bool;
    fn contains_ignore_ascii_case(&self, substring: &str) -> bool;
}

impl StringCaseExt for str {
    fn starts_with_ignore_ascii_case(&self, prefix: &str) -> bool {
        if self.len() < prefix.len() {
            false
        } else {
            self.is_char_boundary(prefix.len()) && self[..prefix.len()].eq_ignore_ascii_case(prefix)
        }
    }

    fn contains_ignore_ascii_case(&self, substring: &str) -> bool {
        if substring.is_empty() {
            return true;
        }
        if self.len() < substring.len() {
            return false;
        }

        // Find indices safely using char boundaries
        for (idx, _) in self.char_indices() {
            if idx + substring.len() <= self.len() {
                if self.is_char_boundary(idx + substring.len())
                    && self[idx..idx + substring.len()].eq_ignore_ascii_case(substring)
                {
                    return true;
                }
            } else {
                break;
            }
        }
        false
    }
}
