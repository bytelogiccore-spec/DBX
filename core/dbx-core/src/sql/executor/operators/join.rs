//! HashJoin Operator — Hash-based join implementation

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::executor::spill::SpillContext;
use crate::sql::planner::JoinType;
use ahash::AHashMap;
use arrow::array::*;
use arrow::compute;
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
// use rayon::prelude::*; // Not used currently in sequential probe
use smallvec::{SmallVec, smallvec};
use std::path::PathBuf;
use std::sync::Arc;

/// Hash Join 실행 상태
#[derive(Debug, Clone, PartialEq)]
enum JoinState {
    InMemory,
    Partitioning,
    JoiningPartitions {
        current_partition: usize,
    },
}

pub struct HashJoinOperator {
    left: Box<dyn PhysicalOperator>,
    right: Box<dyn PhysicalOperator>,
    schema: Arc<Schema>,
    on: Vec<(usize, usize)>,
    join_type: JoinType,
    build_table: Option<AHashMap<Vec<u8>, Vec<usize>>>,
    left_batch: Option<RecordBatch>,
    right_batches: Option<Vec<RecordBatch>>,
    right_batch_idx: usize,
    done: bool,
    build_memory_budget: usize,
    state: JoinState,
    spill_ctx: Option<SpillContext>,
    /// 파티션 수 (Grace Hash Join)
    num_partitions: usize,
    /// 왼쪽 파티션 파일 경로 목록: [partition_idx][file_paths]
    left_partitions: Vec<Vec<PathBuf>>,
    /// 오른쪽 파티션 파일 경로 목록: [partition_idx][file_paths]
    right_partitions: Vec<Vec<PathBuf>>,
    /// Hybrid Hash Join: Partition 0을 메모리에 유지
    left_partition0: Vec<RecordBatch>,
    /// Hybrid Hash Join: Partition 0의 오른쪽 데이터 메모리에 유지
    right_partition0: Vec<RecordBatch>,
    /// Partition 0의 현재 메모리 사용량 추적
    partition0_mem_used: usize,
    /// 재귀적 파티셔닝 깊이 추적: [partition_idx]
    partition_depths: Vec<usize>,
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
            build_memory_budget: 256 * 1024 * 1024,
            state: JoinState::InMemory,
            spill_ctx: None,
            num_partitions: 32,
            left_partitions: (0..32).map(|_| Vec::new()).collect(),
            right_partitions: (0..32).map(|_| Vec::new()).collect(),
            left_partition0: Vec::new(),
            right_partition0: Vec::new(),
            partition0_mem_used: 0,
            partition_depths: vec![0; 32],
        }
    }

    pub fn with_spill(mut self, ctx: SpillContext) -> Self {
        self.spill_ctx = Some(ctx);
        self
    }

    pub fn with_build_memory_limit(mut self, bytes: usize) -> Self {
        self.build_memory_budget = bytes;
        self
    }

    fn partition_batch(
        &self,
        batch: &RecordBatch,
        key_indices: &[usize],
        seed: u64,
    ) -> DbxResult<Vec<Option<RecordBatch>>> {
        let num_partitions = self.num_partitions;
        let mut row_indices: Vec<Vec<u32>> = (0..num_partitions).map(|_| Vec::new()).collect();

        for row_idx in 0..batch.num_rows() {
            let key = extract_join_key(batch, key_indices, row_idx);
            let hash = ahash::RandomState::with_seeds(seed, seed, seed, seed).hash_one(&key);
            let part_idx = (hash % num_partitions as u64) as usize;
            row_indices[part_idx].push(row_idx as u32);
        }

        let mut partitions = Vec::with_capacity(num_partitions);
        for indices in row_indices {
            if indices.is_empty() {
                partitions.push(None);
            } else {
                let idx_array = UInt32Array::from(indices);
                let columns = batch
                    .columns()
                    .iter()
                    .map(|col| compute::take(col.as_ref(), &idx_array, None))
                    .collect::<Result<Vec<_>, _>>()?;
                let partitioned_batch = RecordBatch::try_new(batch.schema(), columns)?;
                partitions.push(Some(partitioned_batch));
            }
        }
        Ok(partitions)
    }

    fn handle_partitioning(
        &mut self,
        batch: RecordBatch,
        key_indices: &[usize],
        is_left: bool,
        depth: usize,
        base_idx: usize,
    ) -> DbxResult<()> {
        let num_partitions = self.num_partitions;
        let parts = self.partition_batch(&batch, key_indices, depth as u64)?;
        
        for (sub_idx, part) in parts.into_iter().enumerate() {
            if let Some(p) = part {
                let target_idx = base_idx + sub_idx;
                let p_bytes = SpillContext::estimate_batch_bytes(&p);
                
                // Hybrid: Level 0의 Partition 0만 메모리에 유지
                if depth == 0 && target_idx == 0 && self.partition0_mem_used + p_bytes < (self.build_memory_budget * 7 / 10) {
                    if is_left {
                        self.left_partition0.push(p);
                    } else {
                        self.right_partition0.push(p);
                    }
                    self.partition0_mem_used += p_bytes;
                } else {
                    let spill_ctx = self.spill_ctx.as_mut().unwrap();
                    let prefix = if is_left { "left" } else { "right" };
                    
                    // P0 메모리 초과 시 디스크로 스필 (Grace 모드 폴백)
                    if depth == 0 && target_idx == 0 && (if is_left { !self.left_partition0.is_empty() } else { !self.right_partition0.is_empty() }) {
                        let to_spill = if is_left {
                            std::mem::take(&mut self.left_partition0)
                        } else {
                            std::mem::take(&mut self.right_partition0)
                        };
                        for p0_batch in to_spill {
                            let path = spill_ctx.spill_partition_batch(prefix, 0, p0_batch)?;
                            if is_left { self.left_partitions[0].push(path); }
                            else { self.right_partitions[0].push(path); }
                        }
                    }

                    let path = spill_ctx.spill_partition_batch(prefix, target_idx, p)?;
                    if is_left {
                        self.left_partitions[target_idx].push(path);
                    } else {
                        self.right_partitions[target_idx].push(path);
                    }
                }
            }
        }
        Ok(())
    }

    fn build_phase(&mut self) -> DbxResult<()> {
        let mut left_batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        let mut left_bytes = 0usize;

        while let Some(batch) = self.left.next()? {
            if batch.num_rows() == 0 { continue; }
            let batch_bytes = SpillContext::estimate_batch_bytes(&batch);
            left_bytes += batch_bytes;
            left_batches.push(batch);

            if left_bytes > self.build_memory_budget && self.spill_ctx.is_some() {
                self.state = JoinState::Partitioning;
                break;
            }
        }

        if matches!(self.state, JoinState::Partitioning) {
            let left_indices: Vec<usize> = self.on.iter().map(|(l, _)| *l).collect();
            let right_indices: Vec<usize> = self.on.iter().map(|(_, r)| *r).collect();

            for batch in left_batches {
                self.handle_partitioning(batch, &left_indices, true, 0, 0)?;
            }

            while let Some(batch) = self.left.next()? {
                if batch.num_rows() == 0 { continue; }
                self.handle_partitioning(batch, &left_indices, true, 0, 0)?;
            }

            while let Some(batch) = self.right.next()? {
                if batch.num_rows() == 0 { continue; }
                self.handle_partitioning(batch, &right_indices, false, 0, 0)?;
            }

            self.state = JoinState::JoiningPartitions { current_partition: 0 };
            return Ok(());
        }

        let mut right_batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        let mut right_bytes = 0usize;
        while let Some(batch) = self.right.next()? {
            if batch.num_rows() > 0 {
                right_bytes += SpillContext::estimate_batch_bytes(&batch);
                right_batches.push(batch);
            }
        }

        let build_side_bytes = left_bytes.min(right_bytes);
        if build_side_bytes > self.build_memory_budget {
            return Err(DbxError::Storage(format!(
                "OOM: HashJoin 빌드 테이블 크기 ({} MB)가 메모리 한도 ({} MB)를 초과합니다.",
                build_side_bytes / (1024 * 1024),
                self.build_memory_budget / (1024 * 1024),
            )));
        }

        if left_batches.is_empty() || right_batches.is_empty() {
            self.build_table = Some(AHashMap::new());
            self.left_batch = None;
            self.right_batches = Some(Vec::new());
            return Ok(());
        }

        let left_rows: usize = left_batches.iter().map(|b| b.num_rows()).sum();
        let right_rows: usize = right_batches.iter().map(|b| b.num_rows()).sum();

        let (build_batches, probe_batches, build_is_left) =
            if right_rows < left_rows && matches!(self.join_type, JoinType::Inner) {
                (right_batches, left_batches, false)
            } else {
                (left_batches, right_batches, true)
            };

        let schema = build_batches[0].schema();
        let merged = super::super::concat_batches(&schema, build_batches.as_slice())?;
        
        let key_columns: Vec<usize> = if build_is_left {
            self.on.iter().map(|(left_col, _)| *left_col).collect()
        } else {
            self.on.iter().map(|(_, right_col)| *right_col).collect()
        };

        let mut hash_table: AHashMap<Vec<u8>, Vec<usize>> = AHashMap::new();
        for row_idx in 0..merged.num_rows() {
            let key = extract_join_key(&merged, &key_columns, row_idx);
            hash_table.entry(key).or_default().push(row_idx);
        }

        self.left_batch = Some(merged);
        self.right_batches = Some(probe_batches.into_vec());
        self.build_table = Some(hash_table);

        Ok(())
    }

    fn next_in_memory(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.left_batch.is_none() || self.right_batches.is_none() {
            self.done = true;
            return Ok(None);
        }
        let build_table = self.build_table.as_ref().unwrap();
        let left_batch = self.left_batch.as_ref().unwrap();
        let right_batches = self.right_batches.as_ref().unwrap();
        
        Self::do_probe(
            &mut self.right_batch_idx,
            build_table,
            left_batch,
            right_batches,
            self.join_type,
            &self.on,
            &self.schema,
        )
    }

    fn estimate_partition_size(&self, paths: &[PathBuf]) -> DbxResult<usize> {
        let mut total = 0;
        for path in paths {
            let meta = std::fs::metadata(path).map_err(|e| DbxError::Storage(e.to_string()))?;
            total += meta.len() as usize;
        }
        Ok(total)
    }

    fn next_partitioned(&mut self, current_part_ptr: &mut usize) -> DbxResult<Option<RecordBatch>> {
        loop {
            if let Some(right_batches) = &self.right_batches {
                if self.right_batch_idx < right_batches.len() {
                    let build_table = self.build_table.as_ref().expect("Partition build table must exist");
                    let left_batch = self.left_batch.as_ref().expect("Partition left batch must exist");
                    if let Some(result) = Self::do_probe(
                        &mut self.right_batch_idx,
                        build_table,
                        left_batch,
                        right_batches,
                        self.join_type,
                        &self.on,
                        &self.schema,
                    )? {
                        return Ok(Some(result));
                    }
                }
            }

            if *current_part_ptr >= self.num_partitions {
                self.done = true;
                return Ok(None);
            }

            let part_idx = *current_part_ptr;
            let depth = self.partition_depths[part_idx];
            *current_part_ptr += 1;

            // Hybrid: Partition 0이 메모리에 있으면 바로 사용 (디스크 로드 생략)
            if part_idx == 0 && !self.left_partition0.is_empty() {
                let l_schema = Arc::new(self.left.schema().clone());
                let l0_batches = std::mem::take(&mut self.left_partition0);
                let left_merged = super::super::concat_batches(&l_schema, &l0_batches)?;
                
                let key_indices: Vec<usize> = self.on.iter().map(|(l, _)| *l).collect();
                let mut hash_table: AHashMap<Vec<u8>, Vec<usize>> = AHashMap::new();
                for row_idx in 0..left_merged.num_rows() {
                    let key = extract_join_key(&left_merged, &key_indices, row_idx);
                    hash_table.entry(key).or_default().push(row_idx);
                }
                
                let r0_batches = std::mem::take(&mut self.right_partition0);
                self.partition0_mem_used = 0;
                
                self.left_batch = Some(left_merged);
                self.right_batches = Some(r0_batches);
                self.build_table = Some(hash_table);
                self.right_batch_idx = 0;
                continue;
            }

            if self.left_partitions[part_idx].is_empty() {
                continue;
            }

            // Recursive Partitioning: 단일 파티션이 여전히 메모리 예산을 초과하면 더 나눔
            let left_size = self.estimate_partition_size(&self.left_partitions[part_idx])?;
            if left_size > self.build_memory_budget {
                if depth >= 3 {
                    return Err(DbxError::Storage(format!(
                        "OOM: 최대 재귀 깊이(3)에 도달했습니다. 데이터 스큐가 너무 심해 파티셔닝이 불가합니다. (Part: {})",
                        part_idx
                    )));
                }

                let next_depth = depth + 1;
                let base_idx = self.num_partitions;
                
                // 파티션 목록 확장
                self.num_partitions += 32;
                self.left_partitions.resize(self.num_partitions, Vec::new());
                self.right_partitions.resize(self.num_partitions, Vec::new());
                self.partition_depths.resize(self.num_partitions, next_depth);
                
                let left_paths = std::mem::take(&mut self.left_partitions[part_idx]);
                let right_paths = std::mem::take(&mut self.right_partitions[part_idx]);
                
                let left_indices: Vec<usize> = self.on.iter().map(|(l, _)| *l).collect();
                let right_indices: Vec<usize> = self.on.iter().map(|(_, r)| *r).collect();

                for path in left_paths {
                    for batch in SpillContext::reload_batches(&path)? {
                        self.handle_partitioning(batch, &left_indices, true, next_depth, base_idx)?;
                    }
                }
                for path in right_paths {
                    for batch in SpillContext::reload_batches(&path)? {
                        self.handle_partitioning(batch, &right_indices, false, next_depth, base_idx)?;
                    }
                }
                continue;
            }

            let mut l_batches = Vec::new();
            for path in &self.left_partitions[part_idx] {
                l_batches.extend(SpillContext::reload_batches(path)?);
            }
            let l_schema = Arc::new(self.left.schema().clone());
            let left_merged = super::super::concat_batches(&l_schema, &l_batches)?;

            let key_indices: Vec<usize> = self.on.iter().map(|(l, _)| *l).collect();
            let mut hash_table: AHashMap<Vec<u8>, Vec<usize>> = AHashMap::new();
            for row_idx in 0..left_merged.num_rows() {
                let key = extract_join_key(&left_merged, &key_indices, row_idx);
                hash_table.entry(key).or_default().push(row_idx);
            }

            let mut r_batches = Vec::new();
            for path in &self.right_partitions[part_idx] {
                r_batches.extend(SpillContext::reload_batches(path)?);
            }

            self.left_batch = Some(left_merged);
            self.right_batches = Some(r_batches);
            self.build_table = Some(hash_table);
            self.right_batch_idx = 0;
        }
    }

    /// 실제 Probing 작업을 처리하는 메서드 (Borrow Checker 회피를 위해 static으로 분리)
    fn do_probe(
        right_batch_idx: &mut usize,
        build_table: &AHashMap<Vec<u8>, Vec<usize>>,
        left_batch: &RecordBatch,
        right_batches: &[RecordBatch],
        join_type: JoinType,
        on: &[(usize, usize)],
        output_schema: &Arc<Schema>,
    ) -> DbxResult<Option<RecordBatch>> {
        while *right_batch_idx < right_batches.len() {
            let right_batch = &right_batches[*right_batch_idx];
            *right_batch_idx += 1;
            if right_batch.num_rows() == 0 { continue; }

            let mut left_indices = Vec::new();
            let mut right_indices = Vec::new();

            let mut matched_left_rows = if matches!(join_type, JoinType::Left) {
                Some(std::collections::HashSet::new())
            } else {
                None
            };
            let mut matched_right_rows = if matches!(join_type, JoinType::Right) {
                Some(vec![false; right_batch.num_rows()])
            } else {
                None
            };

            let right_key_columns: Vec<usize> = on.iter().map(|(_, r)| *r).collect();
            for right_row in 0..right_batch.num_rows() {
                let key = extract_join_key(right_batch, &right_key_columns, right_row);
                if let Some(left_rows) = build_table.get(&key) {
                    for &left_row in left_rows {
                        left_indices.push(left_row as u32);
                        right_indices.push(right_row as u32);
                        if let Some(ref mut m) = matched_left_rows { m.insert(left_row); }
                        if let Some(ref mut m) = matched_right_rows { m[right_row] = true; }
                    }
                }
            }

            if let Some(matched) = matched_left_rows {
                for left_row in 0..left_batch.num_rows() {
                    if !matched.contains(&left_row) {
                        left_indices.push(left_row as u32);
                        right_indices.push(u32::MAX);
                    }
                }
            }
            if let Some(matched) = matched_right_rows {
                for (right_row, &was_matched) in matched.iter().enumerate() {
                    if !was_matched {
                        left_indices.push(u32::MAX);
                        right_indices.push(right_row as u32);
                    }
                }
            }

            if left_indices.is_empty() { continue; }

            let mut output_columns: Vec<ArrayRef> = Vec::new();
            for col in left_batch.columns() {
                output_columns.push(create_column_with_nulls(col, &left_indices, u32::MAX)?);
            }
            for col in right_batch.columns() {
                output_columns.push(create_column_with_nulls(col, &right_indices, u32::MAX)?);
            }

            let result = RecordBatch::try_new(Arc::clone(output_schema), output_columns)?;
            if result.num_rows() > 0 { return Ok(Some(result)); }
        }
        Ok(None)
    }
}

