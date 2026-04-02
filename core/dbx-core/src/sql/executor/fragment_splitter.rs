//! Fragment Splitter — 분산 쿼리 플랜 분할기
//!
//! PhysicalPlan 트리를 분석하여 분산 실행 경계(Partial/Final Aggregate)를 찾고,
//! 코디네이터가 실행할 서브트리와 워커가 실행할 서브트리로 분리합니다.
//!
//! 분기 규칙:
//!   - `PhysicalPlan::HashAggregate { mode: Final }` 노드부터 위가 Coordinator Fragment
//!   - `PhysicalPlan::HashAggregate { mode: Partial }` 노드부터 아래가 Worker Fragment
//!   - Coordinator Fragment의 입력은 `GridExchangeOperator`로 대체됨 (채널 연결)
//!
//! # 지원하지 않는 케이스 (Phase 3 범위 외)
//! - 분산 Join (Shuffle Exchange) — Phase 4에서 구현

use crate::error::DbxResult;
use crate::sql::planner::types::{AggregateMode, PhysicalPlan};

pub struct FragmentStage {
    pub stage_id: usize,
    pub plans: Vec<PhysicalPlan>,
}

/// 분할 결과 — 코디네이터 플랜과 스테이지 배열의 쌍 (Fragment DAG)
pub struct FragmentDAG {
    /// 코디네이터가 실행할 플랜 (Join 이상, 입력은 Exchange 수신으로 교체됨)
    /// `None`이면 플랜이 분산 실행 대상이 아님 (단일 노드 실행)
    pub coordinator_plan: Option<PhysicalPlan>,
    /// 워커 노드들이 실행할 스테이지 플랜들의 위상 정렬된 배열
    pub stages: Vec<FragmentStage>,
}

pub struct FragmentSplitter;

impl FragmentSplitter {
    /// PhysicalPlan → FragmentDAG 분할
    ///
    /// 분산 실행 경계(Final Agg 또는 HashJoin)를 찾으면 분리하고,
    /// 찾을 수 없으면 전체 플랜을 단일 스테이지 플랜으로 반환합니다 (`coordinator_plan: None`).
    pub fn split(plan: PhysicalPlan) -> DbxResult<FragmentDAG> {
        match Self::try_split(plan)? {
            SplitResult::SplitDAG {
                coordinator,
                stages,
            } => Ok(FragmentDAG {
                coordinator_plan: Some(coordinator),
                stages,
            }),
            SplitResult::Split {
                coordinator,
                worker,
            } => Ok(FragmentDAG {
                coordinator_plan: Some(coordinator),
                stages: vec![FragmentStage {
                    stage_id: 1,
                    plans: vec![worker],
                }],
            }),
            SplitResult::Unsplit(plan) => Ok(FragmentDAG {
                coordinator_plan: None,
                stages: vec![FragmentStage {
                    stage_id: 1,
                    plans: vec![plan],
                }],
            }),
        }
    }

