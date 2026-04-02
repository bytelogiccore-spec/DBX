//! LocalExecutor — 단일 노드 PhysicalPlan 실행기
//!
//! PhysicalPlan 트리를 순회하며 PhysicalOperator 트리를 빌드하고 실행합니다.
//! DistributedExecutor의 워커 측 실행과 단독 쿼리 실행 모두에서 재사용됩니다.

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::{
    FilterOperator, HashAggregateOperator, HashJoinOperator, LimitOperator, PhysicalOperator,
    ProjectionOperator, SortOperator, TableScanOperator,
};
use crate::sql::executor::operators::{GridExchangeOperator, GridShuffleWriterOperator};
use crate::sql::planner::types::*;
use arrow::array::{RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

/// 분산 실행 시 동적으로 생성된 IPC 채널을 보관하는 컨테이너
#[derive(Default)]
pub struct DistributedChannels {
    /// GridExchange가 데이터를 수신하기 위해 사용하는 Sender 보관 (key: exchange_id)
    /// GridManager가 이 Sender를 가져가서 네트워크 인커밍 데이터를 밀어넣습니다.
    pub exchanges: HashMap<usize, mpsc::Sender<DbxResult<Option<Vec<u8>>>>>,
    /// ShuffleWriter가 데이터를 송출하기 위해 사용하는 Receiver 보관 (key: exchange_id)
    /// GridManager가 이 Receiver 배열(타겟 워커 당 1개)을 가져가서 QUIC으로 전송합니다.
    pub shuffles: HashMap<
        usize,
        Vec<(
            std::net::SocketAddr,
            mpsc::Receiver<DbxResult<Option<Vec<u8>>>>,
        )>,
    >,
}

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
        Self {
            table_store,
            table_schemas,
        }
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

    /// 분산 채널이 연동된 PhysicalPlan을 실행하고 결과를 반환합니다.
    pub fn execute_collect_distributed(
        &self,
        plan: &PhysicalPlan,
        channels: &mut DistributedChannels,
    ) -> DbxResult<Vec<RecordBatch>> {
        let mut operator = self.build_operator_distributed(plan, channels)?;
        let mut results = Vec::new();
        while let Some(batch) = operator.next()? {
            if batch.num_rows() > 0 {
                results.push(batch);
            }
        }
        Ok(results)
    }

    /// PhysicalPlan → Box<dyn PhysicalOperator> 트리 빌드 (단독 실행용)
    pub fn build_operator(&self, plan: &PhysicalPlan) -> DbxResult<Box<dyn PhysicalOperator>> {
        self.build_operator_internal(plan, &mut None)
    }

    /// 분산 채널이 연동된 PhysicalPlan 빌드
    pub fn build_operator_distributed(
        &self,
        plan: &PhysicalPlan,
        channels: &mut DistributedChannels,
    ) -> DbxResult<Box<dyn PhysicalOperator>> {
        self.build_operator_internal(plan, &mut Some(channels))
    }

    /// 내부 재귀용 빌드 메서드
    fn build_operator_internal(
        &self,
        plan: &PhysicalPlan,
        channels: &mut Option<&mut DistributedChannels>,
    ) -> DbxResult<Box<dyn PhysicalOperator>> {
        match plan {
            PhysicalPlan::TableScan {
                table,
                projection,
                filter,
                ros_files,
            } => {
                let store = self.table_store.read().unwrap();
                let wos_batches = store.get(table).cloned().unwrap_or_default();

                let schemas = self.table_schemas.read().unwrap();
                let schema = schemas
                    .get(table)
                    .cloned()
                    .ok_or_else(|| DbxError::TableNotFound(table.clone()))?;
                drop(schemas);
                drop(store);

                let mut op = TableScanOperator::new(table.clone(), schema, projection.clone());

                // 디스크 연동이 필요한 경우 start_tier_scan을 호출
                if ros_files.is_empty() {
                    op.set_data(wos_batches); // 호환: WOS만 있는 경우
                } else {
                    op.start_tier_scan(wos_batches, ros_files.clone());
                }

                // 필터가 있으면 FilterOperator로 래핑
                if let Some(f) = filter {
                    Ok(Box::new(FilterOperator::new(Box::new(op), f.clone())))
                } else {
                    Ok(Box::new(op))
                }
            }

            PhysicalPlan::HashAggregate {
                input,
                group_by,
                aggregates,
                mode,
            } => {
                let input_op = self.build_operator_internal(input, channels)?;
                let input_schema = input_op.schema().clone();

                // 출력 스키마 구성
                let mut output_fields = Vec::new();
                for &col_idx in group_by.iter() {
                    if col_idx < input_schema.fields().len() {
                        output_fields.push(input_schema.field(col_idx).clone());
                    }
                }
                for agg in aggregates {
                    let name = agg
                        .alias
                        .clone()
                        .unwrap_or_else(|| format!("agg_{}", agg.input));
                    let dtype = match agg.function {
                        AggregateFunction::Count => DataType::Int64,
                        AggregateFunction::Sum
                        | AggregateFunction::Avg
                        | AggregateFunction::Min
                        | AggregateFunction::Max => DataType::Float64,
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

            PhysicalPlan::Projection {
                input,
                exprs,
                aliases,
            } => {
                let input_op = self.build_operator_internal(input, channels)?;
                let input_schema = input_op.schema().clone();

                let output_fields: Vec<Field> = exprs
                    .iter()
                    .zip(aliases.iter())
                    .map(|(expr, alias)| {
                        let dtype = expr.get_type(&input_schema);
                        let name = alias.clone().unwrap_or_else(|| "col".to_string());
                        Field::new(&name, dtype, true)
                    })
                    .collect();
                let output_schema = Arc::new(Schema::new(output_fields));

                Ok(Box::new(ProjectionOperator::new(
                    input_op,
                    output_schema,
                    exprs.clone(),
                )))
            }

            PhysicalPlan::Limit {
                input,
                count,
                offset,
            } => {
                let input_op = self.build_operator_internal(input, channels)?;
                Ok(Box::new(LimitOperator::new(input_op, *count, *offset)))
            }

            PhysicalPlan::SortMerge { input, order_by } => {
                let input_op = self.build_operator_internal(input, channels)?;
                Ok(Box::new(SortOperator::new(input_op, order_by.clone())))
            }

            PhysicalPlan::HashJoin {
                left,
                right,
                on,
                join_type,
            } => {
                let left_op = self.build_operator_internal(left, channels)?;
                let right_op = self.build_operator_internal(right, channels)?;
                let left_schema = left_op.schema().clone();
                let right_schema = right_op.schema().clone();

                // 출력 스키마 = left columns + right columns
                let mut all_fields = left_schema.fields().to_vec();
                all_fields.extend(right_schema.fields().to_vec());
                let join_schema = Arc::new(Schema::new(all_fields));

                Ok(Box::new(HashJoinOperator::new(
                    left_op,
                    right_op,
                    join_schema,
                    on.clone(),
                    *join_type,
                )))
            }

            PhysicalPlan::GridExchange {
                exchange_id,
                schema_hint,
            } => {
                if let Some(ch) = channels.as_mut() {
                    // 수신용 채널 프로비저닝: GridExchangeOperator엔 rx, GridManager엔 tx
                    let (tx, rx) = mpsc::channel(64);
                    ch.exchanges.insert(*exchange_id, tx);

                    // Dummy 스키마 생성 (schema_hint 개수만큼 필드 생성)
                    let mut fields = Vec::with_capacity(*schema_hint);
                    for i in 0..*schema_hint {
                        fields.push(Field::new(format!("col_{}", i), DataType::Float64, true));
                    }
                    let schema = Arc::new(Schema::new(fields));

                    Ok(Box::new(GridExchangeOperator::new(schema, rx)))
                } else {
                    Err(DbxError::SqlExecution {
                        message: "GridExchange encountered but no DistributedChannels provided"
                            .to_string(),
                        context: "LocalExecutor::build_operator_internal".to_string(),
                    })
                }
            }

            PhysicalPlan::ShuffleWriter {
                input,
                hash_params,
                target_nodes,
                exchange_id,
                salting,
            } => {
                if let Some(ch) = channels.as_mut() {
                    let input_op = self.build_operator_internal(input, &mut Some(*ch))?;

                    let mut senders = Vec::new();
                    let mut receivers = Vec::new();

                    // 타겟 워커 개수만큼 송신 채널 프로비저닝
                    for target_addr in target_nodes {
                        let (tx, rx) = mpsc::channel(64);
                        senders.push(tx);
                        receivers.push((*target_addr, rx));
                    }

                    ch.shuffles.insert(*exchange_id, receivers);
                    Ok(Box::new(GridShuffleWriterOperator::new(
                        input_op,
                        hash_params.clone(),
                        *exchange_id,
                        salting.clone(),
                        senders,
                    )))
                } else {
                    Err(DbxError::SqlExecution {
                        message: "ShuffleWriter encountered but no DistributedChannels provided"
                            .to_string(),
                        context: "LocalExecutor::build_operator_internal".to_string(),
                    })
                }
            }

            // DML/DDL 노드들은 LocalExecutor에서 직접 처리하지 않음 (별도 엔진 담당)
            other => Err(DbxError::SqlNotSupported {
                feature: format!(
                    "LocalExecutor: {:?} plan type",
                    std::mem::discriminant(other)
                ),
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
    )
    .unwrap();

    (schema, vec![batch])
}
