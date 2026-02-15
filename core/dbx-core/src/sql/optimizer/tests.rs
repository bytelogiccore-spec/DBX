use super::*;
use crate::sql::planner::{BinaryOperator, Expr, LogicalPlan};
use crate::storage::columnar::ScalarValue;

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
        } => {}
        other => panic!("Expected Scan with filter, got: {:?}", other),
    }
}

#[test]
fn test_predicate_pushdown_merge_filters() {
    let rule = PredicatePushdownRule;
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
        } => {}
        other => panic!("Expected combined AND filter, got: {:?}", other),
    }
}

#[test]
fn test_predicate_pushdown_below_project() {
    let rule = PredicatePushdownRule;
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
    let plan = LogicalPlan::Filter {
        input: Box::new(scan("t")),
        predicate: Expr::BinaryOp {
            left: Box::new(Expr::Literal(ScalarValue::Int32(1))),
            op: BinaryOperator::Eq,
            right: Box::new(Expr::Literal(ScalarValue::Int32(1))),
        },
    };
    let optimized = rule.apply(plan).unwrap();
    match optimized {
        LogicalPlan::Scan { .. } => {}
        other => panic!("Expected Scan (filter eliminated), got: {:?}", other),
    }
}

#[test]
fn test_constant_folding_nested() {
    let rule = ConstantFoldingRule;
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
            LogicalPlan::Limit { count: 10, .. } => {}
            other => panic!("Expected Limit below Project, got: {:?}", other),
        },
        other => panic!("Expected Project at top, got: {:?}", other),
    }
}

#[test]
fn test_limit_merge_consecutive() {
    let rule = LimitPushdownRule;
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
        LogicalPlan::Limit { count: 5, .. } => {}
        other => panic!("Expected Limit(5), got: {:?}", other),
    }
}

// ── Full optimizer pipeline ──

#[test]
fn test_full_optimizer_pipeline() {
    let optimizer = QueryOptimizer::new();
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
    assert!(matches!(optimized, LogicalPlan::Project { .. }));
}
