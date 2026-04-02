use crate::error::DbxResult;
use crate::sql::optimizer::OptimizationRule;
use crate::sql::planner::LogicalPlan;
use crate::storage::metadata::MetadataRegistry;
use std::sync::Arc;

/// 5-Tier Storage 푸루닝 최적화 규칙 (Phase 6)
/// MetadataRegistry를 조회하여 쿼리 조건에 부합하는 파티션(Parquet 파일)만
/// 논리 플랜(TableScan)에 주입합니다.
pub struct TierPruningRule {
    registry: Arc<MetadataRegistry>,
}

impl TierPruningRule {
    pub fn new(registry: Arc<MetadataRegistry>) -> Self {
        Self { registry }
    }
}

impl OptimizationRule for TierPruningRule {
    fn name(&self) -> &str {
        "TierPruning"
    }

    fn apply(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        // 재귀 탐색을 통해 모든 LogicalPlan::Scan 노드 탐색 및 수정
        self.prune(plan)
    }
}

impl TierPruningRule {
    fn prune(&self, plan: LogicalPlan) -> DbxResult<LogicalPlan> {
        match plan {
            LogicalPlan::Scan {
                table,
                columns,
                filter,
                ros_files: _, // 기존값 무시
            } => {
                let mut valid_ros_files = vec![];

                // 1. 레지스트리 조회
                let partitions = self
                    .registry
                    .tables
                    .get(&table)
                    .map(|t| {
                        t.partitions
                            .iter()
                            .map(|kv| kv.value().clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                // 2. 푸루닝 (단위 MVP: 우선 모든 ROS 파티션 등록, 추후 filter 분석 고도화)
                // TODO: Predicate Pushdown (filter) 기반으로 PartitionMeta.min_key ~ max_key 범위 필터링 기능 추가
                for partition in partitions {
                    if partition.tier == crate::storage::metadata::StorageTier::DiskROS {
                        valid_ros_files.push(partition.file_path.clone());
                    }
                }

                Ok(LogicalPlan::Scan {
                    table,
                    columns,
                    filter,
                    ros_files: valid_ros_files,
                })
            }
            LogicalPlan::Project { input, projections } => Ok(LogicalPlan::Project {
                input: Box::new(self.prune(*input)?),
                projections,
            }),
            LogicalPlan::Filter { input, predicate } => Ok(LogicalPlan::Filter {
                input: Box::new(self.prune(*input)?),
                predicate,
            }),
            LogicalPlan::Sort { input, order_by } => Ok(LogicalPlan::Sort {
                input: Box::new(self.prune(*input)?),
                order_by,
            }),
            LogicalPlan::Limit {
                input,
                count,
                offset,
            } => Ok(LogicalPlan::Limit {
                input: Box::new(self.prune(*input)?),
                count,
                offset,
            }),
            LogicalPlan::Aggregate {
                input,
                group_by,
                aggregates,
                mode,
            } => Ok(LogicalPlan::Aggregate {
                input: Box::new(self.prune(*input)?),
                group_by,
                aggregates,
                mode,
            }),
            LogicalPlan::Join {
                left,
                right,
                join_type,
                on,
            } => Ok(LogicalPlan::Join {
                left: Box::new(self.prune(*left)?),
                right: Box::new(self.prune(*right)?),
                join_type,
                on,
            }),
            // 데이터 수정/DDL 등 스캔 노드가 없는 나머지 플랜은 그대로 반환
            other => Ok(other),
        }
    }
}