impl PhysicalOperator for HashJoinOperator {
    fn schema(&self) -> &Schema { &self.schema }
    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.done { return Ok(None); }
        if self.build_table.is_none() && matches!(self.state, JoinState::InMemory) {
            self.build_phase()?;
        }
        
        let state = self.state.clone();
        match state {
            JoinState::InMemory | JoinState::Partitioning => self.next_in_memory(),
            JoinState::JoiningPartitions { mut current_partition } => {
                let res = self.next_partitioned(&mut current_partition);
                self.state = JoinState::JoiningPartitions { current_partition };
                res
            }
        }
    }
    fn reset(&mut self) -> DbxResult<()> {
        self.build_table = None;
        self.left_batch = None;
        self.right_batches = None;
        self.right_batch_idx = 0;
        self.done = false;
        self.state = JoinState::InMemory;
        self.left.reset()?;
        self.right.reset()
    }
}

fn extract_join_key(batch: &RecordBatch, key_columns: &[usize], row_idx: usize) -> Vec<u8> {
    let mut key = Vec::new();
    for &col_idx in key_columns {
        append_value_to_key(&mut key, batch.column(col_idx), row_idx);
    }
    key
}

fn append_value_to_key(key: &mut Vec<u8>, col: &ArrayRef, row_idx: usize) {
    if col.is_null(row_idx) {
        key.push(0);
        return;
    }
    key.push(1);
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
            key.extend_from_slice(format!("{:?}", col).as_bytes());
        }
    }
}

