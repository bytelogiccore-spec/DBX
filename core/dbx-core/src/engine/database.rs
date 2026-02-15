//! Database struct definition — the core data structure

use crate::engine::types::BackgroundJob;
use crate::engine::{DeltaVariant, DurabilityLevel, WosVariant};
use crate::sql::optimizer::QueryOptimizer;
use crate::sql::parser::SqlParser;
use crate::storage::encryption::EncryptionConfig;
use crate::transaction::manager::TransactionManager;
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

    /// Tier 3: WOS (Write-Optimized Store) — persistent storage (plain or encrypted)
    pub(crate) wos: WosVariant,

    /// Schema registry: table_name → Arrow Schema
    #[allow(dead_code)]
    pub(crate) schemas: Arc<RwLock<HashMap<String, Arc<Schema>>>>,

    /// SQL table registry: table_name → Vec<RecordBatch>
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

    /// Checkpoint manager for WAL maintenance (optional)
    #[allow(dead_code)]
    pub(crate) checkpoint_manager: Option<Arc<crate::wal::checkpoint::CheckpointManager>>,

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

    /// Parallel Execution Engine for multi-threaded query execution
    #[allow(dead_code)]
    pub(crate) parallel_engine: Arc<crate::engine::parallel_engine::ParallelExecutionEngine>,
}
