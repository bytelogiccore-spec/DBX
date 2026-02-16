// ═══════════════════════════════════════════════════════════════════════════
// 🔒 OFFICIAL CROSS-DATABASE BENCHMARK — DO NOT MODIFY
// ═══════════════════════════════════════════════════════════════════════════
//
// ⚠️ WARNING: This benchmark is the OFFICIAL reference for comparing DBX
// against other embedded databases (SQLite, Sled, Redb).
//
// 🚫 DO NOT MODIFY THIS FILE WITHOUT EXPLICIT APPROVAL
//
// Any changes to test parameters, data sizes, or methodology will invalidate
// historical comparisons and undermine benchmark credibility.
//
// If you need to add new tests, create a separate benchmark file.
//
// ═══════════════════════════════════════════════════════════════════════════

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use redb::{Database as RedbDatabase, ReadableTable, TableDefinition};
use rusqlite::Connection;
use tempfile::tempdir;

const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("bench");
const NUM_ENTRIES: usize = 10_000;

// ═══════════════════════════════════════════════════════════════════════════
// Test Data Generation
// ═══════════════════════════════════════════════════════════════════════════

fn generate_test_data() -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..NUM_ENTRIES)
        .map(|i| {
            let key = format!("key_{:08}", i).into_bytes();
            let value = format!("value_{:08}_data", i).into_bytes();
            (key, value)
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// DBX Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_dbx_insert(c: &mut Criterion) {
    let data = generate_test_data();
    c.bench_function("dbx_insert_10k", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            for (key, value) in &data {
                black_box(db.insert("bench", key, value).unwrap());
            }
        });
    });
}

fn bench_dbx_get(c: &mut Criterion) {
    let data = generate_test_data();
    let db = Database::open_in_memory().unwrap();
    for (key, value) in &data {
        db.insert("bench", key, value).unwrap();
    }

    c.bench_function("dbx_get_10k", |b| {
        b.iter(|| {
            for (key, _) in &data {
                black_box(db.get("bench", key).unwrap());
            }
        });
    });
}

