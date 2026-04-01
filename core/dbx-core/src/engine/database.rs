//! Database struct definition — the core data structure

use crate::engine::types::{BackgroundJob, TablePersistence};
use crate::engine::{DeltaVariant, DurabilityLevel, WosVariant};
use crate::monitoring::DbxMetrics;
use crate::sql::optimizer::QueryOptimizer;
use crate::sql::parser::SqlParser;
use crate::sql::view::{SharedMaterializedViewRegistry, ViewRegistry};
use crate::storage::encryption::EncryptionConfig;
use crate::transaction::mvcc::manager::TransactionManager;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ════════════════════════════════════════════
// CAS (Compare-And-Swap) Row Latch Lock Manager
// ════════════════════════════════════════════

/// A Lock Striping mechanism to provide fine-grained, row-level locks
/// for concurrent CAS operations without memory bloat.
pub struct RowLockManager {
    stripes: Vec<std::sync::Mutex<()>>,
    mask: usize,
}

impl RowLockManager {
    /// Creates a new RowLockManager with `1 << power_of_two` stripes.
    /// E.g., `power_of_two = 10` gives 1024 locks.
    pub fn new(power_of_two: usize) -> Self {
        let size = 1 << power_of_two;
        let mut stripes = Vec::with_capacity(size);
        for _ in 0..size {
            stripes.push(std::sync::Mutex::new(()));
        }
        Self {
            stripes,
            mask: size - 1,
        }
    }

    /// Acquires a lock for a specific (table, key) combination.
    pub fn lock<'a>(&'a self, table: &str, key: &[u8]) -> std::sync::MutexGuard<'a, ()> {
        let mut hasher = ahash::AHasher::default();
        std::hash::Hash::hash(&table, &mut hasher);
        std::hash::Hash::hash(&key, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher) as usize;
        self.stripes[hash & self.mask].lock().unwrap()
    }
}

/// DBX 데이터베이스 엔진
///
/// 5-Tier Hybrid Storage 아키텍처를 관리하는 메인 API입니다.
///
/// # 데이터 흐름
///
/// - **INSERT**: Delta Store (Tier 1)에 먼저 쓰기
/// - **GET**: Delta → WOS 순서로 조회 (첫 번째 hit에서 short-circuit)
/// - **DELETE**: 모든 계층에서 삭제
/// - **flush()**: Delta Store 데이터를 WOS로 이동
///
/// # 예제
///
/// ```rust
/// use dbx_core::Database;
///
/// # fn main() -> dbx_core::DbxResult<()> {
/// let db = Database::open_in_memory()?;
/// db.insert("users", b"user:1", b"Alice")?;
/// let value = db.get("users", b"user:1")?;
/// assert_eq!(value, Some(b"Alice".to_vec()));
/// # Ok(())
/// # }
/// ```
pub struct Database {
    /// Tier 1: Delta Store (in-memory write buffer) — row-based or columnar
    pub(crate) delta: DeltaVariant,

    /// Tier 3: 메모리 WOS (테이블별 Memory 지정 시 사용)
    pub(crate) memory_wos: WosVariant,

    /// Tier 3: 파일 WOS (path 기반 DB에서만 Some, 테이블별 File/미지정 시 사용)
    pub(crate) file_wos: Option<WosVariant>,

    /// 통합 경로 관리자
    pub(crate) storage_manager: Arc<crate::storage::manager::StoragePathManager>,

    /// 테이블별 저장소 지정 (Memory | File). file_wos가 Some일 때만 사용.
    pub(crate) table_persistence: DashMap<String, TablePersistence>,

    /// Schema registry: table_name → Arrow Schema
    #[allow(dead_code)]
    pub(crate) schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,

    /// SQL table registry: table_name → `Vec<RecordBatch>`
    pub(crate) tables: RwLock<HashMap<String, Vec<RecordBatch>>>,

    /// SQL table schemas: table_name → Schema
    pub(crate) table_schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,

    /// Hash Index: fast O(1) key lookups
    pub(crate) index: Arc<crate::index::HashIndex>,

    /// Row ID counters: table_name → next_row_id
    pub(crate) row_counters: Arc<DashMap<String, std::sync::atomic::AtomicUsize>>,

    /// SQL parser (cached)
    pub(crate) sql_parser: SqlParser,
    /// SQL optimizer (cached)
    pub(crate) sql_optimizer: QueryOptimizer,

    /// Write-Ahead Log for crash recovery (optional, plain or encrypted)
    pub(crate) wal: Option<Arc<crate::wal::WriteAheadLog>>,

    /// Encrypted WAL (optional, used when encryption is enabled)
    pub(crate) encrypted_wal: Option<Arc<crate::wal::encrypted_wal::EncryptedWal>>,

    /// Encryption config (None = no encryption)
    pub(crate) encryption: RwLock<Option<EncryptionConfig>>,

    /// MVCC Transaction Manager
    pub(crate) tx_manager: Arc<TransactionManager>,

