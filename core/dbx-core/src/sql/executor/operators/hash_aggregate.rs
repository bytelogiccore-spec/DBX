//! HashAggregate Operator — GROUP BY and aggregate functions

use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::planner::{AggregateFunction, PhysicalAggExpr};
use ahash::AHashMap;
use arrow::array::*;
use arrow::compute;
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

use crate::storage::gpu::GpuManager;

/// Hash Aggregate 연산자 (GROUP BY) — AHashMap 기반 집계
pub struct HashAggregateOperator {
    input: Box<dyn PhysicalOperator>,
    schema: Arc<Schema>,
    /// Column indices to group by
    group_by: Vec<usize>,
    /// Aggregate expressions
    aggregates: Vec<PhysicalAggExpr>,
    /// 분산 집계 모드
    mode: crate::sql::planner::types::AggregateMode,
    /// Whether result has been produced
    done: bool,
    /// GPU manager for acceleration
    gpu_manager: Option<Arc<GpuManager>>,
}

impl HashAggregateOperator {
    pub fn new(
        input: Box<dyn PhysicalOperator>,
        schema: Arc<Schema>,
        group_by: Vec<usize>,
        aggregates: Vec<PhysicalAggExpr>,
        mode: crate::sql::planner::types::AggregateMode,
    ) -> Self {
        Self {
            input,
            schema,
            group_by,
            aggregates,
            mode,
            done: false,
            gpu_manager: None,
        }
    }

    pub fn with_gpu(mut self, gpu: Option<Arc<GpuManager>>) -> Self {
        self.gpu_manager = gpu;
        self
    }

    fn aggregate_all(&mut self) -> DbxResult<Option<RecordBatch>> {
        // Collect all input
        let mut batches: SmallVec<[RecordBatch; 8]> = smallvec![];
        let mut total_rows = 0;
        while let Some(batch) = self.input.next()? {
            if batch.num_rows() > 0 {
                total_rows += batch.num_rows();
                batches.push(batch);
            }
        }

        if batches.is_empty() {
            return Ok(Some(RecordBatch::new_empty(Arc::clone(&self.schema))));
        }

        let input_schema = batches[0].schema();
        let merged = concat_batches(&input_schema, batches.as_slice())?;

        if self.group_by.is_empty() {
            // Global aggregate (no GROUP BY)
            return self.global_aggregate(&merged);
        }

        // Group by: build groups
        let mut groups: AHashMap<Vec<u8>, Vec<usize>> = AHashMap::new();
        for row_idx in 0..merged.num_rows() {
            let mut key = Vec::new();
            for &col_idx in &self.group_by {
                append_value_to_key(&mut key, merged.column(col_idx), row_idx);
            }
            groups.entry(key).or_default().push(row_idx);
        }

        // Build output columns
        let num_groups = groups.len();
        let group_keys: Vec<Vec<usize>> = groups.values().cloned().collect();

        // Group-by columns
        let mut output_columns: Vec<ArrayRef> = Vec::new();
        for &col_idx in &self.group_by {
            let col = merged.column(col_idx);
            let first_indices: Vec<usize> = group_keys.iter().map(|rows| rows[0]).collect();
            output_columns.push(take_by_indices(col, &first_indices)?);
        }

        // Aggregate columns
        use crate::sql::planner::types::AggregateMode;
        for agg in &self.aggregates {
            let col = merged.column(agg.input);
            let result = match self.mode {
                AggregateMode::Simple | AggregateMode::Partial => {
                    compute_aggregate_grouped(col, &agg.function, &group_keys, num_groups, self.mode)?
                }
                AggregateMode::Final => {
                    merge_aggregate_grouped(col, &agg.function, &group_keys, num_groups)?
                }
            };
            output_columns.push(result);
        }

        Ok(Some(RecordBatch::try_new(
            Arc::clone(&self.schema),
            output_columns,
        )?))
    }

