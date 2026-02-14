//! Database Constructors — factory methods for creating Database instances

use crate::engine::types::BackgroundJob;
use crate::engine::{Database, DeltaVariant, DurabilityLevel, WosVariant};
use crate::error::DbxResult;
use crate::index::HashIndex;
use crate::sql::optimizer::QueryOptimizer;
use crate::sql::parser::SqlParser;
use crate::storage::StorageBackend; // Add this for trait methods
use crate::storage::delta_store::DeltaStore;
use crate::storage::encrypted_wos::EncryptedWosBackend;
use crate::storage::encryption::EncryptionConfig;
use crate::storage::wos::WosBackend;
use crate::transaction::manager::TransactionManager; // Fix path
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tracing::{info, instrument};

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
    pub fn open(path: &Path) -> DbxResult<Self> {
        info!("Opening database at {:?}", path);
        let wos_path = path.join("wos");
        std::fs::create_dir_all(&wos_path)?;

        // Initialize WAL
        let wal_path = path.join("wal.log");
        let wal = Arc::new(crate::wal::WriteAheadLog::open(&wal_path)?);
        let checkpoint_manager = Arc::new(crate::wal::checkpoint::CheckpointManager::new(
            Arc::clone(&wal),
            &wal_path,
        ));

        let wos_backend = Arc::new(WosBackend::open(&wos_path)?);
        let db_index = Arc::new(HashIndex::new());

        // Load persisted metadata (schemas and indexes)
        let loaded_schemas = crate::engine::metadata::load_all_schemas(&wos_backend)?;
        let loaded_indexes = crate::engine::metadata::load_all_indexes(&wos_backend)?;

        info!(
            "Loaded {} schemas and {} indexes from persistent storage",
            loaded_schemas.len(),
            loaded_indexes.len()
        );

        let (tx, rx) = std::sync::mpsc::channel::<BackgroundJob>();
        let wal_for_worker = Some(wal.clone());
        let enc_wal_for_worker: Option<Arc<crate::wal::encrypted_wal::EncryptedWal>> = None;
        let index_for_worker = Arc::clone(&db_index);

        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    BackgroundJob::WalSync => {
                        if let Some(w) = &wal_for_worker {
                            let _ = w.sync();
                        }
                    }
                    BackgroundJob::EncryptedWalSync => {
                        if let Some(w) = &enc_wal_for_worker {
                            let _ = w.sync();
                        }
                    }
                    BackgroundJob::IndexUpdate {
                        table,
                        column,
                        key,
                        row_id,
                    } => {
                        let _ = index_for_worker.update_on_insert(&table, &column, &key, row_id);
                    }
                }
            }
        });

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            wos: WosVariant::Plain(Arc::clone(&wos_backend)),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(loaded_schemas)),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: Some(wal),
            encrypted_wal: None,
            checkpoint_manager: Some(checkpoint_manager.clone()),
            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(loaded_indexes),
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

        info!("Database opened successfully");
        Ok(db)
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
        let wal_for_worker: Option<Arc<crate::wal::WriteAheadLog>> = None;
        let enc_wal_for_worker = Some(Arc::clone(&encrypted_wal));
        let index_for_worker = Arc::clone(&db_index);

        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    BackgroundJob::WalSync => {
                        if let Some(w) = &wal_for_worker {
                            let _ = w.sync();
                        }
                    }
                    BackgroundJob::EncryptedWalSync => {
                        if let Some(w) = &enc_wal_for_worker {
                            let _ = w.sync();
                        }
                    }
                    BackgroundJob::IndexUpdate {
                        table,
                        column,
                        key,
                        row_id,
                    } => {
                        let _ = index_for_worker.update_on_insert(&table, &column, &key, row_id);
                    }
                }
            }
        });

        let db = Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            wos: WosVariant::Encrypted(Arc::clone(&enc_wos)),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: Some(Arc::clone(&encrypted_wal)),
            checkpoint_manager: None,
            encryption: RwLock::new(Some(encryption)),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
        };

        // Perform crash recovery from encrypted WAL
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
        let index_for_worker = Arc::clone(&db_index);

        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    BackgroundJob::WalSync => {
                        // In-memory has no WAL to sync
                    }
                    _ => {
                        // Other jobs or IndexUpdate
                        if let BackgroundJob::IndexUpdate {
                            table,
                            column,
                            key,
                            row_id,
                        } = job
                        {
                            let _ =
                                index_for_worker.update_on_insert(&table, &column, &key, row_id);
                        }
                    }
                }
            }
        });

        Ok(Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            wos: WosVariant::InMemory(Arc::new(
                crate::storage::memory_wos::InMemoryWosBackend::new(),
            )),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: None,
            checkpoint_manager: None,
            encryption: RwLock::new(None),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
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
        let index_for_worker = Arc::clone(&db_index);

        std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                match job {
                    BackgroundJob::WalSync => {
                        // In-memory has no WAL to sync
                    }
                    _ => {
                        if let BackgroundJob::IndexUpdate {
                            table,
                            column,
                            key,
                            row_id,
                        } = job
                        {
                            let _ =
                                index_for_worker.update_on_insert(&table, &column, &key, row_id);
                        }
                    }
                }
            }
        });

        Ok(Self {
            delta: DeltaVariant::RowBased(Arc::new(DeltaStore::new())),
            wos: WosVariant::Encrypted(Arc::new(EncryptedWosBackend::open_temporary(
                encryption.clone(),
            )?)),
            schemas: Arc::new(RwLock::new(HashMap::new())),
            tables: RwLock::new(HashMap::new()),
            table_schemas: Arc::new(RwLock::new(HashMap::new())),
            index: db_index,
            row_counters: Arc::new(DashMap::new()),
            sql_parser: SqlParser::new(),
            sql_optimizer: QueryOptimizer::new(),
            wal: None,
            encrypted_wal: None,
            checkpoint_manager: None,
            encryption: RwLock::new(Some(encryption)),
            tx_manager: Arc::new(TransactionManager::new()),
            columnar_cache: Arc::new(crate::storage::columnar_cache::ColumnarCache::new()),
            gpu_manager: crate::storage::gpu::GpuManager::try_new().map(Arc::new),
            job_sender: Some(tx),
            durability: DurabilityLevel::Lazy,
            index_registry: RwLock::new(HashMap::new()),
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
    ///
    /// # 예제
    ///
    /// ```rust
    /// # use dbx_core::Database;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_safe("financial.db")?;
    /// // 모든 쓰기가 즉시 디스크에 동기화됨
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_safe(path: impl AsRef<Path>) -> DbxResult<Self> {
        let mut db = Self::open(path.as_ref())?;
        db.durability = DurabilityLevel::Full;
        Ok(db)
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
    /// # 예제
    ///
    /// ```rust
    /// # use dbx_core::Database;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_fast("cache.db")?;
    /// // 최고 성능, WAL 없음
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_fast(path: impl AsRef<Path>) -> DbxResult<Self> {
        let mut db = Self::open(path.as_ref())?;
        db.durability = DurabilityLevel::None;
        Ok(db)
    }

    /// 지정된 durability 설정으로 데이터베이스를 엽니다.
    ///
    /// # 인자
    ///
    /// * `path` - 데이터베이스 파일 경로
    /// * `durability` - 내구성 수준
    ///
    /// # 예제
    ///
    /// ```rust
    /// # use dbx_core::{Database, DurabilityLevel};
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_with_durability("app.db", DurabilityLevel::Lazy)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn open_with_durability(
        path: impl AsRef<Path>,
        durability: DurabilityLevel,
    ) -> DbxResult<Self> {
        let mut db = Self::open(path.as_ref())?;
        db.durability = durability;
        Ok(db)
    }
}
