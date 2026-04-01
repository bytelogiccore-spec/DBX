//! LocalExecutor — 단일 노드 PhysicalPlan 실행기
//!
//! PhysicalPlan 트리를 순회하며 PhysicalOperator 트리를 빌드하고 실행합니다.
//! DistributedExecutor의 워커 측 실행과 단독 쿼리 실행 모두에서 재사용됩니다.

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::{
    FilterOperator, HashAggregateOperator, HashJoinOperator, LimitOperator,
    PhysicalOperator, ProjectionOperator, SortOperator, TableScanOperator,
};
use crate::sql::planner::types::*;
use arrow::array::{RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 단일 노드 물리 플랜 실행기
///
/// 테이블 데이터는 외부에서 `Arc<RwLock<HashMap<String, Vec<RecordBatch>>>>` 로 주입됩니다.
/// 이를 통해 테스트에서 가짜 데이터를 쉽게 주입할 수 있습니다.
pub struct LocalExecutor {
    /// 테이블명 → 데이터 배치 (인메모리 테이블 스토어)
    table_store: Arc<RwLock<HashMap<String, Vec<RecordBatch>>>>,
    /// 테이블 스키마 레지스트리
    table_schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,
}

impl LocalExecutor {
    pub fn new(
        table_store: Arc<RwLock<HashMap<String, Vec<RecordBatch>>>>,
        table_schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,
    ) -> Self {
        Self { table_store, table_schemas }
    }

    /// PhysicalPlan을 실행하고 모든 결과 RecordBatch를 반환합니다.
    pub fn execute_collect(&self, plan: &PhysicalPlan) -> DbxResult<Vec<RecordBatch>> {
        let mut operator = self.build_operator(plan)?;
        let mut results = Vec::new();
        while let Some(batch) = operator.next()? {
            if batch.num_rows() > 0 {
                results.push(batch);
            }
        }
        Ok(results)
    }

    /// PhysicalPlan → Box<dyn PhysicalOperator> 트리 빌드
    pub fn build_operator(&self, plan: &PhysicalPlan) -> DbxResult<Box<dyn PhysicalOperator>> {
        match plan {
            PhysicalPlan::TableScan { table, projection, filter } => {
                let store = self.table_store.read().unwrap();
                let batches = store.get(table).cloned().unwrap_or_default();

                let schemas = self.table_schemas.read().unwrap();
                let schema = schemas.get(table)
                    .cloned()
                    .ok_or_else(|| DbxError::TableNotFound(table.clone()))?;
                drop(schemas);
                drop(store);

                let mut op = TableScanOperator::new(
                    table.clone(),
                    schema,
                    projection.clone(),
                );
                op.set_data(batches);

                // 필터가 있으면 FilterOperator로 래핑
                if let Some(f) = filter {
                    Ok(Box::new(FilterOperator::new(Box::new(op), f.clone())))
                } else {
                    Ok(Box::new(op))
                }
            }

            PhysicalPlan::HashAggregate { input, group_by, aggregates, mode } => {
                let input_op = self.build_operator(input)?;
                let input_schema = input_op.schema().clone();

                // 출력 스키마 구성
                let mut output_fields = Vec::new();
                for &col_idx in group_by.iter() {
                    if col_idx < input_schema.fields().len() {
                        output_fields.push(input_schema.field(col_idx).clone());
                    }
                }
                for agg in aggregates {
                    let name = agg.alias.clone().unwrap_or_else(|| format!("agg_{}", agg.input));
                    let dtype = match agg.function {
                        AggregateFunction::Count => DataType::Int64,
                        AggregateFunction::Sum | AggregateFunction::Avg
                        | AggregateFunction::Min | AggregateFunction::Max => DataType::Float64,
                    };
                    output_fields.push(Field::new(&name, dtype, true));
                }
                let output_schema = Arc::new(Schema::new(output_fields));

                Ok(Box::new(HashAggregateOperator::new(
                    input_op,
                    output_schema,
                    group_by.clone(),
                    aggregates.clone(),
                    *mode,
                )))
            }

            PhysicalPlan::Projection { input, exprs, aliases } => {
                let input_op = self.build_operator(input)?;
                let input_schema = input_op.schema().clone();

                let output_fields: Vec<Field> = exprs.iter().zip(aliases.iter()).map(|(expr, alias)| {
                    let dtype = expr.get_type(&input_schema);
                    let name = alias.clone().unwrap_or_else(|| "col".to_string());
                    Field::new(&name, dtype, true)
                }).collect();
                let output_schema = Arc::new(Schema::new(output_fields));

                Ok(Box::new(ProjectionOperator::new(input_op, output_schema, exprs.clone())))
            }

            PhysicalPlan::Limit { input, count, offset } => {
                let input_op = self.build_operator(input)?;
                Ok(Box::new(LimitOperator::new(input_op, *count, *offset)))
            }

            PhysicalPlan::SortMerge { input, order_by } => {
                let input_op = self.build_operator(input)?;
                Ok(Box::new(SortOperator::new(input_op, order_by.clone())))
            }

            PhysicalPlan::HashJoin { left, right, on, join_type } => {
                let left_op = self.build_operator(left)?;
                let right_op = self.build_operator(right)?;
                let left_schema = left_op.schema().clone();
                let right_schema = right_op.schema().clone();

                // 출력 스키마 = left columns + right columns
                let mut all_fields = left_schema.fields().to_vec();
                all_fields.extend(right_schema.fields().to_vec());
                let join_schema = Arc::new(Schema::new(all_fields));

                Ok(Box::new(HashJoinOperator::new(
                    left_op, right_op, join_schema, on.clone(), *join_type,
                )))
            }

            PhysicalPlan::GridExchange { .. } => {
                // 런타임에 DistributedExecutor가 교체해야 하는 플레이스홀더
                Err(DbxError::SqlExecution {
                    message: "GridExchange placeholder must be replaced by DistributedExecutor before execution".to_string(),
                    context: "LocalExecutor::build_operator".to_string(),
                })
            }

            // DML/DDL 노드들은 LocalExecutor에서 직접 처리하지 않음 (별도 엔진 담당)
            other => Err(DbxError::SqlNotSupported {
                feature: format!("LocalExecutor: {:?} plan type", std::mem::discriminant(other)),
                hint: "DML/DDL은 StorageEngine을 통해 실행하세요".to_string(),
            }),
        }
    }
}

/// 더미 RecordBatch 생성 헬퍼 (테스트용)
pub fn make_dummy_table(rows: Vec<(i32, String, i64)>) -> (Arc<Schema>, Vec<RecordBatch>) {
    use arrow::array::{Int32Array, Int64Array};

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("value", DataType::Int64, false),
    ]));

    let ids: Vec<i32> = rows.iter().map(|(id, _, _)| *id).collect();
    let names: Vec<&str> = rows.iter().map(|(_, name, _)| name.as_str()).collect();
    let values: Vec<i64> = rows.iter().map(|(_, _, val)| *val).collect();

    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(Int32Array::from(ids)),
            Arc::new(StringArray::from(names)),
            Arc::new(Int64Array::from(values)),
        ],
    ).unwrap();

    (schema, vec![batch])
}