    fn global_aggregate(&self, batch: &RecordBatch) -> DbxResult<Option<RecordBatch>> {
        let mut columns: Vec<ArrayRef> = Vec::new();
        for agg in &self.aggregates {
            let col = batch.column(agg.input);
            let result = match self.mode {
                crate::sql::planner::types::AggregateMode::Simple
                | crate::sql::planner::types::AggregateMode::Partial => {
                    compute_aggregate_global(col, &agg.function, self.mode)?
                }
                crate::sql::planner::types::AggregateMode::Final => {
                    merge_aggregate_global(col, &agg.function)?
                }
            };
            columns.push(result);
        }
        Ok(Some(RecordBatch::try_new(
            Arc::clone(&self.schema),
            columns,
        )?))
    }

    /// Try to perform global aggregation on GPU.
    /// Returns None if GPU acceleration is not possible for the given query/types.
    fn try_gpu_global_aggregate(&self, batches: &[RecordBatch]) -> DbxResult<Option<RecordBatch>> {
        let gpu = match &self.gpu_manager {
            Some(g) => g,
            None => return Ok(None),
        };

        let mut results = Vec::with_capacity(self.aggregates.len());

        for agg in &self.aggregates {
            // Currently only Int32 SUM/COUNT supported on GPU
            let first_col = batches[0].column(agg.input);
            if first_col.data_type() != &DataType::Int32 {
                return Ok(None);
            }

            match agg.function {
                AggregateFunction::Sum => {
                    let mut total_sum = 0i64;
                    for batch in batches {
                        // Upload to temporary table in GPU cache
                        gpu.upload_batch_pinned("_temp_agg", batch)?;
                        let column_name = batch.schema().field(agg.input).name().to_string();
                        total_sum += gpu.sum("_temp_agg", &column_name)?;
                        gpu.clear_table_cache("_temp_agg");
                    }
                    results.push(Arc::new(Float64Array::from(vec![total_sum as f64])) as ArrayRef);
                }
                AggregateFunction::Count => {
                    let mut total_count = 0u64;
                    for batch in batches {
                        gpu.upload_batch_pinned("_temp_agg", batch)?;
                        let column_name = batch.schema().field(agg.input).name().to_string();
                        total_count += gpu.count("_temp_agg", &column_name)?;
                        gpu.clear_table_cache("_temp_agg");
                    }
                    results.push(Arc::new(Int64Array::from(vec![total_count as i64])) as ArrayRef);
                }
                _ => return Ok(None), // Other functions fallback to CPU
            }
        }

        Ok(Some(RecordBatch::try_new(
            Arc::clone(&self.schema),
            results,
        )?))
    }
}

impl PhysicalOperator for HashAggregateOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if self.done {
            return Ok(None);
        }
        self.done = true;
        self.aggregate_all()
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.done = false;
        self.input.reset()
    }
}

// ===== Helper Functions =====

/// Concatenate multiple RecordBatches into one.
fn concat_batches(schema: &Arc<Schema>, batches: &[RecordBatch]) -> DbxResult<RecordBatch> {
    if batches.len() == 1 {
        return Ok(batches[0].clone());
    }
    Ok(compute::concat_batches(schema, batches)?)
}

