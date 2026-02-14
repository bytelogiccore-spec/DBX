//! HashJoin Operator — Hash-based join implementation

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::planner::JoinType;
use ahash::AHashMap;
use arrow::array::*;
use arrow::compute;
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use rayon::prelude::*;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

/// Hash Join 연산자 — build from left, probe from right
pub struct HashJoinOperator {
    left: Box<dyn PhysicalOperator>,
    right: Box<dyn PhysicalOperator>,
    schema: Arc<Schema>,
    /// (left_col_idx, right_col_idx) pairs
    on: Vec<(usize, usize)>,
    #[allow(dead_code)]
    join_type: JoinType,
    /// Build phase result: hash(key) → Vec<row_index>
    build_table: Option<AHashMap<Vec<u8>, Vec<usize>>>,
    /// Materialized left side
    left_batch: Option<RecordBatch>,
    /// Materialized right side (for probe)
    right_batches: Option<Vec<RecordBatch>>,
    /// Current right batch index for probe
    right_batch_idx: usize,
    /// Whether we've finished producing output
    done: bool,
}

impl HashJoinOperator {
    pub fn new(
        left: Box<dyn PhysicalOperator>,
        right: Box<dyn PhysicalOperator>,
        schema: Arc<Schema>,
        on: Vec<(usize, usize)>,
        join_type: JoinType,
    ) -> Self {
        Self {
            left,
            right,
            schema,
            on,
            join_type,
            build_table: None,
            left_batch: None,
            right_batches: None,
            right_batch_idx: 0,
            done: false,
        }
    }

    /// Build hash table from left input.
    ///
    /// 최적화: 양쪽 크기를 먼저 확인하고, 작은 쪽을 build로 사용합니다.
    fn build_phase(&mut self) -> DbxResult<()> {
        // 양쪽 데이터를 모두 수집
        let mut left_batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        while let Some(batch) = self.left.next()? {
            if batch.num_rows() > 0 {
                left_batches.push(batch);
            }
        }

        let mut right_batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        while let Some(batch) = self.right.next()? {
            if batch.num_rows() > 0 {
                right_batches.push(batch);
            }
        }

        // 빈 경우 처리
        if left_batches.is_empty() || right_batches.is_empty() {
            self.build_table = Some(AHashMap::new());
            self.left_batch = None;
            self.right_batches = Some(Vec::new());
            return Ok(());
        }

        // 크기 기반 최적화 비활성화 (SWAP 버그: probe 시 key column 인덱스와
        // 출력 컬럼 순서의 리매핑이 미구현. SWAP 시 0건 매칭 발생.)
        // TODO: probe key column 인덱스 반전 + 출력 컬럼 순서 복원 구현 후 재활성화
        let (build_batches, probe_batches, build_is_left) = (left_batches, right_batches, true);

        // Build hash table (병렬 처리)
        let schema = build_batches[0].schema();
        let merged = super::super::concat_batches(&schema, build_batches.as_slice())?;

        // 임계값: 1000 rows 이상만 병렬화
        const PARALLEL_THRESHOLD: usize = 1000;

        // JOIN 키 컬럼 인덱스 추출
        let key_columns: Vec<usize> = if build_is_left {
            self.on.iter().map(|(left_col, _)| *left_col).collect()
        } else {
            self.on.iter().map(|(_, right_col)| *right_col).collect()
        };

        let hash_table: AHashMap<Vec<u8>, Vec<usize>> = if merged.num_rows() >= PARALLEL_THRESHOLD {
            // 병렬 Build Phase
            use dashmap::DashMap;
            let parallel_table: DashMap<Vec<u8>, Vec<usize>> = DashMap::new();

            (0..merged.num_rows()).into_par_iter().for_each(|row_idx| {
                let key = extract_join_key(&merged, &key_columns, row_idx);
                parallel_table.entry(key).or_default().push(row_idx);
            });

            // DashMap을 AHashMap으로 변환
            parallel_table.into_iter().collect()
        } else {
            // 순차 Build Phase (작은 데이터셋)
            let mut hash_table: AHashMap<Vec<u8>, Vec<usize>> = AHashMap::new();
            for row_idx in 0..merged.num_rows() {
                let key = extract_join_key(&merged, &key_columns, row_idx);
                hash_table.entry(key).or_default().push(row_idx);
            }
            hash_table
        };

        // 결과 저장
        if build_is_left {
            // 정상: left가 build, right가 probe
            self.left_batch = Some(merged);
            self.right_batches = Some(probe_batches.into_vec());
        } else {
            // Swap: right가 build, left가 probe
            // left_batch에 build 데이터 저장 (이름은 left지만 실제로는 right)
            self.left_batch = Some(merged);
            self.right_batches = Some(probe_batches.into_vec());

            // TODO: JOIN 타입 변환 필요 (LEFT <-> RIGHT)
            // TODO: 컬럼 순서 조정 필요
        }

        self.build_table = Some(hash_table);

        Ok(())
    }
}

