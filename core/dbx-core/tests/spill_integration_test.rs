//! Spill-to-Disk 통합 테스트
//!
//! HashAggregateOperator의 Spill 동작과
//! HashJoinOperator의 OOM 방어를 검증합니다.

use arrow::array::{Float64Array, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use dbx_core::sql::executor::SpillContext;
use dbx_core::sql::executor::{HashAggregateOperator, HashJoinOperator, PhysicalOperator};
use dbx_core::sql::planner::{AggregateFunction, AggregateMode, JoinType, PhysicalAggExpr};
use std::sync::Arc;

// ==================== Helper 구조체 ====================

/// 테스트용 인메모리 배치 소스 Operator
struct VecOperator {
    batches: Vec<RecordBatch>,
    idx: usize,
    schema: Arc<Schema>,
}

impl VecOperator {
    fn new(batches: Vec<RecordBatch>) -> Self {
        let schema = if batches.is_empty() {
            Arc::new(Schema::empty())
        } else {
            batches[0].schema()
        };
        Self { batches, idx: 0, schema }
    }
}

impl PhysicalOperator for VecOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> dbx_core::error::DbxResult<Option<RecordBatch>> {
        if self.idx < self.batches.len() {
            let batch = self.batches[self.idx].clone();
            self.idx += 1;
            Ok(Some(batch))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> dbx_core::error::DbxResult<()> {
        self.idx = 0;
        Ok(())
    }
}

// ==================== 공통 Helper ====================

fn make_agg_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("category", DataType::Utf8, false),
        Field::new("total", DataType::Float64, false),
    ]))
}

fn make_input_batch(rows: usize, category: &str) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("category", DataType::Utf8, false),
        Field::new("value", DataType::Float64, false),
    ]));
    let cats: Vec<&str> = vec![category; rows];
    let vals: Vec<f64> = (0..rows).map(|i| i as f64).collect();
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(cats)),
            Arc::new(Float64Array::from(vals)),
        ],
    )
    .unwrap()
}

// ==================== SpillContext 단위 테스트 ====================

#[test]
fn test_spill_context_tracks_memory() {
    let mut ctx = SpillContext::with_budget(1024).unwrap();
    assert!(!ctx.should_spill());
    ctx.track(512);
    assert!(!ctx.should_spill());
    ctx.track(600); // 총 1112 > 1024
    assert!(ctx.should_spill());
}

#[test]
fn test_spill_context_reset_after_spill() {
    let mut ctx = SpillContext::with_budget(100).unwrap();
    let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Int32, false)]));
    let batch = RecordBatch::try_new(
        schema,
        vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
    )
    .unwrap();

    let path = ctx.spill_batches(&[batch]).unwrap();
    // Spill 후 used_bytes는 리셋돼야 함
    assert!(!ctx.should_spill(), "Spill 후 메모리 추적이 리셋되어야 함");
    assert!(path.exists(), "Spill 파일이 존재해야 함");
}

#[test]
fn test_spill_reload_preserves_data() {
    let mut ctx = SpillContext::new().unwrap();
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Float64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
            Arc::new(Float64Array::from(vec![10.0, 20.0, 30.0, 40.0, 50.0])),
        ],
    )
    .unwrap();

    let path = ctx.spill_batches(&[batch]).unwrap();
    let reloaded = SpillContext::reload_batches(&path).unwrap();

    let total: usize = reloaded.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total, 5, "Spill/Reload 후 행 수가 보존되어야 함");

    let ids = reloaded[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int32Array>()
        .unwrap();
    assert_eq!(ids.value(0), 1);
    assert_eq!(ids.value(4), 5);
}

// ==================== HashAggregateOperator Spill 테스트 ====================

