// Phase 1 성능 벤치마크 (간소화 버전)
//
// Section 1: 2단계 캐싱 시스템 (L1 vs L2 성능 비교)
// Section 2: 병렬 SQL 파싱 (단일 vs 배치 성능 비교)
// Section 3: MVCC 버전 관리 (add_version, get_at_snapshot 성능)

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::engine::{ParallelExecutionEngine, TwoLevelCache};
use dbx_core::sql::ParallelSqlParser;
use dbx_core::transaction::{TimestampOracle, VersionManager};
use std::path::PathBuf;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// Section 1: 2단계 캐싱 시스템 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_two_level_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_level_cache");

    // L1 캐시 벤치마크 (메모리)
    let cache_l1 = TwoLevelCache::new(10 * 1024 * 1024, PathBuf::from("target/bench_cache_l1"));
    let _ = cache_l1.clear();

    group.bench_function("l1_put_1kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 1024];
            cache_l1.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });

    // L1 캐시에 데이터 미리 저장
    for i in 0..100 {
        cache_l1.put(format!("key_{}", i), vec![0u8; 1024]).unwrap();
    }

    group.bench_function("l1_get_1kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 100);
            let _ = cache_l1.get(black_box(&key)).unwrap();
            counter += 1;
        })
    });

    // L2 캐시 벤치마크 (디스크)
    let cache_l2 = TwoLevelCache::new(10, PathBuf::from("target/bench_cache_l2")); // 작은 L1으로 L2 강제
    let _ = cache_l2.clear();

    group.bench_function("l2_put_1kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 1024];
            cache_l2.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });

    group.finish();

    // 정리
    let _ = cache_l1.clear();
    let _ = cache_l2.clear();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2: 병렬 SQL 파싱 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_parallel_sql_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_sql_parser");

    let parser = ParallelSqlParser::new();

    // 단일 SQL 파싱 (베이스라인)
    group.bench_function("single_parse", |b| {
        b.iter(|| {
            parser
                .parse(black_box("SELECT * FROM users WHERE id = 1"))
                .unwrap()
        })
    });

    // 배치 SQL 파싱 (목표: 8-10x)
    let sqls_10: Vec<&str> = vec![
        "SELECT * FROM users WHERE id = 1",
        "SELECT * FROM orders WHERE user_id = 1",
        "SELECT * FROM products WHERE category = 'electronics'",
        "SELECT * FROM reviews WHERE product_id = 1",
        "SELECT * FROM payments WHERE order_id = 1",
        "SELECT * FROM shipping WHERE order_id = 1",
        "SELECT * FROM inventory WHERE product_id = 1",
        "SELECT * FROM categories WHERE parent_id = 1",
        "SELECT * FROM tags WHERE product_id = 1",
        "SELECT * FROM wishlists WHERE user_id = 1",
    ];

    group.bench_function("batch_parse_10", |b| {
        b.iter(|| parser.parse_batch(black_box(&sqls_10)).unwrap())
    });

    let sqls_100: Vec<String> = (0..100)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i % 10, i))
        .collect();
    let sqls_100_refs: Vec<&str> = sqls_100.iter().map(|s| s.as_str()).collect();

    group.bench_function("batch_parse_100", |b| {
        b.iter(|| parser.parse_batch(black_box(&sqls_100_refs)).unwrap())
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3: MVCC 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_version_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_manager");

    let oracle = Arc::new(TimestampOracle::new(1));
    let manager = VersionManager::<String>::new(Arc::clone(&oracle));

    // add_version 벤치마크
    group.bench_function("add_version", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 100).into_bytes();
            let value = format!("value_{}", counter);
            let ts = oracle.as_ref().next();
            manager.add_version(black_box(key), black_box(value), black_box(ts));
            counter += 1;
        })
    });

    // 데이터 미리 추가
    for i in 0..1000 {
        let key = format!("key_{}", i % 100).into_bytes();
        let value = format!("value_{}", i);
        let ts = oracle.as_ref().next();
        manager.add_version(key, value, ts);
    }

    // get_at_snapshot 벤치마크
    let snapshot_ts = oracle.as_ref().read();
    group.bench_function("get_at_snapshot", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 100);
            let _ = manager.get_at_snapshot(black_box(key.as_bytes()), black_box(snapshot_ts));
            counter += 1;
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_two_level_cache,
    bench_parallel_sql_parser,
    bench_version_manager
);
criterion_main!(benches);
