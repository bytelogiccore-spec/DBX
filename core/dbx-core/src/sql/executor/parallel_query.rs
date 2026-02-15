//! Parallel Query Executor — Phase 2: Section 4.2
//!
//! RecordBatch 단위 병렬 처리: 스캔, 필터, 집계를 Rayon 기반으로 병렬화

use crate::error::DbxResult;
use crate::sql::planner::PhysicalExpr;
use arrow::array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, RecordBatch};
use arrow::compute;
use arrow::datatypes::Schema;
use rayon::prelude::*;
use std::sync::Arc;

/// 병렬 쿼리 실행기
///
/// 여러 RecordBatch를 Rayon work-stealing 스레드 풀로 동시에 처리합니다.
/// 총 행 수가 `min_rows_for_parallel` 미만이면 순차 실행으로 fallback합니다.
pub struct ParallelQueryExecutor {
    /// 병렬화 임계값 (이 이상의 batch 수에서 병렬 처리)
    parallel_threshold: usize,
    /// 병렬화 최소 행 수 (이 이하면 순차 실행)
    min_rows_for_parallel: usize,
    /// 사용할 스레드 풀 (None이면 글로벌)
    thread_pool: Option<Arc<rayon::ThreadPool>>,
}

impl ParallelQueryExecutor {
    /// 새 병렬 쿼리 실행기 생성
    pub fn new() -> Self {
        Self {
            parallel_threshold: 2,
            min_rows_for_parallel: 1000,
            thread_pool: None,
        }
    }

    /// 커스텀 스레드 풀 설정
    pub fn with_thread_pool(mut self, pool: Arc<rayon::ThreadPool>) -> Self {
        self.thread_pool = Some(pool);
        self
    }

    /// 병렬화 batch 수 임계값 설정
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// 병렬화 최소 행 수 설정
    pub fn with_min_rows(mut self, min_rows: usize) -> Self {
        self.min_rows_for_parallel = min_rows;
        self
    }

    /// 총 행 수가 임계값 이상인지 판단
    fn should_parallelize(&self, batches: &[RecordBatch]) -> bool {
        if batches.len() < self.parallel_threshold {
            return false;
        }
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        total_rows >= self.min_rows_for_parallel
    }

