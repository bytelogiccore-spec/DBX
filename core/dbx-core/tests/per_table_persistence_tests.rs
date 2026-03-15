//! Per-table persistence tests — 한 DB에서 메모리 테이블 + 파일 테이블 동시 사용
//!
//! path 기반 시나리오가 먼저 실행되는지 검증하려면 `--test-threads=1`로 실행하세요.
//! (기본 병렬 실행에서는 test_00_ 접두사만으로 순서가 보장되지 않음)

use dbx_core::engine::TablePersistence;
use dbx_core::Database;
use tempfile::tempdir;

/// 한 DB에서 Memory 테이블과 File 테이블을 지정하고 insert/get 동작을 검증.
/// test_00_ 접두사로, 단일 스레드 실행 시(--test-threads=1) 맨 먼저 실행되도록 하여
/// Database::open(path)만 단독으로 호출해도 insert/get이 정상 동작함을 검증 (open_in_memory 선행 호출에 의존하지 않음).
#[test]
fn test_00_per_table_persistence_set_and_query() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let db = Database::open(path).unwrap();
    db.set_table_persistence("temp", TablePersistence::Memory).unwrap();
    db.set_table_persistence("persistent", TablePersistence::File).unwrap();

    assert_eq!(db.table_persistence("temp"), Some(TablePersistence::Memory));
    assert_eq!(db.table_persistence("persistent"), Some(TablePersistence::File));
    assert_eq!(db.table_persistence("unknown"), None);

    db.insert("temp", b"k", b"v1").unwrap();
    db.insert("persistent", b"k", b"v2").unwrap();

    assert_eq!(db.get("temp", b"k").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(db.get("persistent", b"k").unwrap(), Some(b"v2".to_vec()));
}

/// 인메모리 DB에서 insert/get (per-table 설정 없이 동작 확인).
#[test]
fn test_per_table_persistence_open_in_memory_insert_get() {
    let db = Database::open_in_memory().unwrap();
    db.insert("t", b"k", b"v").unwrap();
    assert_eq!(db.get("t", b"k").unwrap(), Some(b"v".to_vec()));
}

/// 인메모리 DB에서는 TablePersistence::File 지정 시 에러.
#[test]
fn test_open_in_memory_has_no_file_persistence() {
    let db = Database::open_in_memory().unwrap();
    // Setting File on in-memory DB should error
    let err = db.set_table_persistence("t", TablePersistence::File).unwrap_err();
    assert!(err.to_string().contains("file-backed") || err.to_string().contains("path"));
}
