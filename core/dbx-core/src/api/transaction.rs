//! Typestate Transaction — 타입 안전 트랜잭션
//!
//! Typestate 패턴을 사용하여 트랜잭션 오용을 컴파일 타임에 방지.
//! 트랜잭션 내 쓰기는 로컬 버퍼에 축적되며,
//! `commit()` 시 메인 Delta Store에 원자적으로 반영됩니다.

use crate::api::query::{Execute, Query, QueryOne, QueryOptional, QueryScalar};
use crate::engine::Database;
use crate::error::DbxResult;
use std::collections::HashMap;
use std::marker::PhantomData;

/// 트랜잭션 상태 트레이트
pub trait TxState {}

/// Active 상태 — 트랜잭션 진행 중
pub struct Active;

/// Committed 상태 — 커밋 완료
pub struct Committed;

/// RolledBack 상태 — 롤백 완료
pub struct RolledBack;

impl TxState for Active {}
impl TxState for Committed {}
impl TxState for RolledBack {}

/// 트랜잭션 내 쓰기 작업 로그
#[derive(Debug, Clone)]
enum TxOp {
    /// Insert(table, key, value)
    Insert(String, Vec<u8>, Vec<u8>),
    /// Delete(table, key)
    Delete(String, Vec<u8>),
    /// Batch(table, rows)
    Batch(String, Vec<(Vec<u8>, Vec<u8>)>),
}

/// Typestate Transaction
///
/// Active 상태에서만 쿼리/쓰기가 가능하며,
/// commit/rollback 후에는 컴파일 타임에 사용 불가.
pub struct Transaction<'a, S: TxState> {
    db: &'a Database,
    /// 트랜잭션 내 쓰기 작업 로그 (commit 시 일괄 적용)
    ops: Vec<TxOp>,
    /// 트랜잭션 내 로컬 읽기 버퍼 (Insert된 데이터의 읽기 지원)
    local_buffer: HashMap<String, HashMap<Vec<u8>, Option<Vec<u8>>>>,
    _state: PhantomData<S>,
}

impl Database {
    /// 트랜잭션 시작
    ///
    /// 새로운 Active 트랜잭션을 생성합니다.
    /// 트랜잭션 내 모든 쓰기는 로컬 버퍼에 축적되며,
    /// `commit()` 호출 시에만 메인 스토리지에 반영됩니다.
    pub fn begin(&self) -> DbxResult<Transaction<'_, Active>> {
        Ok(Transaction {
            db: self,
            ops: Vec::new(),
            local_buffer: HashMap::new(),
            _state: PhantomData,
        })
    }
}

impl<'a> Transaction<'a, Active> {
    // ════════════════════════════════════════════
    // Query Methods (read-through to Database)
    // ════════════════════════════════════════════