    fn try_split(plan: PhysicalPlan) -> DbxResult<SplitResult> {
        match plan {
            // ─── 분기점: Final Aggregate ───────────────────────────────────────
            // Final Agg를 만나면 그 입력(Partial Agg + Scan)이 워커 플랜,
            // Final Agg 자체 + 상위 노드가 코디네이터 플랜.
            // 이 메서드는 인입 플랜 전체를 교체하므로 Final Agg 노드 자체가 루트라고 가정.
            PhysicalPlan::HashAggregate {
                input,
                group_by,
                aggregates,
                mode: AggregateMode::Final,
            } => {
                // Worker Fragment: Partial Agg가 루트인 서브트리
                let worker_plan = *input;

                // Coordinator Fragment: Final Agg, 입력은 GridExchange(Placeholder)로 교체.
                let coord_plan = PhysicalPlan::HashAggregate {
                    input: Box::new(PhysicalPlan::GridExchange {
                        exchange_id: 1, // Agg용 기본 코디네이터 단일 Exchange ID
                        schema_hint: extract_output_columns(&worker_plan),
                    }),
                    group_by,
                    aggregates,
                    mode: AggregateMode::Final,
                };

                // Aggregation은 ShuffleWriter (브로드캐스트) 로 감싸서 워커에 배치
                let worker_shuffle = PhysicalPlan::ShuffleWriter {
                    input: Box::new(worker_plan),
                    hash_params: vec![], // ShuffleSalting::ReplicateProbe 등이 사용됨
                    target_nodes: vec![], // DistributedExecutor가 라우팅
                    exchange_id: 1,
                    salting: crate::sql::planner::types::ShuffleSalting::None,
                };

                Ok(SplitResult::SplitDAG {
                    coordinator: coord_plan,
                    stages: vec![FragmentStage {
                        stage_id: 1,
                        plans: vec![worker_shuffle],
                    }],
                })
            }

            // ─── 분기점: HashJoin (Multi-stage DAG) ─────────────────────────
            PhysicalPlan::HashJoin {
                left,
                right,
                join_type,
                on,
            } => {
                let left_worker = *left;
                let right_worker = *right;

                // Coordinator: HashJoin 수신부
                let coord_plan = PhysicalPlan::HashJoin {
                    left: Box::new(PhysicalPlan::GridExchange {
                        exchange_id: 1,
                        schema_hint: extract_output_columns(&left_worker),
                    }),
                    right: Box::new(PhysicalPlan::GridExchange {
                        exchange_id: 2,
                        schema_hint: extract_output_columns(&right_worker),
                    }),
                    join_type,
                    on,
                };

                // Stage 1: Left (Build)
                let left_shuffle = PhysicalPlan::ShuffleWriter {
                    input: Box::new(left_worker),
                    hash_params: vec![], // 향후 Hash 분할용 확장
                    target_nodes: vec![],
                    exchange_id: 1,
                    salting: crate::sql::planner::types::ShuffleSalting::None,
                };

                // Stage 2: Right (Probe)
                let right_shuffle = PhysicalPlan::ShuffleWriter {
                    input: Box::new(right_worker),
                    hash_params: vec![],
                    target_nodes: vec![],
                    exchange_id: 2,
                    salting: crate::sql::planner::types::ShuffleSalting::None,
                };

                Ok(SplitResult::SplitDAG {
                    coordinator: coord_plan,
                    stages: vec![
                        FragmentStage {
                            stage_id: 1,
                            plans: vec![left_shuffle],
                        },
                        FragmentStage {
                            stage_id: 2,
                            plans: vec![right_shuffle],
                        },
                    ],
                })
            }

            // ─── 상위 래퍼: 재귀 탐색 ──────────────────────────────────────────
            PhysicalPlan::Projection {
                input,
                exprs,
                aliases,
            } => match Self::try_split(*input)? {
                SplitResult::SplitDAG {
                    coordinator,
                    stages,
                } => Ok(SplitResult::SplitDAG {
                    coordinator: PhysicalPlan::Projection {
                        input: Box::new(coordinator),
                        exprs,
                        aliases,
                    },
                    stages,
                }),
                SplitResult::Split {
                    coordinator,
                    worker,
                } => Ok(SplitResult::Split {
                    coordinator: PhysicalPlan::Projection {
                        input: Box::new(coordinator),
                        exprs: exprs.clone(),
                        aliases: aliases.clone(),
                    },
                    worker,
                }),
                SplitResult::Unsplit(unchanged) => {
                    Ok(SplitResult::Unsplit(PhysicalPlan::Projection {
                        input: Box::new(unchanged),
                        exprs,
                        aliases,
                    }))
                }
            },

            PhysicalPlan::Limit {
                input,
                count,
                offset,
            } => match Self::try_split(*input)? {
                SplitResult::SplitDAG {
                    coordinator,
                    stages,
                } => Ok(SplitResult::SplitDAG {
                    coordinator: PhysicalPlan::Limit {
                        input: Box::new(coordinator),
                        count,
                        offset,
                    },
                    stages,
                }),
                SplitResult::Split {
                    coordinator,
                    worker,
                } => Ok(SplitResult::Split {
                    coordinator: PhysicalPlan::Limit {
                        input: Box::new(coordinator),
                        count,
                        offset,
                    },
                    worker,
                }),
                SplitResult::Unsplit(unchanged) => Ok(SplitResult::Unsplit(PhysicalPlan::Limit {
                    input: Box::new(unchanged),
                    count,
                    offset,
                })),
            },

            PhysicalPlan::SortMerge { input, order_by } => match Self::try_split(*input)? {
                SplitResult::SplitDAG {
                    coordinator,
                    stages,
                } => Ok(SplitResult::SplitDAG {
                    coordinator: PhysicalPlan::SortMerge {
                        input: Box::new(coordinator),
                        order_by: order_by.clone(),
                    },
                    stages,
                }),
                SplitResult::Split {
                    coordinator,
                    worker,
                } => Ok(SplitResult::Split {
                    coordinator: PhysicalPlan::SortMerge {
                        input: Box::new(coordinator),
                        order_by: order_by.clone(),
                    },
                    worker,
                }),
                SplitResult::Unsplit(unchanged) => {
                    Ok(SplitResult::Unsplit(PhysicalPlan::SortMerge {
                        input: Box::new(unchanged),
                        order_by,
                    }))
                }
            },

            // ─── 분기 불명: 단일 노드 플랜으로 반환 ──────────────────────────
            other => Ok(SplitResult::Unsplit(other)),
        }
    }
}

