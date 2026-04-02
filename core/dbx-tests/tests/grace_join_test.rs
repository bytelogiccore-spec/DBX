use arrow::array::*;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use dbx_core::error::DbxResult;
use dbx_core::sql::executor::operators::HashJoinOperator;
use dbx_core::sql::executor::operators::PhysicalOperator;
use dbx_core::sql::executor::spill::SpillContext;
use dbx_core::sql::planner::JoinType;
use std::sync::Arc;

/// Simple mock operator that returns a list of batches.
struct VecOperator {
    schema: Arc<Schema>,
    batches: Vec<RecordBatch>,
    idx: usize,
}

impl VecOperator {
    fn new(schema: Arc<Schema>, batches: Vec<RecordBatch>) -> Self {
        Self {
            schema,
            batches,
            idx: 0,
        }
    }
}

impl PhysicalOperator for VecOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }
    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.idx < self.batches.len() {
            let res = self.batches[self.idx].clone();
            self.idx += 1;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
    fn reset(&mut self) -> DbxResult<()> {
        self.idx = 0;
        Ok(())
    }
}

fn create_batch(schema: Arc<Schema>, start: i32, end: i32) -> RecordBatch {
    let ids = Arc::new(Int32Array::from((start..end).collect::<Vec<_>>()));
    let vals = Arc::new(Int32Array::from(
        (start..end).map(|i| i * 10).collect::<Vec<_>>(),
    ));
    RecordBatch::try_new(schema, vec![ids, vals]).unwrap()
}

#[test]
fn test_grace_join_spill() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Int32, false),
    ]));

    let left_batches = vec![
        create_batch(Arc::clone(&schema), 0, 1000),
        create_batch(Arc::clone(&schema), 1000, 2000),
    ];
    let right_batches = vec![create_batch(Arc::clone(&schema), 500, 1500)];

    let left = Box::new(VecOperator::new(Arc::clone(&schema), left_batches));
    let right = Box::new(VecOperator::new(Arc::clone(&schema), right_batches));

    let join_schema = Arc::new(Schema::new(vec![
        Field::new("id_l", DataType::Int32, false),
        Field::new("val_l", DataType::Int32, false),
        Field::new("id_r", DataType::Int32, false),
        Field::new("val_r", DataType::Int32, false),
    ]));

    let mut op = HashJoinOperator::new(left, right, join_schema, vec![(0, 0)], JoinType::Inner);

    let spill_ctx = SpillContext::with_budget(10 * 1024).unwrap();
    op = op.with_spill(spill_ctx).with_build_memory_limit(10 * 1024);

    let mut total_rows = 0;
    while let Ok(Some(batch)) = op.next() {
        total_rows += batch.num_rows();
    }

    assert_eq!(total_rows, 1000);
}

#[test]
fn test_grace_join_no_matches() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Int32, false),
    ]));

    let left_batches = vec![create_batch(Arc::clone(&schema), 0, 100)];
    let right_batches = vec![create_batch(Arc::clone(&schema), 200, 300)];

    let left = Box::new(VecOperator::new(Arc::clone(&schema), left_batches));
    let right = Box::new(VecOperator::new(Arc::clone(&schema), right_batches));

    let join_schema = Arc::new(Schema::new(vec![
        Field::new("id_l", DataType::Int32, false),
        Field::new("val_l", DataType::Int32, false),
        Field::new("id_r", DataType::Int32, false),
        Field::new("val_r", DataType::Int32, false),
    ]));

    let mut op = HashJoinOperator::new(left, right, join_schema, vec![(0, 0)], JoinType::Inner);

    let spill_ctx = SpillContext::with_budget(1024).unwrap();
    op = op.with_spill(spill_ctx).with_build_memory_limit(1024);

    let mut total_rows = 0;
    while let Ok(Some(batch)) = op.next() {
        total_rows += batch.num_rows();
    }

    assert_eq!(total_rows, 0);
}

#[test]
fn test_recursive_partitioning() {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("val", DataType::Int32, false),
    ]));

    // 모든 ID가 id % 32 == 0이 되도록 하여 Partition 0으로 몰아넣음
    let left_ids = Arc::new(Int32Array::from(
        (0..100).map(|i| i * 32).collect::<Vec<_>>(),
    ));
    let left_vals = Arc::new(Int32Array::from(vec![1; 100]));
    let left_batch = RecordBatch::try_new(Arc::clone(&schema), vec![left_ids, left_vals]).unwrap();

    let right_ids = Arc::new(Int32Array::from(
        (0..50).map(|i| i * 32).collect::<Vec<_>>(),
    ));
    let right_vals = Arc::new(Int32Array::from(vec![2; 50]));
    let right_batch =
        RecordBatch::try_new(Arc::clone(&schema), vec![right_ids, right_vals]).unwrap();

    let left = Box::new(VecOperator::new(Arc::clone(&schema), vec![left_batch]));
    let right = Box::new(VecOperator::new(Arc::clone(&schema), vec![right_batch]));

    let join_schema = Arc::new(Schema::new(vec![
        Field::new("id_l", DataType::Int32, false),
        Field::new("val_l", DataType::Int32, false),
        Field::new("id_r", DataType::Int32, false),
        Field::new("val_r", DataType::Int32, false),
    ]));

    let mut op = HashJoinOperator::new(left, right, join_schema, vec![(0, 0)], JoinType::Inner);

    // 극도로 작은 예산 (1KB) 설정 -> 100개 로우는 약 1.6KB 이상이므로 한 파티션에 못 들어감 -> 재귀 분할 발생
    let spill_ctx = SpillContext::with_budget(1024).unwrap();
    op = op.with_spill(spill_ctx).with_build_memory_limit(1024);

    let mut total_rows = 0;
    while let Ok(Some(batch)) = op.next() {
        total_rows += batch.num_rows();
    }

    // 0, 32, ..., (49*32) 까지 50개 매칭되어야 함
    assert_eq!(total_rows, 50);
}
