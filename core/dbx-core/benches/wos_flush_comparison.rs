// WOS Flush 방식 비교 벤치마크
//
// 시나리오:
//   1. 10k 행을 미리 flush한 DB 준비 (공통)
//   2. 1k 행 추가 후 flush 시간 측정
//
// 비교 대상:
//   A. current_flush: 전체 읽기 + 재작성 — Database API 사용
//   B. wal_flush:     dirty entries만 WAL 파일에 sequential append
//   C. wal_compact:   WAL + SSTable 병합 (flush와 동일 비용, 발생 빈도가 다름)

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use tempfile::tempdir;
use std::fs::OpenOptions;
use std::io::Write;

const BASE_ROWS: usize = 10_000;
const FLUSH_ROWS: usize = 1_000;

fn gen_kv(i: usize) -> (Vec<u8>, Vec<u8>) {
    (
        format!("key_{:08}", i).into_bytes(),
        // 약 40 bytes value — 실제 데이터 크기 시뮬레이션
        format!("val_{:08}_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx", i).into_bytes(),
    )
}

// ─────────────────────────────────────────────────────────
// WAL 레코드 직렬화 (CRC32 포함)
// [key_len:4][key][val_len:4][val][deleted:1][crc32:4]
// ─────────────────────────────────────────────────────────
fn wal_encode(key: &[u8], val: &[u8], deleted: bool) -> Vec<u8> {
    let mut buf = Vec::with_capacity(4 + key.len() + 4 + val.len() + 1 + 4);
    buf.extend_from_slice(&(key.len() as u32).to_le_bytes());
    buf.extend_from_slice(key);
    buf.extend_from_slice(&(val.len() as u32).to_le_bytes());
    buf.extend_from_slice(val);
    buf.push(deleted as u8);
    let crc = crc32fast::hash(&buf);
    buf.extend_from_slice(&crc.to_le_bytes());
    buf
}

// ─────────────────────────────────────────────────────────
// A. 현재 방식: full-rewrite flush
// 동작: 기존 N 페이지 전부 읽기 + dirty 병합 + 파일 전체 재작성
// 복잡도: O(N + dirty) read + O(N + dirty) write
// ─────────────────────────────────────────────────────────
fn bench_current_flush(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("bench.dbx");

    // 기반 데이터 10k 삽입 + flush
    let db = Database::open(&db_path).unwrap();
    for i in 0..BASE_ROWS {
        let (k, v) = gen_kv(i);
        db.insert("bench", &k, &v).unwrap();
    }
    db.flush().unwrap(); // 10k를 SSTable로 flush

    c.bench_function("A_current_flush_1k_on_10k_base", |b| {
        b.iter(|| {
            // 1k dirty 추가
            for i in BASE_ROWS..BASE_ROWS + FLUSH_ROWS {
                let (k, v) = gen_kv(i);
                db.insert("bench", &k, &v).unwrap();
            }
            // flush: 기존 10k 읽기 + 1k merge + 11k 전체 재작성
            black_box(db.flush().unwrap());
        });
    });
}

// ─────────────────────────────────────────────────────────
// B. WAL flush: sequential append only
// 동작: dirty 1k entries를 .wal 파일에 순차 append
// 복잡도: O(dirty) write only — read 없음, SSTable 건드리지 않음
// ─────────────────────────────────────────────────────────
fn bench_wal_flush(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("bench.wal");

    // WAL flush: 기존 SSTable은 건드리지 않으므로 setup 불필요
    // (WAL 방식의 핵심: flush 시 기존 데이터를 읽지 않음)

    c.bench_function("B_wal_flush_1k_append_only", |b| {
        b.iter(|| {
            // 1k dirty entries를 WAL 파일에 sequential append
            let mut wal = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&wal_path)
                .unwrap();

            for i in BASE_ROWS..BASE_ROWS + FLUSH_ROWS {
                let (k, v) = gen_kv(i);
                let record = wal_encode(&k, &v, false);
                wal.write_all(&record).unwrap();
            }
            // sync (flush 완료)
            black_box(wal.sync_all().unwrap());
        });
    });
}

// ─────────────────────────────────────────────────────────
// C. WAL compact: WAL + SSTable 병합 (full merge)
// 동작: 현재 flush()와 동일한 비용
// 차이: WAL 방식에서는 이 연산이 매우 드물게 발생 (WAL > threshold 시)
// ─────────────────────────────────────────────────────────
fn bench_wal_compact(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("bench.dbx");

    let db = Database::open(&db_path).unwrap();
    for i in 0..BASE_ROWS {
        let (k, v) = gen_kv(i);
        db.insert("bench", &k, &v).unwrap();
    }
    db.flush().unwrap();

    // 1k dirty 미리 준비
    for i in BASE_ROWS..BASE_ROWS + FLUSH_ROWS {
        let (k, v) = gen_kv(i);
        db.insert("bench", &k, &v).unwrap();
    }

    c.bench_function("C_wal_compact_same_as_current_flush", |b| {
        b.iter(|| {
            // compact = 현재 flush()와 동일 비용 (O(N) read + O(N) write)
            black_box(db.flush().unwrap());

            // 다음 iter를 위해 dirty 복구
            for i in BASE_ROWS..BASE_ROWS + FLUSH_ROWS {
                let (k, v) = gen_kv(i);
                db.insert("bench", &k, &v).unwrap();
            }
        });
    });
}

criterion_group!(
    flush_benches,
    bench_current_flush,
    bench_wal_flush,
    bench_wal_compact,
);
criterion_main!(flush_benches);