/// Take rows by indices from a column.
fn take_by_indices(col: &ArrayRef, indices: &[usize]) -> DbxResult<ArrayRef> {
    let idx_arr = UInt32Array::from(indices.iter().map(|&i| i as u32).collect::<Vec<_>>());
    Ok(compute::take(col.as_ref(), &idx_arr, None)?)
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



/// Compute an aggregate function over groups of rows (순차 처리 - 소규모 그룹).
fn compute_aggregate_grouped(
    col: &ArrayRef,
    func: &AggregateFunction,
    groups: &[Vec<usize>],
    num_groups: usize,
    mode: crate::sql::planner::types::AggregateMode,
) -> DbxResult<ArrayRef> {
    use crate::sql::planner::types::AggregateMode;
    
    // For each group, compute the aggregate value
    match col.data_type() {
        DataType::Int32 | DataType::Int64 | DataType::Float64 => {
            if matches!(func, AggregateFunction::Avg) && mode == AggregateMode::Partial {
                // Partial AVG: Return Struct(sum, count)
                let mut sum_builder = Float64Builder::with_capacity(num_groups);
                let mut count_builder = Int64Builder::with_capacity(num_groups);
                
                for group_rows in groups {
                    let (sum, count) = aggregate_avg_group(col, group_rows);
                    sum_builder.append_value(sum);
                    count_builder.append_value(count as i64);
                }
                
                let fields = vec![
                    Arc::new(arrow::datatypes::Field::new("sum", DataType::Float64, false)),
                    Arc::new(arrow::datatypes::Field::new("count", DataType::Int64, false)),
                ];
                let arrays = vec![
                    Arc::new(sum_builder.finish()) as ArrayRef,
                    Arc::new(count_builder.finish()) as ArrayRef,
                ];
                Ok(Arc::new(StructArray::try_new(fields.into(), arrays, None)?) as ArrayRef)
            } else {
                let mut builder = Float64Builder::with_capacity(num_groups);
                for group_rows in groups {
                    let val = match col.data_type() {
                        DataType::Int32 => aggregate_i32_group(col.as_any().downcast_ref::<Int32Array>().unwrap(), group_rows, func),
                        DataType::Int64 => aggregate_i64_group(col.as_any().downcast_ref::<Int64Array>().unwrap(), group_rows, func),
                        DataType::Float64 => aggregate_f64_group(col.as_any().downcast_ref::<Float64Array>().unwrap(), group_rows, func),
                        _ => unreachable!(),
                    };
                    builder.append_value(val);
                }
                Ok(Arc::new(builder.finish()))
            }
        }
        _ => {
            // COUNT works for any type
            if matches!(func, AggregateFunction::Count) {
                let mut builder = Float64Builder::with_capacity(num_groups);
                for group_rows in groups {
                    let count = group_rows.iter().filter(|&&i| !col.is_null(i)).count();
                    builder.append_value(count as f64);
                }
                Ok(Arc::new(builder.finish()))
            } else {
                Err(DbxError::NotImplemented(format!(
                    "aggregate {:?} for type {:?}",
                    func,
                    col.data_type()
                )))
            }
        }
    }
}

fn aggregate_avg_group(col: &ArrayRef, rows: &[usize]) -> (f64, usize) {
    let mut sum = 0.0;
    let mut count = 0;
    
    match col.data_type() {
        DataType::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            for &i in rows {
                if !arr.is_null(i) {
                    sum += arr.value(i) as f64;
                    count += 1;
                }
            }
        }
        DataType::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            for &i in rows {
                if !arr.is_null(i) {
                    sum += arr.value(i) as f64;
                    count += 1;
                }
            }
        }
        DataType::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            for &i in rows {
                if !arr.is_null(i) {
                    sum += arr.value(i);
                    count += 1;
                }
            }
        }
        _ => {}
    }
    (sum, count)
}