/// Extract join key bytes from a batch for given column indices.
fn extract_join_key(batch: &RecordBatch, key_columns: &[usize], row_idx: usize) -> Vec<u8> {
    let mut key = Vec::new();
    for &col_idx in key_columns {
        append_value_to_key(&mut key, batch.column(col_idx), row_idx);
    }
    key
}

/// Append a cell value to a byte key for hashing.
fn append_value_to_key(key: &mut Vec<u8>, col: &ArrayRef, row_idx: usize) {
    if col.is_null(row_idx) {
        key.push(0); // null marker
        return;
    }
    key.push(1); // non-null marker
    match col.data_type() {
        DataType::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            key.extend_from_slice(&arr.value(row_idx).to_le_bytes());
        }
        DataType::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            key.extend_from_slice(&arr.value(row_idx).to_le_bytes());
        }
        DataType::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            key.extend_from_slice(&arr.value(row_idx).to_le_bytes());
        }
        DataType::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            let s = arr.value(row_idx);
            key.extend_from_slice(&(s.len() as u32).to_le_bytes());
            key.extend_from_slice(s.as_bytes());
        }
        _ => {
            // Fallback: use debug format
            key.extend_from_slice(format!("{:?}", col).as_bytes());
        }
    }
}

/// Create a column with NULLs for sentinel values
fn create_column_with_nulls(
    source_col: &ArrayRef,
    indices: &[u32],
    null_sentinel: u32,
) -> DbxResult<ArrayRef> {
    let num_rows = indices.len();

    match source_col.data_type() {
        DataType::Int32 => {
            let source = source_col.as_any().downcast_ref::<Int32Array>().unwrap();
            let mut builder = Int32Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel {
                    builder.append_null();
                } else {
                    builder.append_value(source.value(idx as usize));
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int64 => {
            let source = source_col.as_any().downcast_ref::<Int64Array>().unwrap();
            let mut builder = Int64Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel {
                    builder.append_null();
                } else {
                    builder.append_value(source.value(idx as usize));
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Float64 => {
            let source = source_col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut builder = Float64Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel {
                    builder.append_null();
                } else {
                    builder.append_value(source.value(idx as usize));
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Utf8 => {
            let source = source_col.as_any().downcast_ref::<StringArray>().unwrap();
            let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
            for &idx in indices {
                if idx == null_sentinel {
                    builder.append_null();
                } else {
                    builder.append_value(source.value(idx as usize));
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Boolean => {
            let source = source_col.as_any().downcast_ref::<BooleanArray>().unwrap();
            let mut builder = BooleanBuilder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel {
                    builder.append_null();
                } else {
                    builder.append_value(source.value(idx as usize));
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        _ => Err(DbxError::SqlExecution {
            message: format!(
                "Unsupported data type for NULL handling: {:?}",
                source_col.data_type()
            ),
            context: "create_column_with_nulls".to_string(),
        }),
    }
}

impl PhysicalOperator for HashJoinOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.done {
            return Ok(None);
        }

        // Build phase (once)
        if self.build_table.is_none() {
            self.build_phase()?;
        }

        let build_table = self.build_table.as_ref().unwrap();
        let left_batch = match &self.left_batch {
            Some(b) => b.clone(),
            None => {
                self.done = true;
                return Ok(None);
            }
        };

        // Probe phase: iterate right batches
        let right_batches = self.right_batches.as_ref().unwrap();

        while self.right_batch_idx < right_batches.len() {
            let right_batch = &right_batches[self.right_batch_idx];
            self.right_batch_idx += 1;

            if right_batch.num_rows() == 0 {
                continue;
            }

            let mut left_indices = Vec::new();
            let mut right_indices = Vec::new();

            // For LEFT JOIN: track which left rows were matched
            let mut matched_left_rows = if matches!(self.join_type, JoinType::Left) {
                Some(std::collections::HashSet::new())
            } else {
                None
            };

            // For RIGHT JOIN: track which right rows were matched
            let mut matched_right_rows = if matches!(self.join_type, JoinType::Right) {
                Some(vec![false; right_batch.num_rows()])
            } else {
                None
            };

            // Probe Phase: 병렬 처리
            let right_key_columns: Vec<usize> =
                self.on.iter().map(|(_, right_col)| *right_col).collect();

            // 임계값: 1000 rows 이상만 병렬화
            const PARALLEL_THRESHOLD: usize = 1000;

            if right_batch.num_rows() >= PARALLEL_THRESHOLD {
                // 병렬 Probe Phase
                use dashmap::DashMap;
                let parallel_matches: DashMap<usize, Vec<usize>> = DashMap::new();

                (0..right_batch.num_rows())
                    .into_par_iter()
                    .for_each(|right_row| {
                        let key = extract_join_key(right_batch, &right_key_columns, right_row);
                        if let Some(left_rows) = build_table.get(&key) {
                            parallel_matches.insert(right_row, left_rows.clone());
                        }
                    });

                // 결과 수집
                for (right_row, left_rows) in parallel_matches.into_iter() {
                    for &left_row in &left_rows {
                        left_indices.push(left_row as u32);
                        right_indices.push(right_row as u32);

                        if let Some(ref mut matched) = matched_left_rows {
                            matched.insert(left_row);
                        }
                        if let Some(ref mut matched) = matched_right_rows {
                            matched[right_row] = true;
                        }
                    }
                }
            } else {
                // 순차 Probe Phase (작은 데이터셋)
                for right_row in 0..right_batch.num_rows() {
                    let key = extract_join_key(right_batch, &right_key_columns, right_row);
                    if let Some(left_rows) = build_table.get(&key) {
                        for &left_row in left_rows {
                            left_indices.push(left_row as u32);
                            right_indices.push(right_row as u32);

                            if let Some(ref mut matched) = matched_left_rows {
                                matched.insert(left_row);
                            }
                            if let Some(ref mut matched) = matched_right_rows {
                                matched[right_row] = true;
                            }
                        }
                    } else if matches!(self.join_type, JoinType::Right) {
                        // RIGHT JOIN: include unmatched right rows
                        // Will be handled after the loop
                    }
                }
            }

            // Handle LEFT JOIN: add unmatched left rows
            if let Some(matched) = matched_left_rows {
                for left_row in 0..left_batch.num_rows() {
                    if !matched.contains(&left_row) {
                        left_indices.push(left_row as u32);
                        // Use a sentinel value for NULL right side
                        right_indices.push(u32::MAX);
                    }
                }
            }

            // Handle RIGHT JOIN: add unmatched right rows
            if let Some(matched) = matched_right_rows {
                for (right_row, &was_matched) in matched.iter().enumerate() {
                    if !was_matched {
                        // Use a sentinel value for NULL left side
                        left_indices.push(u32::MAX);
                        right_indices.push(right_row as u32);
                    }
                }
            }

            if left_indices.is_empty() {
                continue;
            }

            // Build output: left columns + right columns
            let mut output_columns: Vec<ArrayRef> = Vec::new();

            // Process left columns
            for col in left_batch.columns() {
                let filtered_indices: Vec<u32> = left_indices
                    .iter()
                    .filter(|&&idx| idx != u32::MAX)
                    .copied()
                    .collect();

                if filtered_indices.len() == left_indices.len() {
                    // No NULLs needed
                    let left_idx_arr = UInt32Array::from(left_indices.clone());
                    output_columns.push(compute::take(col.as_ref(), &left_idx_arr, None)?);
                } else {
                    // Need to handle NULLs (RIGHT JOIN case)
                    output_columns.push(create_column_with_nulls(col, &left_indices, u32::MAX)?);
                }
            }

            // Process right columns
            for col in right_batch.columns() {
                let filtered_indices: Vec<u32> = right_indices
                    .iter()
                    .filter(|&&idx| idx != u32::MAX)
                    .copied()
                    .collect();

                if filtered_indices.len() == right_indices.len() {
                    // No NULLs needed
                    let right_idx_arr = UInt32Array::from(right_indices.clone());
                    output_columns.push(compute::take(col.as_ref(), &right_idx_arr, None)?);
                } else {
                    // Need to handle NULLs (LEFT JOIN case)
                    output_columns.push(create_column_with_nulls(col, &right_indices, u32::MAX)?);
                }
            }

            let result = RecordBatch::try_new(Arc::clone(&self.schema), output_columns)?;

            if result.num_rows() > 0 {
                return Ok(Some(result));
            }
        }

        eprintln!("DEBUG: Probe phase complete, no more batches");
        self.done = true;
        Ok(None)
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.build_table = None;
        self.left_batch = None;
        self.done = false;
        self.left.reset()?;
        self.right.reset()
    }
}