    /// SELECT 쿼리 — 여러 행 반환
    pub fn query<T: crate::api::FromRow>(&self, sql: impl Into<String>) -> Query<'_, T> {
        self.db.query(sql)
    }

    /// SELECT 쿼리 — 단일 행 반환 (없으면 에러)
    pub fn query_one<T: crate::api::FromRow>(&self, sql: impl Into<String>) -> QueryOne<'_, T> {
        self.db.query_one(sql)
    }

    /// SELECT 쿼리 — 단일 행 반환 (없으면 None)
    pub fn query_optional<T: crate::api::FromRow>(
        &self,
        sql: impl Into<String>,
    ) -> QueryOptional<'_, T> {
        self.db.query_optional(sql)
    }

    /// SELECT 쿼리 — 단일 스칼라 값 반환
    pub fn query_scalar<T: crate::api::FromScalar>(
        &self,
        sql: impl Into<String>,
    ) -> QueryScalar<'_, T> {
        self.db.query_scalar(sql)
    }

    /// INSERT/UPDATE/DELETE — 영향받은 행 수 반환
    pub fn execute(&self, sql: impl Into<String>) -> Execute<'_> {
        self.db.execute(sql)
    }

    // ════════════════════════════════════════════
    // Buffered Write Operations
    // ════════════════════════════════════════════

    /// 트랜잭션 내 INSERT — 로컬 버퍼에 저장 (commit 전까지 미반영)
    pub fn insert(&mut self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        self.ops.push(TxOp::Insert(
            table.to_string(),
            key.to_vec(),
            value.to_vec(),
        ));
        // 로컬 읽기 버퍼에도 반영 (트랜잭션 내 read-your-writes)
        self.local_buffer
            .entry(table.to_string())
            .or_default()
            .insert(key.to_vec(), Some(value.to_vec()));
        Ok(())
    }

    /// 트랜잭션 내 BATCH INSERT — 여러 키-값 쌍을 일괄 삽입 (최적화됨)
    pub fn insert_batch(&mut self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        self.ops.push(TxOp::Batch(table.to_string(), rows.clone()));
        // 로컬 읽기 버퍼에도 반영
        let table_buf = self.local_buffer.entry(table.to_string()).or_default();
        for (key, value) in rows {
            table_buf.insert(key, Some(value));
        }
        Ok(())
    }

    /// 트랜잭션 내 DELETE — 로컬 버퍼에 tombstone 기록
    pub fn delete(&mut self, table: &str, key: &[u8]) -> DbxResult<bool> {
        self.ops.push(TxOp::Delete(table.to_string(), key.to_vec()));
        // 로컬 버퍼에 tombstone (None)
        self.local_buffer
            .entry(table.to_string())
            .or_default()
            .insert(key.to_vec(), None);
        Ok(true)
    }

    /// 트랜잭션 내 GET — 로컬 버퍼 우선, 없으면 메인 스토리지 조회
    pub fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // 1. 로컬 버퍼 확인 (read-your-writes)
        if let Some(table_buf) = self.local_buffer.get(table)
            && let Some(value_opt) = table_buf.get(key)
        {
            return Ok(value_opt.clone()); // Some(value) or None (tombstone)
        }
        // 2. 메인 스토리지 fallback
        self.db.get(table, key)
    }

    /// 현재 트랜잭션의 보류 중인 연산 개수
    pub fn pending_ops(&self) -> usize {
        self.ops.len()
    }

    // ════════════════════════════════════════════
    // Commit / Rollback
    // ════════════════════════════════════════════

    /// 트랜잭션 커밋 — 모든 버퍼링된 쓰기를 메인 스토리지에 원자적으로 반영
    pub fn commit(self) -> DbxResult<Transaction<'a, Committed>> {
        // Allocate a unique commit timestamp for this transaction
        let commit_ts = self.db.allocate_commit_ts();

        // ops를 순서대로 메인 Delta Store에 적용 (MVCC 버전 포함)
        for op in &self.ops {
            match op {
                TxOp::Insert(table, key, value) => {
                    // Use insert_versioned to include MVCC timestamp
                    self.db
                        .insert_versioned(table, key, Some(value), commit_ts)?;
                }
                TxOp::Delete(table, key) => {
                    // Use insert_versioned with None to create tombstone
                    self.db.insert_versioned(table, key, None, commit_ts)?;
                }
                TxOp::Batch(table, rows) => {
                    // Batch insert with versioning
                    for (key, value) in rows {
                        self.db
                            .insert_versioned(table, key, Some(value), commit_ts)?;
                    }
                }
            }
        }
        Ok(Transaction {
            db: self.db,
            ops: Vec::new(),
            local_buffer: HashMap::new(),
            _state: PhantomData,
        })
    }

    /// 트랜잭션 롤백 — 모든 버퍼링된 쓰기를 폐기
    pub fn rollback(self) -> DbxResult<Transaction<'a, RolledBack>> {
        // ops 버퍼를 단순히 버림 — 메인 스토리지에는 아무것도 적용하지 않음
        Ok(Transaction {
            db: self.db,
            ops: Vec::new(),
            local_buffer: HashMap::new(),
            _state: PhantomData,
        })
    }
}

// Committed/RolledBack 상태에서는 쿼리 불가 (컴파일 에러)
impl<'a> Transaction<'a, Committed> {
    /// 커밋된 트랜잭션은 더 이상 사용할 수 없음
    pub fn is_committed(&self) -> bool {
        true
    }
}