    /// Columnar Cache for OLAP queries (Tier 2)
    pub(crate) columnar_cache: Arc<crate::storage::columnar_cache::ColumnarCache>,

    /// GPU Manager for optional acceleration (Phase 6.4)
    pub(crate) gpu_manager: Option<Arc<crate::storage::gpu::GpuManager>>,

    /// Background job sender
    pub(crate) job_sender: Option<std::sync::mpsc::Sender<BackgroundJob>>,

    /// WAL 내구성 정책
    pub durability: DurabilityLevel,

    /// Index registry: index_name → (table, column) mapping for DROP INDEX
    pub(crate) index_registry: RwLock<HashMap<String, (String, String)>>,

    /// Automation & Extensibility Engine (UDF, Triggers, Scheduler)
    pub(crate) automation_engine: Arc<crate::automation::ExecutionEngine>,

    /// Trigger Registry (이벤트 매칭용)
    pub(crate) trigger_registry: crate::engine::automation_api::TriggerRegistry,

    /// SQL Trigger Executor
    pub(crate) trigger_executor: Arc<RwLock<crate::automation::TriggerExecutor>>,

    /// Stored Procedure Executor
    pub(crate) procedure_executor: Arc<RwLock<crate::automation::ProcedureExecutor>>,

    /// SQL Schedule Executor
    pub(crate) schedule_executor: Arc<RwLock<crate::automation::ScheduleExecutor>>,

    /// Parallel Execution Engine for multi-threaded query execution
    #[allow(dead_code)]
    pub(crate) parallel_engine: Arc<crate::engine::parallel_engine::ParallelExecutionEngine>,

    /// SQL View Registry — CREATE/DROP VIEW 지원
    pub(crate) view_registry: ViewRegistry,

    /// Materialized View Registry — CREATE/REFRESH/DROP MATERIALIZED VIEW 지원
    pub mat_view_registry: SharedMaterializedViewRegistry,

    /// 파티션 매핑 정보 (테이블명 -> PartitionMap)
    pub(crate) partition_maps:
        Arc<RwLock<std::collections::HashMap<String, crate::storage::partition::PartitionMap>>>,

    /// 파티션별 통계 정보 ("table__partname" → PartitionStats)
    pub(crate) partition_stats: Arc<DashMap<String, crate::storage::partition::PartitionStats>>,

    /// 파티션별 압축 설정 ("table__partname" → CompressionConfig)
    pub(crate) partition_compression:
        Arc<DashMap<String, crate::storage::compression::CompressionConfig>>,

    /// 파티션 수명 주기 정책 (테이블명 → PartitionLifecycle)
    pub(crate) partition_lifecycle:
        Arc<DashMap<String, crate::storage::partition::PartitionLifecycle>>,

    /// 파티션 티어 힌트 ("table__partname" → PartitionTierHint)
    pub(crate) partition_tier_hints:
        Arc<DashMap<String, crate::storage::partition::PartitionTierHint>>,

    /// 파티션 서브테이블 첫 쓰기 시각 ("table__partname" → UNIX timestamp secs)
    /// INSERT 경로에서 자동 기록 → partition_needs_archive/delete의 기준으로 활용
    pub(crate) partition_creation_times: Arc<DashMap<String, u64>>,

    /// Lifecycle 자동 스케줄러 중단 플래그
    /// `true`로 설정되면 백그라운드 스레드가 자동 종료 (Database Drop 시 자동 처리)
    pub(crate) lifecycle_stop_flag: Arc<std::sync::atomic::AtomicBool>,

    /// Lifecycle 스케줄러 기동 여부 (중복 spawn 방지)
    pub(crate) lifecycle_running: Arc<std::sync::atomic::AtomicBool>,

    /// Database Configuration (Parallelism, HTAP Sync)
    pub(crate) config: crate::engine::parallel_engine::DbConfig,

    /// 적응형 워크로드 분석기 (HTAP/OLAP 비율 분석)
    pub(crate) workload_analyzer: Arc<RwLock<crate::engine::workload_analyzer::WorkloadAnalyzer>>,

    /// Replication Master (MVP)
    pub(crate) replication_master: Option<Arc<crate::replication::master::ReplicationMaster>>,

    /// Sharding 라우터
    pub(crate) sharding_router: Arc<crate::sharding::router::ShardRouter>,

    /// Sharding 데이터 분산/수집 처리기
    pub(crate) scatter_gather: Arc<crate::sharding::scatter_gather::ScatterGather>,

    /// Metrics & Observability (Phase 0.4)
    pub(crate) metrics: Arc<DbxMetrics>,

    /// Atomic CAS operations를 위한 Row-level Latch 매니저 (Phase 1.1)
    pub(crate) cas_locks: Arc<RowLockManager>,
}

impl Database {
    /// 테이블에 사용할 WOS. file_wos가 None이면(인메모리 전용 DB) 항상 memory_wos.
    pub(crate) fn wos_for_table(&self, table: &str) -> &WosVariant {
        match &self.file_wos {
            None => &self.memory_wos,
            Some(file) => {
                let is_memory =
                    self.table_persistence.get(table).map(|r| *r) == Some(TablePersistence::Memory);
                if is_memory { &self.memory_wos } else { file }
            }
        }
    }