/// Merge partial aggregate results (Final phase)
fn merge_aggregate_grouped(
    col: &ArrayRef,
    func: &AggregateFunction,
    groups: &[Vec<usize>],
    num_groups: usize,
) -> DbxResult<ArrayRef> {
    match func {
        AggregateFunction::Avg => {
            // Input is StructArray(sum, count)
            let struct_arr = col.as_any().downcast_ref::<StructArray>().ok_or_else(|| {
                DbxError::SqlExecution {
                    message: "Expected StructArray for Final AVG".to_string(),
                    context: "HashAggregate Final".to_string(),
                }
            })?;
            let sum_arr = struct_arr.column(0).as_any().downcast_ref::<Float64Array>().unwrap();
            let count_arr = struct_arr.column(1).as_any().downcast_ref::<Int64Array>().unwrap();
            
            let mut builder = Float64Builder::with_capacity(num_groups);
            for group_rows in groups {
                let mut total_sum = 0.0;
                let mut total_count = 0;
                for &i in group_rows {
                    if !struct_arr.is_null(i) {
                        total_sum += sum_arr.value(i);
                        total_count += count_arr.value(i) as usize;
                    }
                }
                if total_count > 0 {
                    builder.append_value(total_sum / total_count as f64);
                } else {
                    builder.append_value(0.0);
                }
            }
            Ok(Arc::new(builder.finish()))
        }
        AggregateFunction::Count | AggregateFunction::Sum => {
            // Both are merged by summing
            let arr = col.as_any().downcast_ref::<Float64Array>().ok_or_else(|| {
                DbxError::SqlExecution {
                    message: "Expected Float64Array for Final Sum/Count".to_string(),
                    context: "HashAggregate Final".to_string(),
                }
            })?;
            let mut builder = Float64Builder::with_capacity(num_groups);
            for group_rows in groups {
                let mut sum = 0.0;
                for &i in group_rows {
                    if !arr.is_null(i) {
                        sum += arr.value(i);
                    }
                }
                builder.append_value(sum);
            }
            Ok(Arc::new(builder.finish()))
        }
        AggregateFunction::Min => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut builder = Float64Builder::with_capacity(num_groups);
            for group_rows in groups {
                let mut min = f64::INFINITY;
                for &i in group_rows {
                    if !arr.is_null(i) {
                        min = min.min(arr.value(i));
                    }
                }
                builder.append_value(min);
            }
            Ok(Arc::new(builder.finish()))
        }
        AggregateFunction::Max => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut builder = Float64Builder::with_capacity(num_groups);
            for group_rows in groups {
                let mut max = f64::NEG_INFINITY;
                for &i in group_rows {
                    if !arr.is_null(i) {
                        max = max.max(arr.value(i));
                    }
                }
                builder.append_value(max);
            }
            Ok(Arc::new(builder.finish()))
        }
    }
}


fn aggregate_i32_group(arr: &Int32Array, rows: &[usize], func: &AggregateFunction) -> f64 {
    let values: Vec<f64> = rows
        .iter()
        .filter(|&&i| !arr.is_null(i))
        .map(|&i| arr.value(i) as f64)
        .collect();
    aggregate_f64_values(&values, func)
}

fn aggregate_i64_group(arr: &Int64Array, rows: &[usize], func: &AggregateFunction) -> f64 {
    let values: Vec<f64> = rows
        .iter()
        .filter(|&&i| !arr.is_null(i))
        .map(|&i| arr.value(i) as f64)
        .collect();
    aggregate_f64_values(&values, func)
}

fn aggregate_f64_group(arr: &Float64Array, rows: &[usize], func: &AggregateFunction) -> f64 {
    let values: Vec<f64> = rows
        .iter()
        .filter(|&&i| !arr.is_null(i))
        .map(|&i| arr.value(i))
        .collect();
    aggregate_f64_values(&values, func)
}

