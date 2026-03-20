use crate::engine::database::Database;
use crate::error::DbxResult;
use std::sync::Arc;
use tokio::task::spawn_blocking;

/// 분산 그리드 및 비동기 웹 환경을 위한 Async 데이터베이스 래퍼
#[derive(Clone)]
pub struct DatabaseAsync {
    inner: Arc<Database>,
}

impl DatabaseAsync {
    /// 기존 동기 방식의 Database 인스턴스를 바탕으로 Async 래퍼를 생성합니다.
    pub fn new(db: Arc<Database>) -> Self {
        Self { inner: db }
    }

    /// 내부 동기 인스턴스를 반환합니다.
    pub fn inner(&self) -> Arc<Database> {
        self.inner.clone()
    }

    /// (비동기) 데이터 삽입
    pub async fn insert(&self, table: String, key: Vec<u8>, value: Vec<u8>) -> DbxResult<()> {
        let db = self.inner.clone();
        spawn_blocking(move || db.insert(&table, &key, &value))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Async insert thread panicked".to_string(),
                    context: e.to_string(),
                })
            })
    }

    /// (비동기) 데이터 조회
    pub async fn get(&self, table: String, key: Vec<u8>) -> DbxResult<Option<Vec<u8>>> {
        let db = self.inner.clone();
        spawn_blocking(move || db.get(&table, &key))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Async get thread panicked".to_string(),
                    context: e.to_string(),
                })
            })
    }

    /// (비동기) 데이터 삭제
    pub async fn delete(&self, table: String, key: Vec<u8>) -> DbxResult<()> {
        let db = self.inner.clone();
        spawn_blocking(move || db.delete(&table, &key).map(|_| ()))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Async delete thread panicked".to_string(),
                    context: e.to_string(),
                })
            })
    }

    /// (비동기) 원자적 CAS 연산 - 값 존재하지 않을 때 삽입
    pub async fn insert_if_not_exists(
        &self,
        table: String,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> DbxResult<bool> {
        let db = self.inner.clone();
        spawn_blocking(move || db.insert_if_not_exists(&table, &key, &value))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Async CAS thread panicked".to_string(),
                    context: e.to_string(),
                })
            })
    }

    /// (비동기) 원자적 CAS 연산 - 조건 일치 시 업데이트
    pub async fn compare_and_swap(
        &self,
        table: String,
        key: Vec<u8>,
        expected: Vec<u8>,
        new_value: Vec<u8>,
    ) -> DbxResult<bool> {
        let db = self.inner.clone();
        spawn_blocking(move || db.compare_and_swap(&table, &key, &expected, &new_value))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Async CAS thread panicked".to_string(),
                    context: e.to_string(),
                })
            })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
use crate::grid::dlm::DistributedLockManager;
use std::time::Duration;

/// 분산 그리드 환경 전용 (Option 2: 분리형 수동 모드)
/// 기존 `RowLockManager`의 고속 성능을 방해하지 않도록,
/// 명시적으로 네트워크 락(DLM)을 먼저 잡고 로컬 연산을 수행하는 래퍼입니다.
#[derive(Clone)]
pub struct GridDatabaseAsync {
    inner: Arc<Database>,
    dlm: Arc<DistributedLockManager>,
}

impl GridDatabaseAsync {
    pub fn new(db: Arc<Database>, dlm: Arc<DistributedLockManager>) -> Self {
        Self { inner: db, dlm }
    }

    /// (분산 락) 락을 획득한 후 데이터를 안전하게 삽입합니다.
    pub async fn insert_with_lock(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        let fencing_token = self
            .dlm
            .acquire(table, key, 5000, Duration::from_secs(3))
            .await
            .map_err(|e| crate::error::DbxError::InvalidOperation {
                message: "DLM acquire failed (insert)".to_string(),
                context: format!("{:?}", e),
            })?;

        let db = self.inner.clone();
        let t = table.to_string();
        let k = key.to_vec();
        let v = value.to_vec();

        let result = spawn_blocking(move || db.insert(&t, &k, &v))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Grid async insert thread panicked".to_string(),
                    context: e.to_string(),
                })
            });

        // 결과에 상관없이 락은 반드시 반환 (Drop Guard 형태로 승급 가능)
        self.dlm.release(table, key, fencing_token).await;

        result
    }

    /// (분산 락) 원격 기반 CAS 구조체 연산
    pub async fn compare_and_swap_with_lock(
        &self,
        table: &str,
        key: &[u8],
        expected: &[u8],
        new_value: &[u8],
    ) -> DbxResult<bool> {
        let fencing_token = self
            .dlm
            .acquire(table, key, 5000, Duration::from_secs(3))
            .await
            .map_err(|e| crate::error::DbxError::InvalidOperation {
                message: "DLM acquire failed (CAS)".to_string(),
                context: format!("{:?}", e),
            })?;

        let db = self.inner.clone();
        let t = table.to_string();
        let k = key.to_vec();
        let ex = expected.to_vec();
        let nv = new_value.to_vec();

        let result = spawn_blocking(move || db.compare_and_swap(&t, &k, &ex, &nv))
            .await
            .unwrap_or_else(|e| {
                Err(crate::error::DbxError::InvalidOperation {
                    message: "Grid async CAS thread panicked".to_string(),
                    context: e.to_string(),
                })
            });

        self.dlm.release(table, key, fencing_token).await;

        result
    }
}
