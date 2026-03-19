//! dirty 버퍼 모드 전환 테스트
//!
//! BTreeMap ↔ DashMap 전환 시 데이터 정합성, WAL replay, 범위 쿼리 검증.

use dbx_core::engine::DirtyBufferMode;
use dbx_core::storage::StorageBackend;
use dbx_core::storage::native_wos::backend::NativeWosBackend;
use tempfile::tempdir;

// ──────────────────────────────────────────────
// 헬퍼
// ──────────────────────────────────────────────

fn backend_with_mode(mode: DirtyBufferMode) -> (NativeWosBackend, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let b = NativeWosBackend::open_with_mode(dir.path(), mode).unwrap();
    (b, dir)
}

fn backend_reopen_with_mode(dir: &tempfile::TempDir, mode: DirtyBufferMode) -> NativeWosBackend {
    NativeWosBackend::open_with_mode(dir.path(), mode).unwrap()
}

// ──────────────────────────────────────────────
// 기본 동작: BTreeMap 모드
// ──────────────────────────────────────────────

#[test]
fn btree_basic_insert_get() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::BTreeMap);
    b.insert("t", b"k1", b"v1").unwrap();
    assert_eq!(b.get("t", b"k1").unwrap(), Some(b"v1".to_vec()));
}

#[test]
fn btree_basic_delete() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::BTreeMap);
    b.insert("t", b"k1", b"v1").unwrap();
    assert!(b.delete("t", b"k1").unwrap());
    assert_eq!(b.get("t", b"k1").unwrap(), None);
}

// ──────────────────────────────────────────────
// 기본 동작: DashMap 모드
// ──────────────────────────────────────────────

#[test]
fn dashmap_basic_insert_get() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::DashMap);
    b.insert("t", b"k1", b"v1").unwrap();
    assert_eq!(b.get("t", b"k1").unwrap(), Some(b"v1".to_vec()));
}

#[test]
fn dashmap_basic_delete() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::DashMap);
    b.insert("t", b"k1", b"v1").unwrap();
    assert!(b.delete("t", b"k1").unwrap());
    assert_eq!(b.get("t", b"k1").unwrap(), None);
}

// ──────────────────────────────────────────────
// 범위 스캔: 두 모드 모두 동일 결과
// ──────────────────────────────────────────────

#[test]
fn btree_scan_range() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::BTreeMap);
    b.insert("t", b"a", b"1").unwrap();
    b.insert("t", b"b", b"2").unwrap();
    b.insert("t", b"c", b"3").unwrap();
    b.insert("t", b"d", b"4").unwrap();
    let res = b.scan("t", b"b".to_vec()..b"d".to_vec()).unwrap();
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].0, b"b");
    assert_eq!(res[1].0, b"c");
}

#[test]
fn dashmap_scan_range() {
    let (b, _dir) = backend_with_mode(DirtyBufferMode::DashMap);
    b.insert("t", b"a", b"1").unwrap();
    b.insert("t", b"b", b"2").unwrap();
    b.insert("t", b"c", b"3").unwrap();
    b.insert("t", b"d", b"4").unwrap();
    let res = b.scan("t", b"b".to_vec()..b"d".to_vec()).unwrap();
    // DashMap도 범위 쿼리 결과가 동일해야 함 (내부적으로 정렬됨)
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].0, b"b");
    assert_eq!(res[1].0, b"c");
}

// ──────────────────────────────────────────────
// 전환 테스트: BTreeMap → DashMap
// ──────────────────────────────────────────────

#[test]
fn btree_to_dashmap_switch_after_flush() {
    let dir = tempdir().unwrap();

    // 1) BTreeMap 모드로 데이터 저장 + flush (WAL에 기록)
    {
        let b = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::BTreeMap).unwrap();
        b.insert("t", b"k1", b"v1").unwrap();
        b.insert("t", b"k2", b"v2").unwrap();
        b.flush().unwrap();
    }

    // 2) DashMap 모드로 재오픈 → WAL replay 후 데이터 동일해야 함
    let b2 = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::DashMap).unwrap();
    assert_eq!(b2.get("t", b"k1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(b2.get("t", b"k2").unwrap(), Some(b"v2".to_vec()));
}

// ──────────────────────────────────────────────
// 전환 테스트: DashMap → BTreeMap
// ──────────────────────────────────────────────

#[test]
fn dashmap_to_btree_switch_after_flush() {
    let dir = tempdir().unwrap();

    // 1) DashMap 모드로 데이터 저장 + flush
    {
        let b = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::DashMap).unwrap();
        b.insert("t", b"k1", b"v1").unwrap();
        b.insert("t", b"k2", b"v2").unwrap();
        b.flush().unwrap();
    }

    // 2) BTreeMap 모드로 재오픈 → 데이터 동일
    let b2 = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::BTreeMap).unwrap();
    assert_eq!(b2.get("t", b"k1").unwrap(), Some(b"v1".to_vec()));
    assert_eq!(b2.get("t", b"k2").unwrap(), Some(b"v2".to_vec()));
}

// ──────────────────────────────────────────────
// WAL replay: 두 모드 모두 재시작 후 복구 확인
// ──────────────────────────────────────────────

#[test]
fn wal_replay_btree_mode() {
    let dir = tempdir().unwrap();
    {
        let b = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::BTreeMap).unwrap();
        b.insert("t", b"key", b"val").unwrap();
        b.flush().unwrap();
    }
    let b2 = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::BTreeMap).unwrap();
    assert_eq!(b2.get("t", b"key").unwrap(), Some(b"val".to_vec()));
}

#[test]
fn wal_replay_dashmap_mode() {
    let dir = tempdir().unwrap();
    {
        let b = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::DashMap).unwrap();
        b.insert("t", b"key", b"val").unwrap();
        b.flush().unwrap();
    }
    let b2 = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::DashMap).unwrap();
    assert_eq!(b2.get("t", b"key").unwrap(), Some(b"val".to_vec()));
}

// ──────────────────────────────────────────────
// 전환 후 범위 스캔도 정상 동작 확인
// ──────────────────────────────────────────────

#[test]
fn switch_mode_scan_consistency() {
    let dir = tempdir().unwrap();

    // BTreeMap으로 저장 + flush
    {
        let b = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::BTreeMap).unwrap();
        for i in 0u8..10 {
            b.insert("t", &[i], &[i * 10]).unwrap();
        }
        b.flush().unwrap();
    }

    // DashMap으로 재오픈 후 scan
    let b2 = NativeWosBackend::open_with_mode(dir.path(), DirtyBufferMode::DashMap).unwrap();
    let all = b2.scan("t", ..).unwrap();
    assert_eq!(all.len(), 10);
    // 결과가 정렬되어 있는지 확인
    for i in 0u8..10 {
        assert_eq!(all[i as usize].0, vec![i]);
        assert_eq!(all[i as usize].1, vec![i * 10]);
    }
}
