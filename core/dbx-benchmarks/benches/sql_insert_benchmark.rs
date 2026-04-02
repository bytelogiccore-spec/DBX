//! SQL INSERT vs KV API Performance Benchmark
//!
//! Compares the performance of SQL INSERT statements against direct KV API calls.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;

/// Generate SQL INSERT statement with multiple rows
fn generate_multi_row_insert(count: usize) -> String {
    let mut sql = String::from("INSERT INTO users (id, name, email) VALUES ");
    for i in 0..count {
        if i > 0 {
            sql.push_str(", ");
        }
        sql.push_str(&format!("({}, 'user{}', 'user{}@example.com')", i, i, i));
    }
    sql
}

/// Generate KV batch data
fn generate_kv_batch(count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..count)
        .map(|i| {
            let key = i.to_le_bytes().to_vec();
            let value =
                format!(r#"{{"name":"user{}","email":"user{}@example.com"}}"#, i, i).into_bytes();
            (key, value)
        })
        .collect()
}

/// Benchmark: Single row INSERT
fn bench_single_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_insert");

    // SQL INSERT
    group.bench_function("sql", |b| {
        let db = Database::open_in_memory().unwrap();
        b.iter(|| {
            let result = db.execute_sql(black_box(
                "INSERT INTO users (id, name, email) VALUES (1, 'Alice', 'alice@example.com')",
            ));
            black_box(result)
        });
    });

    // KV API
    group.bench_function("kv", |b| {
        let db = Database::open_in_memory().unwrap();
        b.iter(|| {
            let result = db.insert(
                black_box("users"),
                black_box(&1u64.to_le_bytes()),
                black_box(br#"{"name":"Alice","email":"alice@example.com"}"#),
            );
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark: Batch INSERT with varying sizes
fn bench_batch_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_insert");
    group.sample_size(20); // Reduce sample size for large batches

    for size in [100, 1000, 10000].iter() {
        // SQL INSERT (multi-row)
        group.bench_with_input(BenchmarkId::new("sql", size), size, |b, &size| {
            let db = Database::open_in_memory().unwrap();
            let sql = generate_multi_row_insert(size);
            b.iter(|| {
                let result = db.execute_sql(black_box(&sql));
                black_box(result)
            });
        });

        // KV API (batch)
        group.bench_with_input(BenchmarkId::new("kv", size), size, |b, &size| {
            let db = Database::open_in_memory().unwrap();
            let rows = generate_kv_batch(size);
            b.iter(|| {
                let result = db.insert_batch(black_box("users"), black_box(rows.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Throughput test with large batches
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.sample_size(10);

    // SQL INSERT (100K rows)
    group.bench_function("sql_100k", |b| {
        let db = Database::open_in_memory().unwrap();
        let sql = generate_multi_row_insert(100_000);
        b.iter(|| {
            let result = db.execute_sql(black_box(&sql));
            black_box(result)
        });
    });

    // KV API (100K rows)
    group.bench_function("kv_100k", |b| {
        let db = Database::open_in_memory().unwrap();
        let rows = generate_kv_batch(100_000);
        b.iter(|| {
            let result = db.insert_batch(black_box("users"), black_box(rows.clone()));
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_insert,
    bench_batch_insert,
    bench_throughput
);
criterion_main!(benches);
