//! TableScan Operator — Sequential RecordBatch emission

use crate::error::DbxResult;
use crate::sql::executor::operators::PhysicalOperator;
use arrow::datatypes::Schema;
use arrow::record_batch::RecordBatch;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, sync_channel};

/// 테이블 스캔 연산자 — 5-Tier 하이브리드 스트리밍 지원 (Phase 6)
pub struct TableScanOperator {
    table: String,
    schema: Arc<Schema>,
    projection: Vec<usize>,

    /// 백그라운드 태스크에서 비동기로 수신하는 채널 (Option A 패턴)
    receiver: Option<Receiver<DbxResult<RecordBatch>>>,
}

impl TableScanOperator {
    pub fn new(table: String, schema: Arc<Schema>, projection: Vec<usize>) -> Self {
        Self {
            table,
            schema,
            projection,
            receiver: None,
        }
    }

    /// (기존 하위 호환) 인메모리 데이터를 즉시 스캔 채널로 변환하여 주입합니다.
    pub fn set_data(&mut self, batches: Vec<RecordBatch>) {
        if batches.is_empty() {
            self.receiver = None;
            return;
        }

        let (tx, rx) = sync_channel(batches.len());
        for batch in batches {
            let _ = tx.send(Ok(batch));
        }
        self.receiver = Some(rx);
    }

    /// 파일(ROS)과 메모리(WOS)를 동시에 스트리밍하여 통합하는 5-Tier 스캔을 백그라운드로 시작합니다.
    pub fn start_tier_scan(&mut self, wos_batches: Vec<RecordBatch>, ros_files: Vec<String>) {
        // 프리페치(Prefetch)를 위한 채널 버퍼 (32 배치를 미리 읽음)
        let (tx, rx) = sync_channel(32);

        std::thread::Builder::new()
            .name(format!("tablescan-{}", self.table))
            .spawn(move || {
                // 1. WOS 메모리 데이터 우선 전송
                for batch in wos_batches {
                    if tx.send(Ok(batch)).is_err() {
                        return; // 수신자가 종료(Drop)됨
                    }
                }

                // 2. 외부 저장소 (Parquet ROS 티어) 순차 스캔
                for file_path in ros_files {
                    let path = std::path::Path::new(&file_path);
                    if !path.exists() {
                        continue;
                    }

                    match crate::storage::parquet_io::ParquetReader::read(path) {
                        Ok(batches) => {
                            for batch in batches {
                                if tx.send(Ok(batch)).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(Err(e));
                            return;
                        }
                    }
                }
            })
            .ok();

        self.receiver = Some(rx);
    }

    /// Get the table name this operator scans.
    pub fn table_name(&self) -> &str {
        &self.table
    }
}

impl PhysicalOperator for TableScanOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        if let Some(rx) = &self.receiver {
            match rx.recv() {
                Ok(Ok(batch)) => {
                    // Apply projection if specified
                    if self.projection.is_empty() {
                        Ok(Some(batch))
                    } else {
                        use arrow::array::ArrayRef;
                        use arrow::datatypes::Field;

                        let projected_columns: Vec<ArrayRef> = self
                            .projection
                            .iter()
                            .map(|&idx| Arc::clone(batch.column(idx)))
                            .collect();
                        let projected_fields: Vec<Field> = self
                            .projection
                            .iter()
                            .map(|&idx| batch.schema().field(idx).clone())
                            .collect();
                        let projected_schema = Arc::new(Schema::new(projected_fields));
                        Ok(Some(RecordBatch::try_new(
                            projected_schema,
                            projected_columns,
                        )?))
                    }
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Ok(None), // 채널 끊김 = 데이터 소진
            }
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> DbxResult<()> {
        // 스트리밍 방식에서는 Reset 지원 불가 (Unsupported) 혹은 채널 재생성 필요
        // 분산 환경에서는 Fragment가 재배포되므로 로컬 리셋은 사용하지 않습니다.
        Ok(())
    }
}
