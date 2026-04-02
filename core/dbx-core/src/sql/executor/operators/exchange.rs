use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::physical_operator::PhysicalOperator;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use arrow::ipc::reader::StreamReader;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

/// 분산 쿼리 스트리밍 파이프라인의 물리 오퍼레이터 (수신 담당)
/// 코디네이터 노드에서 상위 연산자(Aggregate 등)에 연결되어
/// Bounded Channel로부터 동기적으로(blocking) 바이너리 배치를 가져옵니다.
pub struct GridExchangeOperator {
    schema: Arc<Schema>,
    receiver: Receiver<DbxResult<Option<Vec<u8>>>>,
}

impl GridExchangeOperator {
    pub fn new(schema: Arc<Schema>, receiver: Receiver<DbxResult<Option<Vec<u8>>>>) -> Self {
        Self { schema, receiver }
    }
}

impl PhysicalOperator for GridExchangeOperator {
    fn schema(&self) -> &Schema {
        &self.schema
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        // QUIC 수신 스레드로부터 데이터가 올 때까지 블록킹 대기
        // Bounded channel의 특성 덕분에 여기서 천천히 꺼내가면 송신측 백프레셔(Backpressure)가 발생합니다.
        match self.receiver.blocking_recv() {
            Some(Ok(Some(raw_bytes))) => {
                // I/O 스레드가 아닌 CPU 연산(Physical) 스레드에서 백그라운드 디코딩
                let cursor = Cursor::new(raw_bytes);
                let mut reader =
                    StreamReader::try_new(cursor, None).map_err(|e| DbxError::SqlExecution {
                        message: format!("Arrow IPC stream init failed: {}", e),
                        context: "GridExchangeOperator".to_string(),
                    })?;

                if let Some(result) = reader.next() {
                    let batch = result.map_err(|e| DbxError::SqlExecution {
                        message: format!("Arrow IPC parse failed: {}", e),
                        context: "GridExchangeOperator".to_string(),
                    })?;
                    Ok(Some(batch))
                } else {
                    Err(DbxError::SqlExecution {
                        message: "Empty IPC stream received".to_string(),
                        context: "GridExchangeOperator".to_string(),
                    })
                }
            }
            Some(Ok(None)) => Ok(None), // 정상 EOF
            Some(Err(e)) => Err(e),
            None => {
                // Sender가 EOF 신호 없이 비정상 종료 시 에러 처리
                Err(DbxError::SqlExecution {
                    message: "Grid exchange channel closed unexpectedly".to_string(),
                    context: "GridExchangeOperator".to_string(),
                })
            }
        }
    }

    fn reset(&mut self) -> DbxResult<()> {
        Err(DbxError::SqlExecution {
            message: "Reset is not supported on GridExchangeOperator".to_string(),
            context: "GridExchangeOperator".to_string(),
        })
    }
}
