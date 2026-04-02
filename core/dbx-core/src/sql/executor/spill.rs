//! Spill-to-Disk 메모리 방어 컨텍스트
//!
//! `SpillContext`는 쿼리 단위로 생성되는 메모리 예산 추적기입니다.
//! 메모리 임계치를 초과할 경우 Arrow IPC 포맷으로 임시 파일에 Spill 합니다.
//!
//! # 설계 원칙
//! - **쿼리 단위 격리**: 글로벌 상태 없음. Atomic 불필요 (`next()`는 `&mut self`)
//! - **Arrow IPC 포맷**: Parquet 대비 낮은 쓰기 오버헤드 (메타데이터/통계 없음)
//! - **자동 정리**: `SpillContext` drop 시 TempDir 자동 삭제

use crate::error::{DbxError, DbxResult};
use arrow::ipc::reader::FileReader;
use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use tempfile::TempDir;

/// 쿼리 단위 Spill 컨텍스트.
///
/// `HashAggregateOperator` 등 메모리 집약 연산자에 주입되어,
/// 메모리 사용량이 `budget_bytes`를 초과하면 RecordBatch를 임시 디스크로 내보냅니다.
pub struct SpillContext {
    /// 이 쿼리에 허용된 최대 메모리 바이트 (기본: 128MB)
    budget_bytes: usize,
    /// 현재 추적 중인 메모리 사용량 (바이트)
    used_bytes: usize,
    /// Spill 파일이 기록되는 임시 디렉토리 (Drop 시 자동 삭제)
    _temp_dir: TempDir,
    /// 임시 디렉토리 경로 (파일 생성 시 사용)
    temp_path: PathBuf,
    /// 생성된 Spill 파일 카운터
    spill_count: usize,
}

impl SpillContext {
    /// 기본 메모리 예산(128MB)으로 SpillContext 생성.
    pub fn new() -> DbxResult<Self> {
        Self::with_budget(128 * 1024 * 1024)
    }

