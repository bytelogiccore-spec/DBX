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
use crate::sql::planner::types::{AggregateMode, PhysicalAggExpr, PhysicalPlan};

/// 분할 결과 — 코디네이터 플랜과 워커 플랜의 쌍
pub struct FragmentPair {
    /// 코디네이터가 실행할 플랜 (Final Agg 이상, 입력은 Exchange 수신으로 교체됨)
    /// `None`이면 플랜이 분산 실행 대상이 아님 (단일 노드 실행)
    pub coordinator_plan: Option<PhysicalPlan>,
    /// 워커 노드가 실행할 플랜 (Partial Agg 이하)
    pub worker_plan: PhysicalPlan,
}

pub struct FragmentSplitter;

impl FragmentSplitter {
    /// PhysicalPlan → FragmentPair 분할
    ///
    /// 분산 실행 경계(Final Agg)를 찾으면 분리하고,
    /// 찾을 수 없으면 전체 플랜을 단일 워커 플랜으로 반환합니다 (`coordinator_plan: None`).
    pub fn split(plan: PhysicalPlan) -> DbxResult<FragmentPair> {
        match Self::try_split(plan)? {
            SplitResult::Split { coordinator, worker } => Ok(FragmentPair {
                coordinator_plan: Some(coordinator),
                worker_plan: worker,
            }),
            SplitResult::Unsplit(plan) => Ok(FragmentPair {
                coordinator_plan: None,
                worker_plan: plan,
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
                // 실제 `GridExchangeOperator` 연결은 DistributedExecutor가 런타임에 담당.
                let coord_plan = PhysicalPlan::HashAggregate {
                    input: Box::new(PhysicalPlan::GridExchange {
                        exchange_id: 0, // 기본 코디네이터 단일 Exchange ID
                        schema_hint: extract_output_columns(&worker_plan),
                    }),
                    group_by,
                    aggregates,
                    mode: AggregateMode::Final,
                };
                Ok(SplitResult::Split {
                    coordinator: coord_plan,
                    worker: worker_plan,
                })
            }

            // ─── 상위 래퍼: 재귀 탐색 ──────────────────────────────────────────
            PhysicalPlan::Projection { input, exprs, aliases } => {
                match Self::try_split(*input)? {
                    SplitResult::Split { coordinator, worker } => Ok(SplitResult::Split {
                        coordinator: PhysicalPlan::Projection {
                            input: Box::new(coordinator),
                            exprs,
                            aliases,
                        },
                        worker,
                    }),
                    SplitResult::Unsplit(unchanged) => Ok(SplitResult::Unsplit(
                        PhysicalPlan::Projection {
                            input: Box::new(unchanged),
                            exprs,
                            aliases,
                        },
                    )),
                }
            }

            PhysicalPlan::Limit { input, count, offset } => {
                match Self::try_split(*input)? {
                    SplitResult::Split { coordinator, worker } => Ok(SplitResult::Split {
                        coordinator: PhysicalPlan::Limit {
                            input: Box::new(coordinator),
                            count,
                            offset,
                        },
                        worker,
                    }),
                    SplitResult::Unsplit(unchanged) => Ok(SplitResult::Unsplit(
                        PhysicalPlan::Limit {
                            input: Box::new(unchanged),
                            count,
                            offset,
                        },
                    )),
                }
            }

            PhysicalPlan::SortMerge { input, order_by } => {
                match Self::try_split(*input)? {
                    SplitResult::Split { coordinator, worker } => Ok(SplitResult::Split {
                        coordinator: PhysicalPlan::SortMerge {
                            input: Box::new(coordinator),
                            order_by,
                        },
                        worker,
                    }),
                    SplitResult::Unsplit(unchanged) => Ok(SplitResult::Unsplit(
                        PhysicalPlan::SortMerge {
                            input: Box::new(unchanged),
                            order_by,
                        },
                    )),
                }
            }

            // ─── 분기 불명: 단일 노드 플랜으로 반환 ──────────────────────────
            other => Ok(SplitResult::Unsplit(other)),
        }
    }
}

/// 내부 분할 상태
enum SplitResult {
    Split {
        coordinator: PhysicalPlan,
        worker: PhysicalPlan,
    },
    Unsplit(PhysicalPlan),
}

/// 워커 플랜 루트의 출력 컬럼 수를 추정 (GridExchange 플레이스홀더 스키마 힌트용)
fn extract_output_columns(plan: &PhysicalPlan) -> usize {
    match plan {
        PhysicalPlan::HashAggregate { group_by, aggregates, .. } => {
            group_by.len() + aggregates.len()
        }
        PhysicalPlan::Projection { exprs, .. } => exprs.len(),
        PhysicalPlan::TableScan { projection, .. } => {
            if projection.is_empty() { 8 } else { projection.len() } // 스키마 없이 추정
        }
        _ => 4, // 안전한 기본값
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::planner::types::{AggregateFunction, AggregateMode, PhysicalAggExpr, PhysicalPlan};

    fn make_partial_agg() -> PhysicalPlan {
        PhysicalPlan::HashAggregate {
            input: Box::new(PhysicalPlan::TableScan {
                table: "sales".to_string(),
                projection: vec![],
                filter: None,
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

        let pair = FragmentSplitter::split(plan).unwrap();

        // 코디네이터 플랜 존재 확인
        assert!(pair.coordinator_plan.is_some(), "coordinator_plan should be Some");
        let coord = pair.coordinator_plan.unwrap();

        // 코디네이터 플랜이 Final Agg인지 확인
        assert!(matches!(
            coord,
            PhysicalPlan::HashAggregate { mode: AggregateMode::Final, .. }
        ));

        // 코디네이터 입력이 GridExchange인지 확인
        if let PhysicalPlan::HashAggregate { input, .. } = &coord {
            assert!(matches!(**input, PhysicalPlan::GridExchange { .. }),
                "coordinator input should be GridExchange");
        }

        // 워커 플랜이 Partial Agg인지 확인
        assert!(matches!(
            pair.worker_plan,
            PhysicalPlan::HashAggregate { mode: AggregateMode::Partial, .. }
        ));
    }

    #[test]
    fn test_no_split_simple_scan() {
        let plan = PhysicalPlan::TableScan {
            table: "t".to_string(),
            projection: vec![],
            filter: None,
        };

        let pair = FragmentSplitter::split(plan).unwrap();
        assert!(pair.coordinator_plan.is_none(), "simple scan should not split");
    }
}