fn bench_dbx_scan(c: &mut Criterion) {
    let data = generate_test_data();
    let db = Database::open_in_memory().unwrap();
    for (key, value) in &data {
        db.insert("bench", key, value).unwrap();
    }

    c.bench_function("dbx_scan_10k", |b| {
        b.iter(|| {
            black_box(db.scan("bench").unwrap());
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// SQLite Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_sqlite_insert(c: &mut Criterion) {
    let data = generate_test_data();
    c.bench_function("sqlite_insert_10k", |b| {
        b.iter(|| {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute("CREATE TABLE bench (key BLOB PRIMARY KEY, value BLOB)", [])
                .unwrap();
            for (key, value) in &data {
                conn.execute(
                    "INSERT INTO bench (key, value) VALUES (?1, ?2)",
                    [key, value],
                )
                .unwrap();
            }
        });
    });
}

fn bench_sqlite_get(c: &mut Criterion) {
    let data = generate_test_data();
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE bench (key BLOB PRIMARY KEY, value BLOB)", [])
        .unwrap();
    for (key, value) in &data {
        conn.execute(
            "INSERT INTO bench (key, value) VALUES (?1, ?2)",
            [key, value],
        )
        .unwrap();
    }

    c.bench_function("sqlite_get_10k", |b| {
        b.iter(|| {
            for (key, _) in &data {
                let value: Vec<u8> = conn
                    .query_row("SELECT value FROM bench WHERE key = ?1", [key], |row| {
                        row.get(0)
                    })
                    .unwrap();
                black_box(value);
            }
        });
    });
}

fn bench_sqlite_scan(c: &mut Criterion) {
    let data = generate_test_data();
    let conn = Connection::open_in_memory().unwrap();
    conn.execute("CREATE TABLE bench (key BLOB PRIMARY KEY, value BLOB)", [])
        .unwrap();
    for (key, value) in &data {
        conn.execute(
            "INSERT INTO bench (key, value) VALUES (?1, ?2)",
            [key, value],
        )
        .unwrap();
    }

    c.bench_function("sqlite_scan_10k", |b| {
        b.iter(|| {
            let mut stmt = conn.prepare("SELECT key, value FROM bench").unwrap();
            let rows: Vec<(Vec<u8>, Vec<u8>)> = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            black_box(rows);
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Sled Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_sled_insert(c: &mut Criterion) {
    let data = generate_test_data();
    c.bench_function("sled_insert_10k", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let db = sled::open(dir.path()).unwrap();
            for (key, value) in &data {
                black_box(db.insert(key, value.as_slice()).unwrap());
            }
        });
    });
}

fn bench_sled_get(c: &mut Criterion) {
    let data = generate_test_data();
    let dir = tempdir().unwrap();
    let db = sled::open(dir.path()).unwrap();
    for (key, value) in &data {
        db.insert(key, value.as_slice()).unwrap();
    }

    c.bench_function("sled_get_10k", |b| {
        b.iter(|| {
            for (key, _) in &data {
                black_box(db.get(key).unwrap());
            }
        });
    });
}

fn bench_sled_scan(c: &mut Criterion) {
    let data = generate_test_data();
    let dir = tempdir().unwrap();
    let db = sled::open(dir.path()).unwrap();
    for (key, value) in &data {
        db.insert(key, value.as_slice()).unwrap();
    }

    c.bench_function("sled_scan_10k", |b| {
        b.iter(|| {
            let results: Vec<_> = db.iter().map(|r| r.unwrap()).collect();
            black_box(results);
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Redb Benchmarks
// ═══════════════════════════════════════════════════════════════════════════

fn bench_redb_insert(c: &mut Criterion) {
    let data = generate_test_data();
    c.bench_function("redb_insert_10k", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let db = RedbDatabase::create(dir.path().join("bench.redb")).unwrap();
            let write_txn = db.begin_write().unwrap();
            {
                let mut table = write_txn.open_table(TABLE).unwrap();
                for (key, value) in &data {
                    black_box(table.insert(key.as_slice(), value.as_slice()).unwrap());
                }
            }
            write_txn.commit().unwrap();
        });
    });
}

fn bench_redb_get(c: &mut Criterion) {
    let data = generate_test_data();
    let dir = tempdir().unwrap();
    let db = RedbDatabase::create(dir.path().join("bench.redb")).unwrap();
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).unwrap();
        for (key, value) in &data {
            table.insert(key.as_slice(), value.as_slice()).unwrap();
        }
    }
    write_txn.commit().unwrap();

    c.bench_function("redb_get_10k", |b| {
        b.iter(|| {
            let read_txn = db.begin_read().unwrap();
            let table = read_txn.open_table(TABLE).unwrap();
            for (key, _) in &data {
                black_box(table.get(key.as_slice()).unwrap());
            }
        });
    });
}

fn bench_redb_scan(c: &mut Criterion) {
    let data = generate_test_data();
    let dir = tempdir().unwrap();
    let db = RedbDatabase::create(dir.path().join("bench.redb")).unwrap();
    let write_txn = db.begin_write().unwrap();
    {
        let mut table = write_txn.open_table(TABLE).unwrap();
        for (key, value) in &data {
            table.insert(key.as_slice(), value.as_slice()).unwrap();
        }
    }
    write_txn.commit().unwrap();

    c.bench_function("redb_scan_10k", |b| {
        b.iter(|| {
            let read_txn = db.begin_read().unwrap();
            let table = read_txn.open_table(TABLE).unwrap();
            let results: Vec<_> = table.iter().unwrap().map(|r| r.unwrap()).collect();
            black_box(results);
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// Criterion Groups
// ═══════════════════════════════════════════════════════════════════════════

criterion_group!(dbx_benches, bench_dbx_insert, bench_dbx_get, bench_dbx_scan);

criterion_group!(
    sqlite_benches,
    bench_sqlite_insert,
    bench_sqlite_get,
    bench_sqlite_scan
);

criterion_group!(
    sled_benches,
    bench_sled_insert,
    bench_sled_get,
    bench_sled_scan
);

criterion_group!(
    redb_benches,
    bench_redb_insert,
    bench_redb_get,
    bench_redb_scan
);

criterion_main!(dbx_benches, sqlite_benches, sled_benches, redb_benches);
