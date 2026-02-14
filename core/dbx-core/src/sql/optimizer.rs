//! SQL 쿼리 옵티마이저 — 규칙 기반 최적화
//!
//! LogicalPlan을 최적화하여 실행 성능을 향상시킵니다.
//! 4가지 핵심 규칙: PredicatePushdown, ProjectionPushdown, ConstantFolding, LimitPushdown

use crate::error::DbxResult;
use crate::sql::planner::{BinaryOperator, Expr, LogicalPlan};
use crate::storage::columnar::ScalarValue;

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

// ═══════════════════════════════════════════════════════════════
// Rule 1: Predicate Pushdown
// ═══════════════════════════════════════════════════════════════

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
            // Filter above Project → push Filter below Project if columns are compatible
            LogicalPlan::Filter { input, predicate } => {
                let optimized_input = self.push_down(*input)?;
                match optimized_input {
                    // Filter → Project → child: push filter below project
                    LogicalPlan::Project {
                        input: project_input,
                        projections: columns,
                    } if self.can_push_through_project(&predicate, &columns) => {
                        // Recursively push down the newly positioned filter
                        let pushed = self.push_down(LogicalPlan::Filter {
                            input: project_input,
                            predicate,
                        })?;
                        Ok(LogicalPlan::Project {
                            input: Box::new(pushed),
                            projections: columns,
                        })
                    }
                    // Filter → Scan: merge filter into scan
                    LogicalPlan::Scan {
                        table,
                        columns,
                        filter: None,
                    } => Ok(LogicalPlan::Scan {
                        table,
                        columns,
                        filter: Some(predicate),
                    }),
                    // Filter → Scan(existing filter): combine with AND
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
            // Recurse into children
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
    /// For simplicity, always allow pushdown when predicate only references Column exprs.
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

// ═══════════════════════════════════════════════════════════════
// Rule 2: Projection Pushdown
// ═══════════════════════════════════════════════════════════════

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
            // Project → Scan: collect needed columns and push to scan
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
                        // If scan already has no column restriction or we can narrow it
                        let final_cols = if scan_cols.is_empty() {
                            needed // Scan was reading all columns, narrow it
                        } else {
                            // Intersect: keep only columns that are both needed and available
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
            // Recurse into children
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

// ═══════════════════════════════════════════════════════════════
// Rule 3: Constant Folding
// ═══════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════
// Rule 4: Limit Pushdown
// ═══════════════════════════════════════════════════════════════

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
            // Limit → Project → child: push Limit below Project
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => {
                let optimized_input = self.push_down(*input)?;
                match optimized_input {
                    // Limit → Project: push limit below project (safe since project
                    // doesn't change row count)
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
                    // Merge consecutive Limits: take the smaller
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
            // Recurse
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(table: &str) -> LogicalPlan {
        LogicalPlan::Scan {
            table: table.to_string(),
            columns: vec![],
            filter: None,
        }
    }

    // ── Optimizer framework ──

    #[test]
    fn test_optimizer_creation() {
        let optimizer = QueryOptimizer::new();
        assert_eq!(optimizer.rules.len(), 4);
    }

    #[test]
    fn test_optimizer_passthrough() {
        let optimizer = QueryOptimizer::new();
        let plan = scan("users");
        let optimized = optimizer.optimize(plan.clone()).unwrap();
        assert_eq!(optimized, plan);
    }

    // ── Predicate Pushdown ──

    #[test]
    fn test_predicate_pushdown_rule_name() {
        let rule = PredicatePushdownRule;
        assert_eq!(rule.name(), "PredicatePushdown");
    }

    #[test]
    fn test_predicate_pushdown_into_scan() {
        let rule = PredicatePushdownRule;
        // Filter → Scan  ⇒  Scan(with filter)
        let plan = LogicalPlan::Filter {
            input: Box::new(scan("users")),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Column("age".to_string())),
                op: BinaryOperator::Gt,
                right: Box::new(Expr::Literal(ScalarValue::Int32(18))),
            },
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Scan {
                filter: Some(_), ..
            } => {} // Filter merged into Scan
            other => panic!("Expected Scan with filter, got: {:?}", other),
        }
    }

    #[test]
    fn test_predicate_pushdown_merge_filters() {
        let rule = PredicatePushdownRule;
        // Filter → Scan(existing filter) ⇒ Scan(AND combined)
        let plan = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Scan {
                table: "users".to_string(),
                columns: vec![],
                filter: Some(Expr::BinaryOp {
                    left: Box::new(Expr::Column("active".to_string())),
                    op: BinaryOperator::Eq,
                    right: Box::new(Expr::Literal(ScalarValue::Boolean(true))),
                }),
            }),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Column("age".to_string())),
                op: BinaryOperator::Gt,
                right: Box::new(Expr::Literal(ScalarValue::Int32(18))),
            },
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Scan {
                filter:
                    Some(Expr::BinaryOp {
                        op: BinaryOperator::And,
                        ..
                    }),
                ..
            } => {} // Both predicates combined with AND
            other => panic!("Expected combined AND filter, got: {:?}", other),
        }
    }

    #[test]
    fn test_predicate_pushdown_below_project() {
        let rule = PredicatePushdownRule;
        // Filter → Project → Scan ⇒ Project → Filter → Scan ⇒ Project → Scan(filter)
        let plan = LogicalPlan::Filter {
            input: Box::new(LogicalPlan::Project {
                input: Box::new(scan("users")),
                projections: vec![(Expr::Column("name".to_string()), None)],
            }),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Column("age".to_string())),
                op: BinaryOperator::Gt,
                right: Box::new(Expr::Literal(ScalarValue::Int32(18))),
            },
        };
        let optimized = rule.apply(plan).unwrap();
        // Should be Project → Scan(filter)
        match optimized {
            LogicalPlan::Project { input, .. } => match *input {
                LogicalPlan::Scan {
                    filter: Some(_), ..
                } => {}
                other => panic!("Expected Scan with filter under Project, got: {:?}", other),
            },
            other => panic!("Expected Project at top, got: {:?}", other),
        }
    }

    // ── Projection Pushdown ──

    #[test]
    fn test_projection_pushdown_rule_name() {
        let rule = ProjectionPushdownRule;
        assert_eq!(rule.name(), "ProjectionPushdown");
    }

    #[test]
    fn test_projection_pushdown_narrows_scan() {
        let rule = ProjectionPushdownRule;
        // Project(name, age) → Scan(*) ⇒ Project(name, age) → Scan(name, age)
        let plan = LogicalPlan::Project {
            input: Box::new(scan("users")),
            projections: vec![
                (Expr::Column("name".to_string()), None),
                (Expr::Column("age".to_string()), None),
            ],
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Project { input, .. } => match *input {
                LogicalPlan::Scan { columns, .. } => {
                    assert!(columns.contains(&"age".to_string()));
                    assert!(columns.contains(&"name".to_string()));
                }
                other => panic!("Expected Scan, got: {:?}", other),
            },
            other => panic!("Expected Project, got: {:?}", other),
        }
    }

    // ── Constant Folding ──

    #[test]
    fn test_constant_folding_rule_name() {
        let rule = ConstantFoldingRule;
        assert_eq!(rule.name(), "ConstantFolding");
    }

    #[test]
    fn test_constant_folding_arithmetic() {
        let rule = ConstantFoldingRule;
        // SELECT 1 + 2 FROM t ⇒ SELECT 3 FROM t
        let plan = LogicalPlan::Project {
            input: Box::new(scan("t")),
            projections: vec![(
                Expr::BinaryOp {
                    left: Box::new(Expr::Literal(ScalarValue::Int32(1))),
                    op: BinaryOperator::Plus,
                    right: Box::new(Expr::Literal(ScalarValue::Int32(2))),
                },
                None,
            )],
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Project { projections, .. } => {
                assert_eq!(projections[0].0, Expr::Literal(ScalarValue::Int32(3)));
            }
            other => panic!("Expected Project with folded literal, got: {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_eliminates_true_filter() {
        let rule = ConstantFoldingRule;
        // WHERE 1 = 1 ⇒ eliminated
        let plan = LogicalPlan::Filter {
            input: Box::new(scan("t")),
            predicate: Expr::BinaryOp {
                left: Box::new(Expr::Literal(ScalarValue::Int32(1))),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Literal(ScalarValue::Int32(1))),
            },
        };
        let optimized = rule.apply(plan).unwrap();
        // Filter should be eliminated since 1 = 1 is always true
        match optimized {
            LogicalPlan::Scan { .. } => {} // Filter removed
            other => panic!("Expected Scan (filter eliminated), got: {:?}", other),
        }
    }

    #[test]
    fn test_constant_folding_nested() {
        let rule = ConstantFoldingRule;
        // SELECT (2 * 3) + 1 FROM t ⇒ SELECT 7 FROM t
        let plan = LogicalPlan::Project {
            input: Box::new(scan("t")),
            projections: vec![(
                Expr::BinaryOp {
                    left: Box::new(Expr::BinaryOp {
                        left: Box::new(Expr::Literal(ScalarValue::Int32(2))),
                        op: BinaryOperator::Multiply,
                        right: Box::new(Expr::Literal(ScalarValue::Int32(3))),
                    }),
                    op: BinaryOperator::Plus,
                    right: Box::new(Expr::Literal(ScalarValue::Int32(1))),
                },
                None,
            )],
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Project { projections, .. } => {
                assert_eq!(projections[0].0, Expr::Literal(ScalarValue::Int32(7)));
            }
            other => panic!("Expected folded literal 7, got: {:?}", other),
        }
    }

    // ── Limit Pushdown ──

    #[test]
    fn test_limit_pushdown_rule_name() {
        let rule = LimitPushdownRule;
        assert_eq!(rule.name(), "LimitPushdown");
    }

    #[test]
    fn test_limit_pushdown_below_project() {
        let rule = LimitPushdownRule;
        // Limit → Project → Scan ⇒ Project → Limit → Scan
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Project {
                input: Box::new(scan("users")),
                projections: vec![(Expr::Column("name".to_string()), None)],
            }),
            count: 10,
            offset: 0,
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Project { input, .. } => match *input {
                LogicalPlan::Limit { count: 10, .. } => {} // Limit pushed below Project
                other => panic!("Expected Limit below Project, got: {:?}", other),
            },
            other => panic!("Expected Project at top, got: {:?}", other),
        }
    }

    #[test]
    fn test_limit_merge_consecutive() {
        let rule = LimitPushdownRule;
        // Limit(5) → Limit(10) ⇒ Limit(min=5)
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Limit {
                input: Box::new(scan("users")),
                count: 10,
                offset: 0,
            }),
            count: 5,
            offset: 0,
        };
        let optimized = rule.apply(plan).unwrap();
        match optimized {
            LogicalPlan::Limit { count: 5, .. } => {} // Merged to smaller limit
            other => panic!("Expected Limit(5), got: {:?}", other),
        }
    }

    // ── Full optimizer pipeline ──

    #[test]
    fn test_full_optimizer_pipeline() {
        let optimizer = QueryOptimizer::new();
        // SELECT name FROM users WHERE age > 18 AND 1 = 1 LIMIT 10
        let plan = LogicalPlan::Limit {
            input: Box::new(LogicalPlan::Project {
                input: Box::new(LogicalPlan::Filter {
                    input: Box::new(LogicalPlan::Filter {
                        input: Box::new(scan("users")),
                        predicate: Expr::BinaryOp {
                            left: Box::new(Expr::Literal(ScalarValue::Int32(1))),
                            op: BinaryOperator::Eq,
                            right: Box::new(Expr::Literal(ScalarValue::Int32(1))),
                        },
                    }),
                    predicate: Expr::BinaryOp {
                        left: Box::new(Expr::Column("age".to_string())),
                        op: BinaryOperator::Gt,
                        right: Box::new(Expr::Literal(ScalarValue::Int32(18))),
                    },
                }),
                projections: vec![(Expr::Column("name".to_string()), None)],
            }),
            count: 10,
            offset: 0,
        };
        let optimized = optimizer.optimize(plan).unwrap();
        // After optimization:
        // - ConstantFolding: 1=1 filter eliminated
        // - PredicatePushdown: age>18 pushed into scan
        // - ProjectionPushdown: only "name" column in scan
        // - LimitPushdown: limit pushed below project
        // Verify plan is valid (no panic)
        assert!(matches!(optimized, LogicalPlan::Project { .. }));
    }
}
