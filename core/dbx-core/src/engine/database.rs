//! Database struct definition — the core data structure

use crate::engine::types::{BackgroundJob, TablePersistence};
use crate::engine::{DeltaVariant, DurabilityLevel, WosVariant};
use crate::sql::optimizer::QueryOptimizer;
use crate::sql::parser::SqlParser;
use crate::storage::encryption::EncryptionConfig;
use crate::transaction::mvcc::manager::TransactionManager;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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
}
