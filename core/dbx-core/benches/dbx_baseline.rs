// DBX Baseline Benchmark - 리팩토링 전후 성능 비교용
//
// 사용법:
// cargo bench --bench dbx_baseline

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use std::sync::Arc;

const SMALL_SIZE: usize = 100;
const MEDIUM_SIZE: usize = 1_000;
const LARGE_SIZE: usize = 10_000;

// ════════════════════════════════════════════
// DBX Benchmarks
// ════════════════════════════════════════════

fn bench_dbx_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("dbx_insert");

    for size in [SMALL_SIZE, MEDIUM_SIZE, LARGE_SIZE] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let db = Database::open_in_memory().unwrap();
                for i in 0..size {
                    let key = format!("key_{}", i);
                    let value = format!("value_{}", i);
                    db.insert("bench", key.as_bytes(), value.as_bytes())
                        .unwrap();
                }
            });
        });
    }
    group.finish();
}

fn bench_dbx_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("dbx_get");

    for size in [SMALL_SIZE, MEDIUM_SIZE, LARGE_SIZE] {
        // Setup: pre-populate database
        let db = Arc::new(Database::open_in_memory().unwrap());
        for i in 0..size {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            db.insert("bench", key.as_bytes(), value.as_bytes())
                .unwrap();
        }

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let db = Arc::clone(&db);
            b.iter(|| {
                for i in 0..size {
                    let key = format!("key_{}", i);
                    let _ = black_box(db.get("bench", key.as_bytes()).unwrap());
                }
            });
        });
    }
    group.finish();
}

fn bench_dbx_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("dbx_scan");

    for size in [SMALL_SIZE, MEDIUM_SIZE, LARGE_SIZE] {
        // Setup: pre-populate database
        let db = Arc::new(Database::open_in_memory().unwrap());
        for i in 0..size {
            let key = format!("key_{:05}", i);
            let value = format!("value_{}", i);
            db.insert("bench", key.as_bytes(), value.as_bytes())
                .unwrap();
        }

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let db = Arc::clone(&db);
            b.iter(|| {
                let results = db.scan("bench").unwrap();
                black_box(results);
            });
        });
    }
    group.finish();
}

fn bench_dbx_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("dbx_mixed_workload");

    for size in [SMALL_SIZE, MEDIUM_SIZE, LARGE_SIZE] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let db = Database::open_in_memory().unwrap();

                // 70% Insert
                for i in 0..(size * 7 / 10) {
                    let key = format!("key_{}", i);
                    let value = format!("value_{}", i);
                    db.insert("bench", key.as_bytes(), value.as_bytes())
                        .unwrap();
                }

                // 20% Get
                for i in 0..(size * 2 / 10) {
                    let key = format!("key_{}", i);
                    let _ = black_box(db.get("bench", key.as_bytes()).unwrap());
                }

                // 10% Delete
                for i in 0..(size / 10) {
                    let key = format!("key_{}", i);
                    db.delete("bench", key.as_bytes()).unwrap();
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_dbx_insert,
    bench_dbx_get,
    bench_dbx_scan,
    bench_dbx_mixed_workload
);
criterion_main!(benches);