fn create_column_with_nulls(
    source_col: &ArrayRef,
    indices: &[u32],
    null_sentinel: u32,
) -> DbxResult<ArrayRef> {
    let num_rows = indices.len();
    let has_nulls = indices.iter().any(|&idx| idx == null_sentinel);
    if !has_nulls {
        let idx_arr = UInt32Array::from(indices.to_vec());
        return compute::take(source_col.as_ref(), &idx_arr, None).map_err(Into::into);
    }

    match source_col.data_type() {
        DataType::Int32 => {
            let source = source_col.as_any().downcast_ref::<Int32Array>().unwrap();
            let mut builder = Int32Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel { builder.append_null(); }
                else { builder.append_value(source.value(idx as usize)); }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Int64 => {
            let source = source_col.as_any().downcast_ref::<Int64Array>().unwrap();
            let mut builder = Int64Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel { builder.append_null(); }
                else { builder.append_value(source.value(idx as usize)); }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Float64 => {
            let source = source_col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut builder = Float64Builder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel { builder.append_null(); }
                else { builder.append_value(source.value(idx as usize)); }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Utf8 => {
            let source = source_col.as_any().downcast_ref::<StringArray>().unwrap();
            let mut builder = StringBuilder::with_capacity(num_rows, num_rows * 10);
            for &idx in indices {
                if idx == null_sentinel { builder.append_null(); }
                else { builder.append_value(source.value(idx as usize)); }
            }
            Ok(Arc::new(builder.finish()))
        }
        DataType::Boolean => {
            let source = source_col.as_any().downcast_ref::<BooleanArray>().unwrap();
            let mut builder = BooleanBuilder::with_capacity(num_rows);
            for &idx in indices {
                if idx == null_sentinel { builder.append_null(); }
                else { builder.append_value(source.value(idx as usize)); }
            }
            Ok(Arc::new(builder.finish()))
        }
        _ => Err(DbxError::SqlExecution {
            message: format!("Unsupported type: {:?}", source_col.data_type()),
            context: "create_column_with_nulls".to_string(),
        }),
    }
}