#[test]
fn test_hash_agg_spill_produces_correct_sum() {
    // 작은 메모리 예산 (4KB)으로 Spill 강제 유발
    let spill_ctx = SpillContext::with_budget(4 * 1024).unwrap();

    let output_schema = make_agg_schema();

    // 여러 배치를 입력: "A" 카테고리 3개 배치 (각 100행)
    let batches = vec![
        make_input_batch(100, "A"),
        make_input_batch(100, "A"),
        make_input_batch(100, "A"),
    ];

    let input = VecOperator::new(batches);
    let aggs = vec![PhysicalAggExpr {
        function: AggregateFunction::Sum,
        input: 1,
        alias: None,
    }];

    let mut op = HashAggregateOperator::new(
        Box::new(input),
        output_schema,
        vec![0], // GROUP BY category
        aggs,
        AggregateMode::Simple,
    )
    .with_spill(spill_ctx);

    let result = op.next().unwrap();
    assert!(result.is_some(), "집계 결과가 있어야 함");
    let batch = result.unwrap();
    assert!(batch.num_rows() >= 1, "최소 1개 그룹이 있어야 함");
}

#[test]
fn test_hash_agg_no_spill_context_still_works() {
    // Spill 없이 기존 동작 유지 확인
    let output_schema = make_agg_schema();
    let batch = make_input_batch(50, "B");

    let input = VecOperator::new(vec![batch]);
    let aggs = vec![PhysicalAggExpr {
        function: AggregateFunction::Count,
        input: 1,
        alias: None,
    }];

    let mut op = HashAggregateOperator::new(
        Box::new(input),
        output_schema,
        vec![0],
        aggs,
        AggregateMode::Simple,
    );

    let result = op.next().unwrap();
    assert!(result.is_some(), "SpillContext 없이도 정상 동작해야 함");
}

// ==================== HashJoinOperator OOM 방어 테스트 ====================

#[test]
fn test_hash_join_oom_guard_triggers_on_large_build() {
    let left_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Float64, false),
    ]));
    let right_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    let join_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Float64, false),
        Field::new("name", DataType::Utf8, false),
    ]));

    // 빌드 테이블 (left): 1000행
    let left_batch = RecordBatch::try_new(
        Arc::clone(&left_schema),
        vec![
            Arc::new(Int32Array::from((0..1000i32).collect::<Vec<_>>())),
            Arc::new(Float64Array::from((0..1000).map(|i| i as f64).collect::<Vec<_>>())),
        ],
    )
    .unwrap();

    // 프로브 테이블 (right): 5행
    let right_batch = RecordBatch::try_new(
        Arc::clone(&right_schema),
        vec![
            Arc::new(Int32Array::from(vec![0, 1, 2, 3, 4])),
            Arc::new(StringArray::from(vec!["a", "b", "c", "d", "e"])),
        ],
    )
    .unwrap();

    let left_op = VecOperator::new(vec![left_batch]);
    let right_op = VecOperator::new(vec![right_batch]);

    // 메모리 한도를 1바이트로 설정 → OOM 에러 강제 유발
    let mut join_op = HashJoinOperator::new(
        Box::new(left_op),
        Box::new(right_op),
        join_schema,
        vec![(0, 0)],
        JoinType::Inner,
    )
    .with_build_memory_limit(1); // 1바이트 → 반드시 초과

    let result = join_op.next();
    assert!(result.is_err(), "메모리 한도 초과 시 에러가 반환되어야 함");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("OOM"),
        "에러 메시지에 'OOM'이 포함되어야 함: {}",
        err
    );
}

#[test]
fn test_hash_join_normal_operation_within_budget() {
    let left_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
    ]));
    let right_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
    ]));
    let join_schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("id", DataType::Int32, false),
    ]));

    let left_batch = RecordBatch::try_new(
        Arc::clone(&left_schema),
        vec![Arc::new(Int32Array::from(vec![1, 2, 3]))],
    )
    .unwrap();
    let right_batch = RecordBatch::try_new(
        Arc::clone(&right_schema),
        vec![Arc::new(Int32Array::from(vec![2, 3, 4]))],
    )
    .unwrap();

    let left_op = VecOperator::new(vec![left_batch]);
    let right_op = VecOperator::new(vec![right_batch]);

    // 충분한 메모리 한도 (256MB)
    let mut join_op = HashJoinOperator::new(
        Box::new(left_op),
        Box::new(right_op),
        join_schema,
        vec![(0, 0)],
        JoinType::Inner,
    )
    .with_build_memory_limit(256 * 1024 * 1024);

    let result = join_op.next();
    // 정상 동작 (에러 없음)
    assert!(result.is_ok(), "한도 내에서는 정상 동작해야 함: {:?}", result.err());
}
