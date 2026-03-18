//! Database Constructors - factory methods for creating Database instances

use crate::engine::types::BackgroundJob;
use crate::engine::{Database, DbConfig, DeltaVariant, DurabilityLevel, WosVariant};
use crate::error::DbxResult;
use crate::index::HashIndex;
use crate::sql::optimizer::QueryOptimizer;
use crate::sql::parser::SqlParser;
use crate::sql::view::ViewRegistry;
use crate::storage::StorageBackend; // Add this for trait methods
use crate::storage::delta_store::DeltaStore;
use crate::storage::encryption::EncryptionConfig;
use crate::storage::encryption::wos::EncryptedWosBackend;
use crate::storage::memory_wos::InMemoryWosBackend;
use crate::storage::native_wos::NativeWosBackend;
use crate::transaction::mvcc::manager::TransactionManager; // Fix path
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{info, instrument};

/// Spawn a background worker thread that handles WAL sync and index update jobs.
fn spawn_background_worker(
    rx: std::sync::mpsc::Receiver<BackgroundJob>,
    wal: Option<Arc<crate::wal::WriteAheadLog>>,
    enc_wal: Option<Arc<crate::wal::encrypted_wal::EncryptedWal>>,
    index: Arc<HashIndex>,
) {
    std::thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            match job {
                BackgroundJob::WalSync => {
                    if let Some(w) = &wal {
                        let _ = w.sync();
                    }
                }
                BackgroundJob::EncryptedWalSync => {
                    if let Some(w) = &enc_wal {
                        let _ = w.sync();
                    }
                }
                BackgroundJob::IndexUpdate {
                    table,
                    column,
                    key,
                    row_id,
                } => {
                    let _ = index.update_on_insert(&table, &column, &key, row_id);
                }
            }
        }
    });
}

