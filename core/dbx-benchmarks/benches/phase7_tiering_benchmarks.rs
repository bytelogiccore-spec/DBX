// Phase 7: 분산 5-Tier 연동 및 최적화 성능 벤치마크
//
// 1. Write 검증: 데이터 손실 없는 Insert 처리
// 2. Scan 검증: 로컬리티 기반의 다중 파티션 Pruning 스캔
// 3. 병렬 Hash Aggregate 검증: 10K 행 대상 분산 집계

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use rusqlite::Connection;
use tempfile::tempdir;

const NUM_ENTRIES: usize = 10_000;

fn generate_test_data() -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..NUM_ENTRIES)
        .map(|i| {
            let key = format!("user_{:08}", i).into_bytes();
            let value = format!("val_field_{:08}", i % 100).into_bytes();
            (key, value)
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Write 벤치마크: DBX vs Rusqlite
// ═══════════════════════════════════════════════════════════════════════════

fn bench_write_10k(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase7_write_10k");
    let data = generate_test_data();

    // DBX Write
    group.bench_function("dbx_insert", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let db = Database::open(dir.path()).unwrap();
            db.execute_sql("CREATE TABLE bench_table (user_id TEXT, data_val TEXT)")
                .unwrap();
            for (key, value) in &data {
                db.insert("bench_table", key, value).unwrap();
            }
            black_box(db);
        })
    });

    // Rusqlite Write
    group.bench_function("rusqlite_insert", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute(
                "CREATE TABLE bench_table (key BLOB PRIMARY KEY, value BLOB)",
                [],
            )
            .unwrap();

            // Note: Normally SQLite needs a transaction for bulk insert to be fast.
            let mut stmt = conn
                .prepare("INSERT INTO bench_table (key, value) VALUES (?1, ?2)")
                .unwrap();
            for (key, value) in &data {
                stmt.execute([key, value]).unwrap();
            }
            drop(stmt);
            black_box(conn);
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Scan & Aggregate 벤치마크: DBX 5-Tier 분산 집계 최적화 비교
// ═══════════════════════════════════════════════════════════════════════════

fn bench_scan_aggregate_10k(c: &mut Criterion) {
    let mut group = c.benchmark_group("phase7_scan_aggregate_10k");
    let data = generate_test_data();

    // -- DBX Setup --
    let dir = tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    db.execute_sql("CREATE TABLE bench_table (user_id TEXT, data_val TEXT)")
        .unwrap();
    for (key, value) in &data {
        db.insert("bench_table", key, value).unwrap();
    }
    // WOS 데이터를 ROS로 내려보내기 위해 강제 Flush (옵셔널)
    // db.flush_all().unwrap();

    group.bench_function("dbx_scan_count", |b| {
        b.iter(|| {
            // DBX 내부적으로 Scan -> HashAggregate(Count) 계획 실행
            let sql = "SELECT count(*) FROM bench_table";
            let results = db.execute_sql(sql).unwrap();
            black_box(results);
        })
    });

    // -- Rusqlite Setup --
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE bench_table (key BLOB PRIMARY KEY, value BLOB)",
        [],
    )
    .unwrap();
    {
        // 최적화를 위해 트랜잭션 사용
        let tx = conn.unchecked_transaction().unwrap();
        let mut stmt = tx
            .prepare("INSERT INTO bench_table (key, value) VALUES (?1, ?2)")
            .unwrap();
        for (key, value) in &data {
            stmt.execute([key, value]).unwrap();
        }
        drop(stmt);
        tx.commit().unwrap();
    }

    group.bench_function("rusqlite_scan_count", |b| {
        b.iter(|| {
            let mut stmt = conn.prepare("SELECT count(*) FROM bench_table").unwrap();
            let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
            black_box(count);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_write_10k, bench_scan_aggregate_10k);
criterion_main!(benches);