    /// 병렬 테이블 스캔 + 필터
    ///
    /// 여러 RecordBatch를 병렬로 필터링합니다.
    pub fn par_filter(
        &self,
        batches: &[RecordBatch],
        predicate: &PhysicalExpr,
    ) -> DbxResult<Vec<RecordBatch>> {
        if !self.should_parallelize(batches) {
            // Sequential fallback (소규모 데이터)
            return batches
                .iter()
                .filter_map(
                    |batch| match Self::apply_filter_to_batch(batch, predicate) {
                        Ok(Some(b)) if b.num_rows() > 0 => Some(Ok(b)),
                        Ok(_) => None,
                        Err(e) => Some(Err(e)),
                    },
                )
                .collect();
        }

        // Parallel
        let results: Vec<DbxResult<Option<RecordBatch>>> = self.run_parallel(batches, |batch| {
            Self::apply_filter_to_batch(batch, predicate)
        });

        results
            .into_iter()
            .filter_map(|r| match r {
                Ok(Some(b)) if b.num_rows() > 0 => Some(Ok(b)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect()
    }

    /// 병렬 집계 (SUM, COUNT, AVG, MIN, MAX)
    ///
    /// 각 batch를 병렬로 부분 집계 후, 최종 집계합니다.
    pub fn par_aggregate(
        &self,
        batches: &[RecordBatch],
        column_idx: usize,
        agg_type: AggregateType,
    ) -> DbxResult<AggregateResult> {
        if batches.is_empty() {
            return Ok(AggregateResult::empty(agg_type));
        }

        // 행 수 기반 순차/병렬 분기
        let partials: Vec<DbxResult<PartialAggregate>> = if self.should_parallelize(batches) {
            self.run_parallel(batches, |batch| {
                Self::partial_aggregate(batch, column_idx, agg_type)
            })
        } else {
            batches
                .iter()
                .map(|batch| Self::partial_aggregate(batch, column_idx, agg_type))
                .collect()
        };

        // Phase 2: merge partial results
        let mut merged = PartialAggregate::empty(agg_type);
        for partial in partials {
            merged.merge(&partial?);
        }

        Ok(merged.finalize())
    }

    /// 병렬 프로젝션 (컬럼 선택)
    pub fn par_project(
        &self,
        batches: &[RecordBatch],
        indices: &[usize],
    ) -> DbxResult<Vec<RecordBatch>> {
        if !self.should_parallelize(batches) {
            return batches
                .iter()
                .map(|batch| Self::project_batch(batch, indices))
                .collect();
        }

        self.run_parallel(batches, |batch| Self::project_batch(batch, indices))
            .into_iter()
            .collect()
    }

    // ─── Internal helpers ───────────────────────────────

    /// 단일 batch에 필터 적용
    fn apply_filter_to_batch(
        batch: &RecordBatch,
        predicate: &PhysicalExpr,
    ) -> DbxResult<Option<RecordBatch>> {
        if batch.num_rows() == 0 {
            return Ok(None);
        }

        let result = crate::sql::executor::evaluate_expr(predicate, batch)?;
        let mask = result
            .as_any()
            .downcast_ref::<BooleanArray>()
            .ok_or_else(|| crate::error::DbxError::TypeMismatch {
                expected: "BooleanArray".to_string(),
                actual: format!("{:?}", result.data_type()),
            })?;

        let filtered = compute::filter_record_batch(batch, mask)?;
        if filtered.num_rows() > 0 {
            Ok(Some(filtered))
        } else {
            Ok(None)
        }
    }

    /// 단일 batch에 프로젝션 적용
    fn project_batch(batch: &RecordBatch, indices: &[usize]) -> DbxResult<RecordBatch> {
        let columns: Vec<ArrayRef> = indices
            .iter()
            .map(|&idx| Arc::clone(batch.column(idx)))
            .collect();
        let fields: Vec<_> = indices
            .iter()
            .map(|&idx| batch.schema().field(idx).clone())
            .collect();
        let schema = Arc::new(Schema::new(fields));
        Ok(RecordBatch::try_new(schema, columns)?)
    }

    /// 단일 batch에 대한 부분 집계
    fn partial_aggregate(
        batch: &RecordBatch,
        column_idx: usize,
        agg_type: AggregateType,
    ) -> DbxResult<PartialAggregate> {
        let column = batch.column(column_idx);
        let mut partial = PartialAggregate::empty(agg_type);

        // Try as Int64 first, then Float64
        if let Some(arr) = column.as_any().downcast_ref::<Int64Array>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let val = arr.value(i) as f64;
                    partial.accumulate(val);
                }
            }
        } else if let Some(arr) = column.as_any().downcast_ref::<Float64Array>() {
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    partial.accumulate(arr.value(i));
                }
            }
        }

        Ok(partial)
    }

    /// Rayon 기반 병렬 실행 (스레드 풀 사용)
    fn run_parallel<T, F>(&self, batches: &[RecordBatch], op: F) -> Vec<T>
    where
        T: Send,
        F: Fn(&RecordBatch) -> T + Sync,
    {
        if let Some(pool) = &self.thread_pool {
            pool.install(|| batches.par_iter().map(&op).collect())
        } else {
            batches.par_iter().map(&op).collect()
        }
    }
}

impl Default for ParallelQueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// 집계 연산 종류
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggregateType {
    Sum,
    Count,
    Avg,
    Min,
    Max,
}

/// 부분 집계 결과 (병렬 merge 가능)
#[derive(Debug, Clone)]
pub struct PartialAggregate {
    pub agg_type: AggregateType,
    pub sum: f64,
    pub count: u64,
    pub min: f64,
    pub max: f64,
}

