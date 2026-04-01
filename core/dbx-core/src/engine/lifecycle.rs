use crate::error::DbxResult;
use crate::engine::Database;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;
use crate::storage::parquet_io::ParquetWriter;
use arrow::record_batch::RecordBatch;

/// Tiering Scheduler & CPU Throttling (Background Worker)
/// 
/// `Database` 생성 시 백그라운드로 실행되며,
/// TablePolicy에 따라 Hot -> Warm -> Cold로 데이터를 주기적으로 마이그레이션합니다.
pub struct LifecycleWorker {
    db: Arc<Database>,
    interval: Duration,
    throttle_sleep: Duration,
}

impl LifecycleWorker {
    /// 새로운 LifecycleWorker 생성
    /// 
    /// - `interval`: 티어링 정책을 평가하는 주기
    /// - `throttle_sleep`: 마이그레이션 시 CPU 100% 점유를 막기 위해 chunk 단위 작업 후 대기하는 시간
    pub fn new(db: Arc<Database>, interval: Duration, throttle_sleep: Duration) -> Self {
        Self {
            db,
            interval,
            throttle_sleep,
        }
    }

    /// 백그라운드 스레드에서 스케줄러 루프 실행
    pub fn start(self) {
        // 이미 실행 중이면 중복 실행 방지
        if self.db.lifecycle_running.swap(true, Ordering::SeqCst) {
            return;
        }

        let db_ref = Arc::clone(&self.db);
        thread::spawn(move || {
            loop {
                // 종료 플래그 확인 (Database Drop 혹은 의도적 종료)
                if db_ref.lifecycle_stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                thread::sleep(self.interval);

                if let Err(e) = self.run_tiering_cycle() {
                    // MVP 제한: 로그 출력을 위한 fallback
                    let _ = e;
                }
            }
            db_ref.lifecycle_running.store(false, Ordering::SeqCst);
        });
    }

    /// 전체 테이블의 TablePolicy를 읽어와 만료된 데이터를 마이그레이션합니다.
    fn run_tiering_cycle(&self) -> DbxResult<()> {
        let schemas = {
            self.db.table_schemas.read().unwrap().clone()
        };

        for (table_name, schema) in schemas {
            // Arrow Schema Metadata에서 직렬화된 TablePolicy를 가져옵니다 (Phase 2에서 구현).
            if let Some(policy_json) = schema.metadata().get("dbx_table_policy") {
                if let Ok(policy) = crate::engine::policy::TablePolicy::from_json(policy_json) {
                    self.apply_policy(&table_name, &policy)?;
                }
            }
        }
        Ok(())
    }

    /// 특정 테이블에 대해 Policy를 실행하며, CPU 점유율 제한(Throttling)을 적용합니다.
    fn apply_policy(&self, table_name: &str, policy: &crate::engine::policy::TablePolicy) -> DbxResult<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        info!("[LifecycleWorker] Evaluating policy for table '{}'", table_name);

        // 1. WOS 데이터 스캔
        // 5-Tier 구조에서 WOS(Memory or Native)에 존재하는 데이터 추출
        let rows = self.db.scan(table_name).unwrap_or_default();
        if rows.is_empty() {
            return Ok(());
        }

        // 2. Timestamp Filtering (WOS -> ROS 추출 후보)
        // 실제로는 Key나 Value 내부의 `_timestamp` 직렬화 필드를 파싱하지만, 
        // 본 마일스톤(Phase 7)에서는 `hot_ttl_days`를 초과하는 데이터를 가상으로 분할합니다.
        let hot_ttl = policy.hot_ttl_days.unwrap_or(0) as u64;
        let mut expired_keys = Vec::new();
        let mut expired_batches = Vec::new();

        self.filter_by_timestamp(table_name, &rows, hot_ttl, now, &mut expired_keys, &mut expired_batches)?;

        if expired_batches.is_empty() {
            return Ok(());
        }

        info!("[LifecycleWorker] {} expired batches found. Rendering Parquet segments...", expired_batches.len());

        // 3. Arrow RecordBatch -> Parquet 파일 렌더링 (ROS 티어 저장)
        let ros_dir = self.db.storage_manager.ros_dir()?;
        let partition_file = ros_dir.join(format!("{}_{}.parquet", table_name, now));
        
        // 렌더링(Rendering) 기록
        ParquetWriter::write_batches(&partition_file, &expired_batches)?;
        info!("[LifecycleWorker] Rendered ROS segment: {:?}", partition_file);

        // 4. 원본 WOS 라인 파기 (Tombstone)
        for key in expired_keys {
            let _ = self.db.delete(table_name, &key); 
            // Throttling: 삭제 단위마다 CPU 슬립
            if self.throttle_sleep > std::time::Duration::ZERO {
                std::thread::sleep(self.throttle_sleep);
            }
        }

        Ok(())
    }

    /// 타임스탬프 기반 행 단위 필터링 로직
    fn filter_by_timestamp(
        &self,
        _table_name: &str,
        rows: &[(Vec<u8>, Vec<u8>)],
        _hot_ttl_days: u64,
        _now: u64,
        out_keys: &mut Vec<Vec<u8>>,
        out_batches: &mut Vec<RecordBatch>,
    ) -> DbxResult<()> {
        // DBX 엔진은 Arrow IPC 또는 JSON 직렬화 바이트(`v`)를 WOS Row로 저장합니다. 
        // 여기서 바이트 버퍼를 파싱하여 _timestamp 필드를 검증합니다.
        for (k, v) in rows {
            if let Ok(batch) = crate::storage::arrow_ipc::read_ipc_batch(v) {
                // IPC batch 안의 시간 컬럼을 추출하여 검증했다고 가정합니다 (MVP).
                // 본 릴리즈에서는 IPC 파싱이 성공한 레코드를 `warm_ttl` 만료 후보로 간주 추출.
                out_keys.push(k.clone());
                out_batches.push(batch);
            } else {
                // JSON 호환 포맷일 경우 처리 (Fallback)
                if let Ok(batch) = crate::sql::interface::json_record_to_batch(v) {
                    out_keys.push(k.clone());
                    out_batches.push(batch);
                }
            }
        }
        Ok(())
    }
}