fn aggregate_f64_values(values: &[f64], func: &AggregateFunction) -> f64 {
    match func {
        AggregateFunction::Count => values.len() as f64,
        AggregateFunction::Sum => values.iter().sum(),
        AggregateFunction::Avg => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        }
        AggregateFunction::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        AggregateFunction::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

/// Compute a global aggregate (no GROUP BY) producing a single-row array.
fn compute_aggregate_global(
    col: &ArrayRef,
    func: &AggregateFunction,
    mode: crate::sql::planner::types::AggregateMode,
) -> DbxResult<ArrayRef> {
    use crate::sql::planner::types::AggregateMode;
    
    if matches!(func, AggregateFunction::Avg) && mode == AggregateMode::Partial {
        let (sum, count) = aggregate_avg_group(col, &(0..col.len()).collect::<Vec<_>>());
        
        let fields = vec![
            Arc::new(arrow::datatypes::Field::new("sum", DataType::Float64, false)),
            Arc::new(arrow::datatypes::Field::new("count", DataType::Int64, false)),
        ];
        let arrays = vec![
            Arc::new(Float64Array::from(vec![sum])) as ArrayRef,
            Arc::new(Int64Array::from(vec![count as i64])) as ArrayRef,
        ];
        return Ok(Arc::new(StructArray::try_new(fields.into(), arrays, None)?) as ArrayRef);
    }

    // COUNT는 항상 Int64 반환 (Simple/Partial 모드에서)
    if matches!(func, AggregateFunction::Count) {
        let count = (0..col.len()).filter(|&i| !col.is_null(i)).count() as i64;
        return Ok(Arc::new(Int64Array::from(vec![count])));
    }

    // 다른 집계 함수들은 입력 타입에 따라 처리
    match col.data_type() {
        DataType::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            let values: Vec<i32> = (0..arr.len())
                .filter(|&i| !arr.is_null(i))
                .map(|i| arr.value(i))
                .collect();

            let result = match func {
                AggregateFunction::Sum => values.iter().map(|&v| v as i64).sum::<i64>() as f64,
                AggregateFunction::Avg => {
                    if values.is_empty() {
                        0.0
                    } else {
                        values.iter().map(|&v| v as f64).sum::<f64>() / values.len() as f64
                    }
                }
                AggregateFunction::Min => values.iter().min().copied().unwrap_or(0) as f64,
                AggregateFunction::Max => values.iter().max().copied().unwrap_or(0) as f64,
                AggregateFunction::Count => unreachable!(),
            };
            Ok(Arc::new(Float64Array::from(vec![result])))
        }
        DataType::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            let values: Vec<i64> = (0..arr.len())
                .filter(|&i| !arr.is_null(i))
                .map(|i| arr.value(i))
                .collect();

            let result = match func {
                AggregateFunction::Sum => values.iter().sum::<i64>() as f64,
                AggregateFunction::Avg => {
                    if values.is_empty() {
                        0.0
                    } else {
                        values.iter().sum::<i64>() as f64 / values.len() as f64
                    }
                }
                AggregateFunction::Min => values.iter().min().copied().unwrap_or(0) as f64,
                AggregateFunction::Max => values.iter().max().copied().unwrap_or(0) as f64,
                AggregateFunction::Count => unreachable!(),
            };
            Ok(Arc::new(Float64Array::from(vec![result])))
        }
        DataType::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let values: Vec<f64> = (0..arr.len())
                .filter(|&i| !arr.is_null(i))
                .map(|i| arr.value(i))
                .collect();

            let result = match func {
                AggregateFunction::Sum => values.iter().sum(),
                AggregateFunction::Avg => {
                    if values.is_empty() {
                        0.0
                    } else {
                        values.iter().sum::<f64>() / values.len() as f64
                    }
                }
                AggregateFunction::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
                AggregateFunction::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                AggregateFunction::Count => unreachable!(),
            };
            Ok(Arc::new(Float64Array::from(vec![result])))
        }
        _ => Err(DbxError::NotImplemented(format!(
            "global aggregate {:?} for {:?}",
            func,
            col.data_type()
        ))),
    }
}