/// 내부 분할 상태
enum SplitResult {
    SplitDAG {
        coordinator: PhysicalPlan,
        stages: Vec<FragmentStage>,
    },
    #[allow(dead_code)]
    Split {
        coordinator: PhysicalPlan,
        worker: PhysicalPlan,
    },
    Unsplit(PhysicalPlan),
}

/// 워커 플랜 루트의 출력 컬럼 수를 추정 (GridExchange 플레이스홀더 스키마 힌트용)
fn extract_output_columns(plan: &PhysicalPlan) -> usize {
    match plan {
        PhysicalPlan::HashAggregate {
            group_by,
            aggregates,
            ..
        } => group_by.len() + aggregates.len(),
        PhysicalPlan::Projection { exprs, .. } => exprs.len(),
        PhysicalPlan::TableScan {
            projection,
            ros_files: _,
            ..
        } => {
            if projection.is_empty() {
                8
            } else {
                projection.len()
            }
        }
        _ => 4, // 안전한 기본값
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::planner::types::{
        AggregateFunction, AggregateMode, PhysicalAggExpr, PhysicalPlan,
    };

    fn make_partial_agg() -> PhysicalPlan {
        PhysicalPlan::HashAggregate {
            input: Box::new(PhysicalPlan::TableScan {
                table: "sales".to_string(),
                projection: vec![],
                filter: None,
                ros_files: vec![],
            }),
            group_by: vec![0],
            aggregates: vec![PhysicalAggExpr {
                function: AggregateFunction::Sum,
                input: 1,
                alias: Some("partial_sum".to_string()),
            }],
            mode: AggregateMode::Partial,
        }
    }

    #[test]
    fn test_split_final_over_partial_agg() {
        let plan = PhysicalPlan::HashAggregate {
            input: Box::new(make_partial_agg()),
            group_by: vec![0],
            aggregates: vec![PhysicalAggExpr {
                function: AggregateFunction::Sum,
                input: 1,
                alias: Some("total_sum".to_string()),
            }],
            mode: AggregateMode::Final,
        };

        let dag = FragmentSplitter::split(plan).unwrap();

        // 코디네이터 플랜 존재 확인
        assert!(
            dag.coordinator_plan.is_some(),
            "coordinator_plan should be Some"
        );
        let coord = dag.coordinator_plan.unwrap();

        // 코디네이터 플랜이 Final Agg인지 확인
        assert!(matches!(
            coord,
            PhysicalPlan::HashAggregate {
                mode: AggregateMode::Final,
                ..
            }
        ));

        // 코디네이터 입력이 GridExchange인지 확인
        if let PhysicalPlan::HashAggregate { input, .. } = &coord {
            assert!(
                matches!(**input, PhysicalPlan::GridExchange { .. }),
                "coordinator input should be GridExchange"
            );
        }

        // 워커 플랜 스테이지 확인
        assert_eq!(dag.stages.len(), 1, "Should have 1 stage for aggregation");
        let worker_plan = &dag.stages[0].plans[0];

        // 플랜이 ShuffleWriter로 래핑되어 있는지 확인
        if let PhysicalPlan::ShuffleWriter { input, .. } = worker_plan {
            assert!(matches!(
                **input,
                PhysicalPlan::HashAggregate {
                    mode: AggregateMode::Partial,
                    ..
                }
            ));
        } else {
            panic!("Expected ShuffleWriter");
        }
    }

    #[test]
    fn test_no_split_simple_scan() {
        let plan = PhysicalPlan::TableScan {
            table: "T1".to_string(),
            projection: vec![],
            filter: None,
            ros_files: vec![],
        };

        let dag = FragmentSplitter::split(plan).unwrap();
        assert!(
            dag.coordinator_plan.is_none(),
            "simple scan should not split"
        );
        assert_eq!(dag.stages.len(), 1);
        assert_eq!(dag.stages[0].plans.len(), 1);
    }
}
