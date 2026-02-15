use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dbx_core::engine::{ParallelExecutionEngine, ParallelizationPolicy};
use dbx_core::sql::ParallelSqlParser;
use rayon::prelude::*;
use std::sync::Arc;

// ════════════════════════════════════════════
// Parallel Execution Engine Benchmarks
// ════════════════════════════════════════════

fn bench_parallel_engine_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_engine_policies");

    // Test different policies with varying workload sizes
    for size in [100, 1_000, 10_000, 100_000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        // Auto policy
        group.bench_with_input(BenchmarkId::new("auto", size), size, |b, &size| {
            let engine = ParallelExecutionEngine::new_auto().unwrap();
            b.iter(|| {
                engine.execute(|| {
                    (0..size)
                        .into_par_iter()
                        .map(|i| black_box(i * 2))
                        .sum::<usize>()
                })
            });
        });

        // Fixed policy (4 threads)
        group.bench_with_input(BenchmarkId::new("fixed_4", size), size, |b, &size| {
            let engine = ParallelExecutionEngine::new_fixed(4).unwrap();
            b.iter(|| {
                engine.execute(|| {
                    (0..size)
                        .into_par_iter()
                        .map(|i| black_box(i * 2))
                        .sum::<usize>()
                })
            });
        });

        // Sequential baseline
        group.bench_with_input(BenchmarkId::new("sequential", size), size, |b, &size| {
            b.iter(|| (0..size).map(|i| black_box(i * 2)).sum::<usize>());
        });
    }

    group.finish();
}

fn bench_auto_tune(c: &mut Criterion) {
    let mut group = c.benchmark_group("auto_tune");
    let engine = ParallelExecutionEngine::new_auto().unwrap();

    for size in [100, 1_000, 10_000, 100_000, 1_000_000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| engine.auto_tune(black_box(size)));
        });
    }

    group.finish();
}

// ════════════════════════════════════════════
// Parallel SQL Parser Benchmarks
// ════════════════════════════════════════════

fn bench_parallel_sql_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_sql_parser");

    // Generate test SQL statements
    let sqls: Vec<String> = (0..100)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i, i))
        .collect();
    let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();

    // Test different batch sizes
    for batch_size in [1, 5, 10, 20, 50, 100].iter() {
        let batch = &sql_refs[..*batch_size];
        group.throughput(Throughput::Elements(*batch_size as u64));

        // Parallel parsing
        group.bench_with_input(
            BenchmarkId::new("parallel", batch_size),
            batch,
            |b, batch| {
                let parser = ParallelSqlParser::new();
                b.iter(|| parser.parse_batch(black_box(batch)).unwrap());
            },
        );

        // Sequential parsing (baseline)
        group.bench_with_input(
            BenchmarkId::new("sequential", batch_size),
            batch,
            |b, batch| {
                let parser = ParallelSqlParser::new();
                b.iter(|| {
                    batch
                        .iter()
                        .map(|sql| parser.parse(sql).unwrap())
                        .collect::<Vec<_>>()
                });
            },
        );
    }

    group.finish();
}

fn bench_parallel_parser_with_custom_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_parser_custom_pool");

    let sqls: Vec<String> = (0..50)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i, i))
        .collect();
    let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();

    // Test different thread pool sizes
    for num_threads in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .build()
                    .unwrap();
                let parser = ParallelSqlParser::with_thread_pool(Arc::new(pool));
                b.iter(|| parser.parse_batch(black_box(&sql_refs)).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_parallel_parser_partial(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_parser_partial");

    // Mix of valid and invalid SQL
    let sqls = vec![
        "SELECT * FROM users",
        "SELECT * FROM orders",
        "INVALID SQL HERE",
        "SELECT * FROM products",
        "ANOTHER INVALID",
        "SELECT * FROM categories",
    ];

    group.bench_function("parse_batch_partial", |b| {
        let parser = ParallelSqlParser::new();
        b.iter(|| parser.parse_batch_partial(black_box(&sqls)));
    });

    group.finish();
}

// ════════════════════════════════════════════
// Integration Benchmarks
// ════════════════════════════════════════════

fn bench_parallel_engine_with_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("integration");

    let sqls: Vec<String> = (0..20)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i, i))
        .collect();
    let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();

    group.bench_function("parallel_engine_with_parser", |b| {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        let parser = ParallelSqlParser::new();
        b.iter(|| {
            engine.execute(|| parser.parse_batch(black_box(&sql_refs)).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parallel_engine_policies,
    bench_auto_tune,
    bench_parallel_sql_parser,
    bench_parallel_parser_with_custom_pool,
    bench_parallel_parser_partial,
    bench_parallel_engine_with_parser,
);

criterion_main!(benches);