impl PartialAggregate {
    fn empty(agg_type: AggregateType) -> Self {
        Self {
            agg_type,
            sum: 0.0,
            count: 0,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    fn accumulate(&mut self, val: f64) {
        self.sum += val;
        self.count += 1;
        if val < self.min {
            self.min = val;
        }
        if val > self.max {
            self.max = val;
        }
    }

    fn merge(&mut self, other: &PartialAggregate) {
        self.sum += other.sum;
        self.count += other.count;
        if other.min < self.min {
            self.min = other.min;
        }
        if other.max > self.max {
            self.max = other.max;
        }
    }

    fn finalize(&self) -> AggregateResult {
        match self.agg_type {
            AggregateType::Sum => AggregateResult {
                value: self.sum,
                count: self.count,
            },
            AggregateType::Count => AggregateResult {
                value: self.count as f64,
                count: self.count,
            },
            AggregateType::Avg => {
                let avg = if self.count > 0 {
                    self.sum / self.count as f64
                } else {
                    0.0
                };
                AggregateResult {
                    value: avg,
                    count: self.count,
                }
            }
            AggregateType::Min => AggregateResult {
                value: self.min,
                count: self.count,
            },
            AggregateType::Max => AggregateResult {
                value: self.max,
                count: self.count,
            },
        }
    }
}

/// 최종 집계 결과
#[derive(Debug, Clone)]
pub struct AggregateResult {
    pub value: f64,
    pub count: u64,
}

impl AggregateResult {
    fn empty(_agg_type: AggregateType) -> Self {
        Self {
            value: 0.0,
            count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};

    fn make_test_batch(ids: &[i64], names: &[&str]) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int64Array::from(ids.to_vec())),
                Arc::new(StringArray::from(names.to_vec())),
            ],
        )
        .unwrap()
    }

    #[test]
    fn test_par_aggregate_sum() {
        let executor = ParallelQueryExecutor::new();
        let batches = vec![
            make_test_batch(&[1, 2, 3], &["a", "b", "c"]),
            make_test_batch(&[4, 5, 6], &["d", "e", "f"]),
            make_test_batch(&[7, 8, 9], &["g", "h", "i"]),
        ];

        let result = executor
            .par_aggregate(&batches, 0, AggregateType::Sum)
            .unwrap();
        assert_eq!(result.value, 45.0); // 1+2+...+9
        assert_eq!(result.count, 9);
    }

    #[test]
    fn test_par_aggregate_avg() {
        let executor = ParallelQueryExecutor::new();
        let batches = vec![
            make_test_batch(&[10, 20], &["a", "b"]),
            make_test_batch(&[30, 40], &["c", "d"]),
        ];

        let result = executor
            .par_aggregate(&batches, 0, AggregateType::Avg)
            .unwrap();
        assert_eq!(result.value, 25.0);
    }

    #[test]
    fn test_par_aggregate_min_max() {
        let executor = ParallelQueryExecutor::new();
        let batches = vec![
            make_test_batch(&[5, 1, 8], &["a", "b", "c"]),
            make_test_batch(&[3, 9, 2], &["d", "e", "f"]),
        ];

        let min_result = executor
            .par_aggregate(&batches, 0, AggregateType::Min)
            .unwrap();
        assert_eq!(min_result.value, 1.0);

        let max_result = executor
            .par_aggregate(&batches, 0, AggregateType::Max)
            .unwrap();
        assert_eq!(max_result.value, 9.0);
    }

    #[test]
    fn test_par_project() {
        let executor = ParallelQueryExecutor::new();
        let batches = vec![
            make_test_batch(&[1, 2], &["a", "b"]),
            make_test_batch(&[3, 4], &["c", "d"]),
            make_test_batch(&[5, 6], &["e", "f"]),
        ];

        let projected = executor.par_project(&batches, &[0]).unwrap();
        assert_eq!(projected.len(), 3);
        assert_eq!(projected[0].num_columns(), 1);
        assert_eq!(projected[0].schema().field(0).name(), "id");
    }

    #[test]
    fn test_par_aggregate_empty() {
        let executor = ParallelQueryExecutor::new();
        let batches: Vec<RecordBatch> = vec![];

        let result = executor
            .par_aggregate(&batches, 0, AggregateType::Count)
            .unwrap();
        assert_eq!(result.count, 0);
    }
}