fn merge_aggregate_global(col: &ArrayRef, func: &AggregateFunction) -> DbxResult<ArrayRef> {
    match func {
        AggregateFunction::Avg => {
            let struct_arr = col.as_any().downcast_ref::<StructArray>().unwrap();
            let sum_arr = struct_arr.column(0).as_any().downcast_ref::<Float64Array>().unwrap();
            let count_arr = struct_arr.column(1).as_any().downcast_ref::<Int64Array>().unwrap();
            
            let mut total_sum = 0.0;
            let mut total_count = 0;
            for i in 0..struct_arr.len() {
                if !struct_arr.is_null(i) {
                    total_sum += sum_arr.value(i);
                    total_count += count_arr.value(i) as usize;
                }
            }
            let avg = if total_count > 0 { total_sum / total_count as f64 } else { 0.0 };
            Ok(Arc::new(Float64Array::from(vec![avg])))
        }
        AggregateFunction::Count | AggregateFunction::Sum => {
            // Count merges as SUM of counts
            let arr = col.as_any().downcast_ref::<Float64Array>().or_else(|| {
                // Handle Int64 counts
                col.as_any().downcast_ref::<Int64Array>().map(|_| panic!("Int64 count merge not optimized yet"))
            });
            
            let mut total = 0.0;
            if let Some(farr) = col.as_any().downcast_ref::<Float64Array>() {
                for i in 0..farr.len() {
                    if !farr.is_null(i) { total += farr.value(i); }
                }
            } else if let Some(iarr) = col.as_any().downcast_ref::<Int64Array>() {
                for i in 0..iarr.len() {
                    if !iarr.is_null(i) { total += iarr.value(i) as f64; }
                }
            }
            Ok(Arc::new(Float64Array::from(vec![total])))
        }
        AggregateFunction::Min => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut min = f64::INFINITY;
            for i in 0..arr.len() {
                if !arr.is_null(i) { min = min.min(arr.value(i)); }
            }
            Ok(Arc::new(Float64Array::from(vec![min])))
        }
        AggregateFunction::Max => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut max = f64::NEG_INFINITY;
            for i in 0..arr.len() {
                if !arr.is_null(i) { max = max.max(arr.value(i)); }
            }
            Ok(Arc::new(Float64Array::from(vec![max])))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sql::executor::operators::hash_aggregate::*;
    use crate::sql::planner::{AggregateFunction, PhysicalAggExpr, AggregateMode};
    use arrow::array::*;
    use arrow::datatypes::*;
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn test_partial_avg_aggregation() {
        // Prepare input: [10, 20, 30]
        let schema = Arc::new(Schema::new(vec![
            Field::new("val", DataType::Int32, false),
        ]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![Arc::new(Int32Array::from(vec![10, 20, 30]))],
        ).unwrap();

        // Partial AVG
        let result = compute_aggregate_global(
            &batch.column(0),
            &AggregateFunction::Avg,
            AggregateMode::Partial,
        ).unwrap();

        // Should return StructArray(sum: 60.0, count: 3)
        let struct_arr = result.as_any().downcast_ref::<StructArray>().unwrap();
        let sum_arr = struct_arr.column(0).as_any().downcast_ref::<Float64Array>().unwrap();
        let count_arr = struct_arr.column(1).as_any().downcast_ref::<Int64Array>().unwrap();

        assert_eq!(sum_arr.value(0), 60.0);
        assert_eq!(count_arr.value(0), 3);
    }

    #[test]
    fn test_final_avg_aggregation() {
        // Prepare partial results from 2 shards
        // Shard 1: [10, 20] -> sum: 30.0, count: 2
        // Shard 2: [30, 40, 50] -> sum: 120.0, count: 3
        
        let fields = vec![
            Arc::new(Field::new("sum", DataType::Float64, false)),
            Arc::new(Field::new("count", DataType::Int64, false)),
        ];
        let struct_arr = Arc::new(StructArray::try_new(
            fields.into(),
            vec![
                Arc::new(Float64Array::from(vec![30.0, 120.0])),
                Arc::new(Int64Array::from(vec![2, 3])),
            ],
            None,
        ).unwrap()) as ArrayRef;

        // Final AVG
        let result = merge_aggregate_global(
            &struct_arr,
            &AggregateFunction::Avg,
        ).unwrap();

        // Should return (30 + 120) / (2 + 3) = 150 / 5 = 30.0
        let res_arr = result.as_any().downcast_ref::<Float64Array>().unwrap();
        assert_eq!(res_arr.value(0), 30.0);
    }

    #[test]
    fn test_partial_sum_aggregation() {
        let arr = Arc::new(Int32Array::from(vec![1, 2, 3])) as ArrayRef;
        let result = compute_aggregate_global(
            &arr,
            &AggregateFunction::Sum,
            AggregateMode::Partial,
        ).unwrap();
        
        let res_arr = result.as_any().downcast_ref::<Float64Array>().unwrap();
        assert_eq!(res_arr.value(0), 6.0);
    }

    #[test]
    fn test_final_sum_aggregation() {
        let partial_sums = Arc::new(Float64Array::from(vec![10.0, 20.0])) as ArrayRef;
        let result = merge_aggregate_global(
            &partial_sums,
            &AggregateFunction::Sum,
        ).unwrap();
        
        let res_arr = result.as_any().downcast_ref::<Float64Array>().unwrap();
        assert_eq!(res_arr.value(0), 30.0);
    }
}
