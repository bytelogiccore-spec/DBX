//! SQL UPDATE/DELETE Performance Benchmark
//!
//! Benchmarks UPDATE and DELETE operations with various WHERE clauses

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;

/// Setup database with test data
fn setup_db_with_data(count: usize) -> Database {
    let db = Database::open_in_memory().unwrap();

    // Insert test data
    for i in 0..count {
        let sql = format!(
            "INSERT INTO users (id, name, age) VALUES ({}, 'user{}', {})",
            i,
            i,
            20 + (i % 50) // Ages from 20 to 69
        );
        db.execute_sql(&sql).unwrap();
    }

    db
}

/// Benchmark: UPDATE all records
fn bench_update_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_all");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    let result = db.execute_sql(black_box("UPDATE users SET age = 50"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: UPDATE with WHERE = condition (10% of records)
fn bench_update_where_eq(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_where_eq");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Update ~10% of records (age = 25)
                    let result =
                        db.execute_sql(black_box("UPDATE users SET age = 26 WHERE age = 25"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: UPDATE with WHERE > condition (50% of records)
fn bench_update_where_gt(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_where_gt");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Update ~50% of records (age > 45)
                    let result =
                        db.execute_sql(black_box("UPDATE users SET age = 50 WHERE age > 45"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: UPDATE with WHERE AND condition
fn bench_update_where_and(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_where_and");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Update records with AND condition
                    let result = db.execute_sql(black_box(
                        "UPDATE users SET age = 30 WHERE age > 25 AND age < 35",
                    ));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: DELETE all records
fn bench_delete_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_all");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    let result = db.execute_sql(black_box("DELETE FROM users"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: DELETE with WHERE = condition (10% of records)
fn bench_delete_where_eq(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_where_eq");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Delete ~10% of records (age = 25)
                    let result = db.execute_sql(black_box("DELETE FROM users WHERE age = 25"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: DELETE with WHERE < condition (30% of records)
fn bench_delete_where_lt(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_where_lt");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Delete ~30% of records (age < 35)
                    let result = db.execute_sql(black_box("DELETE FROM users WHERE age < 35"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark: DELETE with WHERE OR condition
fn bench_delete_where_or(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_where_or");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || setup_db_with_data(size),
                |db| {
                    // Delete records with OR condition
                    let result =
                        db.execute_sql(black_box("DELETE FROM users WHERE age < 25 OR age > 65"));
                    black_box(result)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_update_all,
    bench_update_where_eq,
    bench_update_where_gt,
    bench_update_where_and,
    bench_delete_all,
    bench_delete_where_eq,
    bench_delete_where_lt,
    bench_delete_where_or
);
criterion_main!(benches);