impl Database {
    /// 데이터베이스를 열거나 생성합니다.
    ///
    /// 지정된 경로에 데이터베이스를 생성하거나 기존 데이터베이스를 엽니다.
    /// WOS (sled)를 통해 영구 저장소를 제공합니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 디렉토리 경로
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use std::path::Path;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open(Path::new("./data"))?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(path))]
    pub fn open(path: &Path) -> DbxResult<Arc<Self>> {
        info!("Opening database at {:?}", path);
        let wos_path = path.join("wos");
        std::fs::create_dir_all(&wos_path)?;

        // Initialize WAL
        let wal_path = path.join("wal.log");
        let wal = Arc::new(crate::wal::WriteAheadLog::open(&wal_path)?);

        let wos_backend = Arc::new(NativeWosBackend::open(&wos_path)?);
        let db_index = Arc::new(HashIndex::new());

        // Load persisted metadata (schemas, indexes, and triggers)
        let loaded_schemas = crate::engine::metadata::load_all_schemas(&wos_backend)?;
        let loaded_indexes = crate::engine::metadata::load_all_indexes(&wos_backend)?;
        let loaded_triggers = crate::engine::metadata::load_all_triggers(&wos_backend)?;
        let loaded_procedures = crate::engine::metadata::load_all_procedures(&wos_backend)?;
        let loaded_schedules = crate::engine::metadata::load_all_schedules(&wos_backend)?;

        info!(
            "Loaded {} schemas, {} indexes, {} triggers, {} procedures, and {} schedules from persistent storage",
            loaded_schemas.len(),
            loaded_indexes.len(),
            loaded_triggers.len(),
            loaded_procedures.len(),
            loaded_schedules.len()
        );

        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(rx, Some(wal.clone()), None, Arc::clone(&db_index));

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            memory_wos: WosVariant::InMemory(Arc::new(InMemoryWosBackend::new())),
            file_wos: Some(WosVariant::Native(Arc::clone(&wos_backend))),
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(loaded_schemas)),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: Some(wal),
            encrypted_wal: None,

            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(loaded_indexes),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(crate::automation::ProcedureExecutor::new())),
            schedule_executor: Arc::new(RwLock::new(crate::automation::ScheduleExecutor::new())),
            parallel_engine: Arc::new(
                crate::engine::parallel_engine::ParallelExecutionEngine::new_auto()
                    .expect("Failed to create parallel engine"),
            ),
            view_registry: ViewRegistry::new(),
        };

        // Perform crash recovery
        let apply_fn = |record: &crate::wal::WalRecord| -> DbxResult<()> {
            match record {
                crate::wal::WalRecord::Insert {
                    table,
                    key,
                    value,
                    ts: _,
                } => {
                    db.delta.insert(table, key, value)?;
                }
                crate::wal::WalRecord::Delete { table, key, ts: _ } => {
                    db.delta.delete(table, key)?;
                }
                crate::wal::WalRecord::Batch { table, rows, ts: _ } => {
                    db.delta.insert_batch(table, rows.clone())?;
                }
                _ => {}
            }
            Ok(())
        };

        let recovered_count =
            crate::wal::checkpoint::CheckpointManager::recover(&wal_path, apply_fn)?;
        if recovered_count > 0 {
            info!("Recovered {} WAL records", recovered_count);
            // Flush recovered data to WOS to prevent duplicate inserts
            info!("Flushing recovered WAL data to WOS");
            db.flush()?;
        }

        // Auto-register loaded SQL triggers
        if !loaded_triggers.is_empty() {
            info!(
                "Auto-registering {} persisted triggers",
                loaded_triggers.len()
            );
            let mut executor = db.trigger_executor.write().unwrap();
            executor.register_all(loaded_triggers);
        }

        // Auto-register loaded SQL procedures
        if !loaded_procedures.is_empty() {
            info!(
                "Auto-registering {} persisted procedures",
                loaded_procedures.len()
            );
            let mut executor = db.procedure_executor.write().unwrap();
            executor.register_all(loaded_procedures);
        }

        // Auto-register loaded SQL schedules
        if !loaded_schedules.is_empty() {
            info!(
                "Auto-registering {} persisted schedules",
                loaded_schedules.len()
            );
            let executor = db.schedule_executor.write().unwrap();
            for (_, schedule) in loaded_schedules {
                let _ = executor.register(schedule);
            }
        }

        info!("Database opened successfully");

        // Wrap in Arc
        let db_arc = Arc::new(db);

        // Start background scheduler
        let db_weak = Arc::downgrade(&db_arc);
        db_arc
            .schedule_executor
            .write()
            .unwrap()
            .start_scheduler(db_weak)?;

        Ok(db_arc)
    }

    /// 암호화된 데이터베이스를 열거나 생성합니다.
    ///
    /// 지정된 경로에 암호화된 데이터베이스를 생성하거나 기존 암호화 DB를 엽니다.
    /// WAL과 WOS 모두 암호화됩니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 디렉토리 경로
    /// * `encryption` - 암호화 설정 (패스워드 또는 raw key 기반)
    ///
    /// # 예제
    ///
    /// ```rust,no_run
    /// use dbx_core::Database;
    /// use dbx_core::storage::encryption::EncryptionConfig;
    /// use std::path::Path;
    ///
    /// let enc = EncryptionConfig::from_password("my-secret-password");
    /// let db = Database::open_encrypted(Path::new("./data"), enc).unwrap();
    /// ```
    #[instrument(skip(path, encryption))]
    pub fn open_encrypted(path: &Path, encryption: EncryptionConfig) -> DbxResult<Self> {
        info!("Opening encrypted database at {:?}", path);
        let wos_path = path.join("wos");
        std::fs::create_dir_all(&wos_path)?;

        // Initialize encrypted WAL
        let wal_path = path.join("wal.enc.log");
        let encrypted_wal = Arc::new(crate::wal::encrypted_wal::EncryptedWal::open(
            &wal_path,
            encryption.clone(),
        )?);

        // Initialize encrypted WOS
        let enc_wos = Arc::new(EncryptedWosBackend::open(&wos_path, encryption.clone())?);
        let db_index = Arc::new(HashIndex::new());

        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(
            rx,
            None,
            Some(Arc::clone(&encrypted_wal)),
            Arc::clone(&db_index),
        );

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            memory_wos: WosVariant::InMemory(Arc::new(InMemoryWosBackend::new())),
            file_wos: Some(WosVariant::Encrypted(Arc::clone(&enc_wos))),
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: Some(Arc::clone(&encrypted_wal)),

            encryption: RwLock::new(Some(encryption)),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(crate::automation::ProcedureExecutor::new())),
            schedule_executor: Arc::new(RwLock::new(crate::automation::ScheduleExecutor::new())),
            parallel_engine: Arc::new(
                crate::engine::parallel_engine::ParallelExecutionEngine::new_auto()
                    .expect("Failed to create parallel engine"),
            ),
            view_registry: ViewRegistry::new(),
        };
        let records = encrypted_wal.replay()?;
        let mut recovered_count = 0;
        for record in &records {
            match record {
                crate::wal::WalRecord::Insert {
                    table,
                    key,
                    value,
                    ts: _,
                } => {
                    db.delta.insert(table, key, value)?;
                    recovered_count += 1;
                }
                crate::wal::WalRecord::Delete { table, key, ts: _ } => {
                    db.delta.delete(table, key)?;
                    recovered_count += 1;
                }
                crate::wal::WalRecord::Batch { table, rows, ts: _ } => {
                    db.delta.insert_batch(table, rows.clone())?;
                    recovered_count += rows.len();
                }
                _ => {}
            }
        }
        if recovered_count > 0 {
            info!("Recovered {} encrypted WAL records", recovered_count);
        }

        info!("Encrypted database opened successfully");
        Ok(db)
    }

    /// 인메모리 데이터베이스를 생성합니다.
    ///
    /// 테스트 및 임시 데이터 저장용으로 사용됩니다. 영구 저장되지 않습니다.
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.insert("cache", b"key1", b"value1")?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument]
    pub fn open_in_memory() -> DbxResult<Self> {
        info!("Creating in-memory database");
        let db_index = Arc::new(HashIndex::new());
        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(rx, None, None, Arc::clone(&db_index));

        Ok(Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            memory_wos: WosVariant::InMemory(Arc::new(InMemoryWosBackend::new())),
            file_wos: None,
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: None,

            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(crate::automation::ProcedureExecutor::new())),
            schedule_executor: Arc::new(RwLock::new(crate::automation::ScheduleExecutor::new())),
            parallel_engine: Arc::new(
                crate::engine::parallel_engine::ParallelExecutionEngine::new_auto()
                    .expect("Failed to create parallel engine"),
            ),
            view_registry: ViewRegistry::new(),
        })
    }

    /// 암호화된 인메모리 데이터베이스를 생성합니다.
    ///
    /// 테스트 및 임시 데이터 저장용으로, 메모리 상에서 value가 암호화됩니다.
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::storage::encryption::EncryptionConfig;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let enc = EncryptionConfig::from_password("secret");
    /// let db = Database::open_in_memory_encrypted(enc)?;
    /// db.insert("users", b"user:1", b"Alice")?;
    /// let val = db.get("users", b"user:1")?;
    /// assert_eq!(val, Some(b"Alice".to_vec()));
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_in_memory_encrypted(encryption: EncryptionConfig) -> DbxResult<Self> {
        let db_index = Arc::new(HashIndex::new());
        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(rx, None, None, Arc::clone(&db_index));

        Ok(Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            memory_wos: WosVariant::Encrypted(Arc::new(EncryptedWosBackend::open_temporary(
                encryption.clone(),
            )?)),
            file_wos: None,
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: None,

            encryption: RwLock::new(Some(encryption)),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(crate::automation::ProcedureExecutor::new())),
            schedule_executor: Arc::new(RwLock::new(crate::automation::ScheduleExecutor::new())),
            parallel_engine: Arc::new(
                crate::engine::parallel_engine::ParallelExecutionEngine::new_auto()
                    .expect("Failed to create parallel engine"),
            ),
            view_registry: ViewRegistry::new(),
        })
    }

    /// 최대 안전성 설정으로 데이터베이스를 엽니다 (Full durability).
    ///
    /// 금융, 의료 등 데이터 손실이 절대 허용되지 않는 경우 사용합니다.
    /// 모든 쓰기 작업마다 fsync를 수행하여 최대 안전성을 보장하지만,
    /// 성능은 기본 설정(Lazy)보다 느립니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 파일 경로
    pub fn open_safe(path: impl AsRef<Path>) -> DbxResult<Arc<Self>> {
        Self::open_with_durability(path, DurabilityLevel::Full)
    }

    /// 최고 성능 설정으로 데이터베이스를 엽니다 (No durability).
    ///
    /// WAL을 사용하지 않아 최고 성능을 제공하지만,
    /// 크래시 시 데이터 손실 가능성이 있습니다.
    /// 캐시, 임시 데이터, 벤치마크 등에 적합합니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 파일 경로
    ///
    pub fn open_fast(path: impl AsRef<Path>) -> DbxResult<Arc<Self>> {
        Self::open_with_durability(path, DurabilityLevel::None)
    }

    /// 지정된 durability 설정으로 데이터베이스를 엽니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 파일 경로
    /// * `durability` - 내구성 수준
    pub fn open_with_durability(
        path: impl AsRef<Path>,
        durability: DurabilityLevel,
    ) -> DbxResult<Arc<Self>> {
        info!(
            "Opening database at {:?} with durability {:?}",
            path.as_ref(),
            durability
        );
        let path = path.as_ref();
        let wos_path = path.join("wos");
        std::fs::create_dir_all(&wos_path)?;

        // Initialize WAL
        let wal_path = path.join("wal.log");
        let wal = Arc::new(crate::wal::WriteAheadLog::open(&wal_path)?);

        let wos_backend = Arc::new(NativeWosBackend::open(&wos_path)?);
        let db_index = Arc::new(HashIndex::new());

        // Load persisted metadata
        let loaded_schemas = crate::engine::metadata::load_all_schemas(&wos_backend)?;
        let loaded_indexes = crate::engine::metadata::load_all_indexes(&wos_backend)?;
        let loaded_triggers = crate::engine::metadata::load_all_triggers(&wos_backend)?;
        let loaded_procedures = crate::engine::metadata::load_all_procedures(&wos_backend)?;
        let loaded_schedules = crate::engine::metadata::load_all_schedules(&wos_backend)?;

        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(rx, Some(wal.clone()), None, Arc::clone(&db_index));

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            memory_wos: WosVariant::InMemory(Arc::new(InMemoryWosBackend::new())),
            file_wos: Some(WosVariant::Native(Arc::clone(&wos_backend))),
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(loaded_schemas)),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: Some(wal),
            encrypted_wal: None,
            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability, // ← set BEFORE Arc wrapping
            index_registry: RwLock::new(loaded_indexes),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(crate::automation::ProcedureExecutor::new())),
            schedule_executor: Arc::new(RwLock::new(crate::automation::ScheduleExecutor::new())),
            parallel_engine: Arc::new(
                crate::engine::parallel_engine::ParallelExecutionEngine::new_auto()
                    .expect("Failed to create parallel engine"),
            ),
            view_registry: ViewRegistry::new(),
        };

        // Crash recovery
        let apply_fn = |record: &crate::wal::WalRecord| -> DbxResult<()> {
            match record {
                crate::wal::WalRecord::Insert {
                    table,
                    key,
                    value,
                    ts: _,
                } => {
                    db.delta.insert(table, key, value)?;
                }
                crate::wal::WalRecord::Delete { table, key, ts: _ } => {
                    db.delta.delete(table, key)?;
                }
                crate::wal::WalRecord::Batch { table, rows, ts: _ } => {
                    db.delta.insert_batch(table, rows.clone())?;
                }
                _ => {}
            }
            Ok(())
        };
        let recovered_count =
            crate::wal::checkpoint::CheckpointManager::recover(&wal_path, apply_fn)?;
        if recovered_count > 0 {
            info!("Recovered {} WAL records", recovered_count);
            db.flush()?;
        }

        // Auto-register triggers, procedures, schedules
        if !loaded_triggers.is_empty() {
            db.trigger_executor
                .write()
                .unwrap()
                .register_all(loaded_triggers);
        }
        if !loaded_procedures.is_empty() {
            db.procedure_executor
                .write()
                .unwrap()
                .register_all(loaded_procedures);
        }
        if !loaded_schedules.is_empty() {
            let executor = db.schedule_executor.write().unwrap();
            for (_, schedule) in loaded_schedules {
                let _ = executor.register(schedule);
            }
        }

        info!(
            "Database opened successfully with durability {:?}",
            durability
        );

        let db_arc = Arc::new(db);
        let db_weak = Arc::downgrade(&db_arc);
        db_arc
            .schedule_executor
            .write()
            .unwrap()
            .start_scheduler(db_weak)?;

        Ok(db_arc)
    }
}
/// open_with_config: DbConfig를 받는 새 생성자 그룹
impl Database {
    /// `DbConfig`를 사용하여 데이터베이스를 엽니다.
    ///
    /// `config.parallelism.cpu_cap`으로 CPU 사용량을 제어할 수 있습니다.
    ///
    /// # 예시
    ///
    /// ```rust,no_run
    /// use dbx_core::Database;
    /// use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig};
    /// use std::path::Path;
    ///
    /// let db = Database::open_with_config(
    ///     Path::new("./data"),
    ///     DbConfig {
    ///         parallelism: ParallelismConfig::conservative(), // CPU 50%만 사용
    ///     },
    /// ).unwrap();
    /// ```
    pub fn open_with_config(
        path: &std::path::Path,
        config: DbConfig,
    ) -> DbxResult<std::sync::Arc<Self>> {
        use crate::engine::parallel_engine::{ParallelExecutionEngine, ParallelizationPolicy};
        use crate::storage::native_wos::NativeWosBackend;
        use crate::transaction::mvcc::manager::TransactionManager;
        use dashmap::DashMap;
        use std::collections::HashMap;
        use crate::index::HashIndex;
        use crate::sql::optimizer::QueryOptimizer;
        use crate::sql::parser::SqlParser;
        use std::sync::{Arc, RwLock};
        use tracing::info;

        info!("Opening database at {:?} with custom config", path);
        let wos_path = path.join("wos");
        std::fs::create_dir_all(&wos_path)?;

        let wal_path = path.join("wal.log");
        let wal = Arc::new(crate::wal::WriteAheadLog::open(&wal_path)?);

        let wos_backend = Arc::new(NativeWosBackend::open(&wos_path)?);
        let db_index = Arc::new(HashIndex::new());

        let loaded_schemas = crate::engine::metadata::load_all_schemas(&wos_backend)?;
        let loaded_indexes = crate::engine::metadata::load_all_indexes(&wos_backend)?;
        let loaded_triggers = crate::engine::metadata::load_all_triggers(&wos_backend)?;
        let loaded_procedures = crate::engine::metadata::load_all_procedures(&wos_backend)?;
        let loaded_schedules = crate::engine::metadata::load_all_schedules(&wos_backend)?;

        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        spawn_background_worker(rx, Some(wal.clone()), None, Arc::clone(&db_index));

        // 병렬 엔진을 config로 생성
        let parallel_engine = Arc::new(
            ParallelExecutionEngine::new_with_config(
                ParallelizationPolicy::Auto,
                config.parallelism.clone(),
            )
            .expect("Failed to create parallel engine"),
        );

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(crate::storage::delta_store::DeltaStore::new())),
            memory_wos: crate::engine::WosVariant::InMemory(Arc::new(
                crate::storage::memory_wos::InMemoryWosBackend::new(),
            )),
            file_wos: Some(crate::engine::WosVariant::Native(Arc::clone(&wos_backend))),
            table_persistence: DashMap::new(),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(loaded_schemas)),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: Some(wal),
            encrypted_wal: None,
            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(loaded_indexes),
            automation_engine: Arc::new(crate::automation::ExecutionEngine::new()),
            trigger_registry: crate::engine::automation_api::TriggerRegistry::new(),
            trigger_executor: Arc::new(RwLock::new(crate::automation::TriggerExecutor::new())),
            procedure_executor: Arc::new(RwLock::new(
                crate::automation::ProcedureExecutor::new(),
            )),
            schedule_executor: Arc::new(RwLock::new(
                crate::automation::ScheduleExecutor::new(),
            )),
            parallel_engine,
            view_registry: ViewRegistry::new(),
        };

        // Crash recovery
        let apply_fn = |record: &crate::wal::WalRecord| -> DbxResult<()> {
            match record {
                crate::wal::WalRecord::Insert { table, key, value, ts: _ } => {
                    db.delta.insert(table, key, value)?;
                }
                crate::wal::WalRecord::Delete { table, key, ts: _ } => {
                    db.delta.delete(table, key)?;
                }
                crate::wal::WalRecord::Batch { table, rows, ts: _ } => {
                    db.delta.insert_batch(table, rows.clone())?;
                }
                _ => {}
            }
            Ok(())
        };
        let recovered_count =
            crate::wal::checkpoint::CheckpointManager::recover(&wal_path, apply_fn)?;
        if recovered_count > 0 {
            info!("Recovered {} WAL records", recovered_count);
            db.flush()?;
        }

        // Triggers, procedures, schedules
        if !loaded_triggers.is_empty() {
            db.trigger_executor.write().unwrap().register_all(loaded_triggers);
        }
        if !loaded_procedures.is_empty() {
            db.procedure_executor.write().unwrap().register_all(loaded_procedures);
        }
        if !loaded_schedules.is_empty() {
            let executor = db.schedule_executor.write().unwrap();
            for (_, schedule) in loaded_schedules {
                let _ = executor.register(schedule);
            }
        }

        info!("Database opened with custom parallelism config (cpu_cap={:.0}%)",
              config.parallelism.cpu_cap * 100.0);

        let db_arc = Arc::new(db);
        let db_weak = Arc::downgrade(&db_arc);
        db_arc.schedule_executor.write().unwrap().start_scheduler(db_weak)?;

        Ok(db_arc)
    }
}