    /// 스키마/인덱스 메타데이터 저장용 WOS. 파일이 있으면 파일, 없으면 메모리.
    pub(crate) fn wos_for_metadata(&self) -> &WosVariant {
        self.file_wos.as_ref().unwrap_or(&self.memory_wos)
    }

    /// 테이블별 저장소를 지정합니다 (파일 DB에서만 유효).
    ///
    /// `TablePersistence::File`은 파일에 저장(재시작 후 유지),
    /// `TablePersistence::Memory`는 메모리만 사용(프로세스 종료 시 사라짐).
    /// 인메모리 전용 DB(`open_in_memory`)에서는 `File` 지정 시 에러를 반환합니다.
    pub fn set_table_persistence(
        &self,
        table: &str,
        persistence: TablePersistence,
    ) -> crate::error::DbxResult<()> {
        if persistence == TablePersistence::File && self.file_wos.is_none() {
            return Err(crate::error::DbxError::InvalidOperation {
                message: "TablePersistence::File requires a file-backed database (open with path)"
                    .to_string(),
                context: "Use Database::open(path) to enable per-table file persistence"
                    .to_string(),
            });
        }
        self.table_persistence
            .insert(table.to_string(), persistence);
        Ok(())
    }

    /// 테이블에 지정된 저장소 종류를 반환합니다 (미지정 시 None).
    pub fn table_persistence(&self, table: &str) -> Option<TablePersistence> {
        self.table_persistence.get(table).map(|r| *r)
    }

    /// Replication Master를 활성화하고, WAL 변경사항을 구독할 수 있는 Receiver를 반환합니다.
    ///
    /// `capacity`는 브로드캐스트 채널의 버퍼 크기입니다.
    /// 이미 활성화되어 있다면 새로운 Receiver만 생성하여 반환합니다.
    pub fn enable_replication(
        &mut self,
        capacity: usize,
    ) -> tokio::sync::broadcast::Receiver<crate::replication::protocol::ReplicationMessage> {
        use crate::replication::master::ReplicationMaster;

        if let Some(master) = &self.replication_master {
            master.subscribe()
        } else {
            let (master, rx) = ReplicationMaster::new(capacity);
            self.replication_master = Some(Arc::new(master));
            rx
        }
    }

    // ── Monitoring / Observability API (Phase 0.4) ────────────────────

    /// Export all DBX metrics in Prometheus exposition text format.
    ///
    /// The returned string can be served at a `/metrics` HTTP endpoint
    /// and consumed by Prometheus or any compatible scraper.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.insert("users", b"k1", b"v1")?;
    /// let text = db.export_metrics();
    /// assert!(text.contains("dbx_inserts_total 1"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn export_metrics(&self) -> String {
        crate::monitoring::export_prometheus(&self.metrics)
    }

    /// Return a non-atomic snapshot of current metrics.
    ///
    /// Useful for programmatic inspection of counters and hit rates.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.insert("users", b"k1", b"v1")?;
    /// let snap = db.metrics_snapshot();
    /// assert_eq!(snap.inserts_total, 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn metrics_snapshot(&self) -> crate::monitoring::MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Reset all metrics counters and histograms to zero.
    pub fn reset_metrics(&self) {
        self.metrics.reset();
    }

    // ── Streaming Ingestion API ──────────────────────────────────────────────

    /// 채널 기반 스트리밍 수집 파이프라인 생성
    ///
    /// INSERT / UPDATE / DELETE 이벤트를 [`StreamEvent`](crate::engine::StreamEvent)로
    /// 전송하면 백그라운드에서 자동 배치 처리합니다.
    ///
    /// # 인수
    ///
    /// * `table` - 이벤트를 적용할 테이블 이름
    /// * `batch_size` - 이 이벤트 수에 도달하면 즉시 flush
    /// * `max_latency_ms` - 버퍼가 차지 않아도 이 밀리초마다 강제 flush
    ///
    /// # 예시
    ///
    /// ```rust,no_run
    /// use dbx_core::{Database, engine::{StreamIngester, StreamEvent}};
    /// use std::{sync::Arc, time::Duration};
    ///
    /// let db = Arc::new(Database::open_in_memory().unwrap());
    /// let ingester = db.create_stream_ingester("orders", 1000, 100);
    /// let tx = ingester.sender();
    /// tx.send(vec![StreamEvent::Insert { key: "1".into(), value: b"data".to_vec() }]).unwrap();
    /// ingester.flush().unwrap();
    /// ```
    pub fn create_stream_ingester(
        self: &Arc<Self>,
        table: &str,
        batch_size: usize,
        max_latency_ms: u64,
    ) -> crate::engine::StreamIngester {
        crate::engine::StreamIngester::new(
            Arc::clone(self),
            table,
            batch_size,
            std::time::Duration::from_millis(max_latency_ms),
        )
    }
}