    /// 지정된 메모리 예산(바이트)으로 SpillContext 생성.
    pub fn with_budget(budget_bytes: usize) -> DbxResult<Self> {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| DbxError::Storage(format!("Spill: tempdir 생성 실패: {}", e)))?;
        let temp_path = temp_dir.path().to_path_buf();
        Ok(Self {
            budget_bytes,
            used_bytes: 0,
            _temp_dir: temp_dir,
            temp_path,
            spill_count: 0,
        })
    }

    /// 바이트 단위 메모리 사용량을 등록합니다.
    /// RecordBatch의 크기는 `estimate_batch_bytes`로 추정합니다.
    #[inline]
    pub fn track(&mut self, bytes: usize) {
        self.used_bytes += bytes;
    }

    /// 현재 메모리 사용량이 예산을 초과했는지 확인합니다.
    #[inline]
    pub fn should_spill(&self) -> bool {
        self.used_bytes >= self.budget_bytes
    }

    /// 메모리 사용량 추적을 초기화합니다 (Spill 후 재시작).
    #[inline]
    pub fn reset_tracking(&mut self) {
        self.used_bytes = 0;
    }

    /// RecordBatch 목록을 Arrow IPC 포맷으로 임시 파일에 씁니다.
    ///
    /// # 반환
    /// Spill된 파일의 경로를 반환합니다.
    pub fn spill_batches(&mut self, batches: &[RecordBatch]) -> DbxResult<PathBuf> {
        if batches.is_empty() {
            return Err(DbxError::Storage("Spill: 빈 batch 목록".to_string()));
        }

        let file_path = self
            .temp_path
            .join(format!("spill_{}.ipc", self.spill_count));
        self.spill_count += 1;
        self.write_ipc_file(&file_path, batches)?;

        self.reset_tracking();
        Ok(file_path)
    }

    /// 특정 파티션에 속한 RecordBatch를 Spill 합니다 (Grace Hash Join 용).
    /// 파일명 규칙: `{side}_{part_idx}_{count}.ipc`
    pub fn spill_partition_batch(
        &mut self,
        side: &str,
        part_idx: usize,
        batch: RecordBatch,
    ) -> DbxResult<PathBuf> {
        let file_path = self
            .temp_path
            .join(format!("{}_{}_{}.ipc", side, part_idx, self.spill_count));
        self.spill_count += 1;
        self.write_ipc_file(&file_path, &[batch])?;
        Ok(file_path)
    }

    /// 내부 유틸리티: RecordBatch 목록을 IPC 파일로 기록.
    fn write_ipc_file(&self, path: &PathBuf, batches: &[RecordBatch]) -> DbxResult<()> {
        let file = File::create(path).map_err(|e| {
            DbxError::Storage(format!("Spill: 파일 생성 실패 {}: {}", path.display(), e))
        })?;
        let writer_buf = BufWriter::new(file);
        let schema = batches[0].schema();

        let mut writer = FileWriter::try_new(writer_buf, &schema).map_err(|e| {
            DbxError::Storage(format!("Spill: Arrow IPC writer 초기화 실패: {}", e))
        })?;

        for batch in batches {
            writer
                .write(batch)
                .map_err(|e| DbxError::Storage(format!("Spill: batch 쓰기 실패: {}", e)))?;
        }

        writer
            .finish()
            .map_err(|e| DbxError::Storage(format!("Spill: IPC 파일 완료 실패: {}", e)))?;

        Ok(())
    }

    /// Spill 파일에서 RecordBatch 목록을 읽어옵니다.
    ///
    /// 읽기 완료 후 파일은 삭제되지 않습니다 (TempDir drop 시 일괄 삭제).
    pub fn reload_batches(path: &PathBuf) -> DbxResult<Vec<RecordBatch>> {
        let file = File::open(path).map_err(|e| {
            DbxError::Storage(format!(
                "Spill: reload 파일 열기 실패 {}: {}",
                path.display(),
                e
            ))
        })?;

        let reader = FileReader::try_new(file, None).map_err(|e| {
            DbxError::Storage(format!("Spill: Arrow IPC reader 초기화 실패: {}", e))
        })?;

        let mut batches = Vec::new();
        for result in reader {
            let batch =
                result.map_err(|e| DbxError::Storage(format!("Spill: batch 읽기 실패: {}", e)))?;
            batches.push(batch);
        }

        Ok(batches)
    }

    /// RecordBatch의 메모리 사용량을 추정합니다 (컬럼 버퍼 합산).
    pub fn estimate_batch_bytes(batch: &RecordBatch) -> usize {
        batch
            .columns()
            .iter()
            .map(|col| col.get_buffer_memory_size())
            .sum()
    }

    /// 현재 메모리 사용량 반환 (바이트).
    #[cfg(test)]
    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    /// 생성된 Spill 파일 수 반환.
    #[cfg(test)]
    pub fn spill_count(&self) -> usize {
        self.spill_count
    }

    /// 메모리 예산 반환 (바이트).
    #[cfg(test)]
    pub fn budget_bytes(&self) -> usize {
        self.budget_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, Int32Array};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    fn make_batch(n: usize) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("val", DataType::Float64, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(Int32Array::from((0..n as i32).collect::<Vec<_>>())),
                Arc::new(Float64Array::from(
                    (0..n).map(|i| i as f64).collect::<Vec<_>>(),
                )),
            ],
        )
        .unwrap()
    }

    #[test]
    fn test_spill_context_creation() {
        let ctx = SpillContext::new().unwrap();
        assert_eq!(ctx.budget_bytes(), 128 * 1024 * 1024);
        assert!(!ctx.should_spill());
    }

    #[test]
    fn test_memory_tracking() {
        let mut ctx = SpillContext::with_budget(1000).unwrap();
        ctx.track(500);
        assert!(!ctx.should_spill());
        ctx.track(600);
        assert!(ctx.should_spill());
    }

    #[test]
    fn test_spill_and_reload_round_trip() {
        let mut ctx = SpillContext::new().unwrap();
        let batch1 = make_batch(100);
        let batch2 = make_batch(50);
        let batches = vec![batch1, batch2];

        let path = ctx.spill_batches(&batches).unwrap();
        assert!(path.exists());
        assert_eq!(ctx.spill_count(), 1);
        assert_eq!(ctx.used_bytes(), 0); // reset 됐어야 함

        let reloaded = SpillContext::reload_batches(&path).unwrap();
        let total_rows: usize = reloaded.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 150);
    }

    #[test]
    fn test_estimate_batch_bytes() {
        let batch = make_batch(1000);
        let size = SpillContext::estimate_batch_bytes(&batch);
        assert!(size > 0, "배치 크기가 0보다 커야 함");
    }

    #[test]
    fn test_multiple_spills() {
        let mut ctx = SpillContext::new().unwrap();
        let batch = make_batch(10);

        let path1 = ctx.spill_batches(&[batch.clone()]).unwrap();
        let path2 = ctx.spill_batches(&[batch]).unwrap();

        assert_ne!(path1, path2, "각 Spill은 다른 파일이어야 함");
        assert_eq!(ctx.spill_count(), 2);
    }
}