impl<'a> Transaction<'a, RolledBack> {
    /// 롤백된 트랜잭션은 더 이상 사용할 수 없음
    pub fn is_rolled_back(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::Database;

    #[test]
    fn test_begin_commit() {
        let db = Database::open_in_memory().unwrap();
        let mut tx = db.begin().unwrap();

        tx.insert("users", b"u1", b"Alice").unwrap();
        tx.insert("users", b"u2", b"Bob").unwrap();

        // 커밋 전: 메인 스토리지에는 없음
        assert_eq!(db.get("users", b"u1").unwrap(), None);

        // 트랜잭션 내 read-your-writes
        assert_eq!(tx.get("users", b"u1").unwrap(), Some(b"Alice".to_vec()));

        // 커밋
        let committed = tx.commit().unwrap();
        assert!(committed.is_committed());

        // 커밋 후: 메인 스토리지에 반영됨
        assert_eq!(db.get("users", b"u1").unwrap(), Some(b"Alice".to_vec()));
        assert_eq!(db.get("users", b"u2").unwrap(), Some(b"Bob".to_vec()));
    }

    #[test]
    fn test_begin_rollback() {
        let db = Database::open_in_memory().unwrap();
        let mut tx = db.begin().unwrap();

        tx.insert("users", b"u1", b"Alice").unwrap();
        tx.insert("users", b"u2", b"Bob").unwrap();

        // 롤백
        let rolled_back = tx.rollback().unwrap();
        assert!(rolled_back.is_rolled_back());

        // 롤백 후: 메인 스토리지에 반영 안 됨
        assert_eq!(db.get("users", b"u1").unwrap(), None);
        assert_eq!(db.get("users", b"u2").unwrap(), None);
    }

    #[test]
    fn test_delete_in_transaction() {
        let db = Database::open_in_memory().unwrap();

        // 메인에 데이터 먼저 삽입
        db.insert("users", b"u1", b"Alice").unwrap();
        assert_eq!(db.get("users", b"u1").unwrap(), Some(b"Alice".to_vec()));

        // 트랜잭션에서 삭제
        let mut tx = db.begin().unwrap();
        tx.delete("users", b"u1").unwrap();

        // 트랜잭션 내에서 tombstone 확인
        assert_eq!(tx.get("users", b"u1").unwrap(), None);

        // 메인에는 아직 있음
        assert_eq!(db.get("users", b"u1").unwrap(), Some(b"Alice".to_vec()));

        // 커밋 후 삭제 반영
        tx.commit().unwrap();
        assert_eq!(db.get("users", b"u1").unwrap(), None);
    }

    #[test]
    fn test_read_your_writes() {
        let db = Database::open_in_memory().unwrap();

        // 메인에 초기 데이터
        db.insert("t", b"k1", b"old").unwrap();

        let mut tx = db.begin().unwrap();

        // 트랜잭션 내 덮어쓰기
        tx.insert("t", b"k1", b"new").unwrap();
        assert_eq!(tx.get("t", b"k1").unwrap(), Some(b"new".to_vec()));

        // 메인 데이터를 트랜잭션에서도 조회 가능 (로컬 버퍼에 없는 키)
        db.insert("t", b"k2", b"main_data").unwrap();
        assert_eq!(tx.get("t", b"k2").unwrap(), Some(b"main_data".to_vec()));

        tx.rollback().unwrap();
        // 롤백 후 메인 데이터 원복
        assert_eq!(db.get("t", b"k1").unwrap(), Some(b"old".to_vec()));
    }

    #[test]
    fn test_pending_ops_count() {
        let db = Database::open_in_memory().unwrap();
        let mut tx = db.begin().unwrap();

        assert_eq!(tx.pending_ops(), 0);
        tx.insert("t", b"a", b"1").unwrap();
        assert_eq!(tx.pending_ops(), 1);
        tx.delete("t", b"b").unwrap();
        assert_eq!(tx.pending_ops(), 2);
        tx.insert("t", b"c", b"3").unwrap();
        assert_eq!(tx.pending_ops(), 3);
    }

    #[test]
    fn test_empty_transaction_commit() {
        let db = Database::open_in_memory().unwrap();
        let tx = db.begin().unwrap();
        let committed = tx.commit().unwrap();
        assert!(committed.is_committed());
    }

    #[test]
    fn test_empty_transaction_rollback() {
        let db = Database::open_in_memory().unwrap();
        let tx = db.begin().unwrap();
        let rolled_back = tx.rollback().unwrap();
        assert!(rolled_back.is_rolled_back());
    }
}
