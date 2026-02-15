// Phase 1 개선된 벤치마크
//
// 더 큰 데이터 크기, 복잡한 SQL 쿼리, 큰 배치 크기

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dbx_core::engine::TwoLevelCache;
use dbx_core::sql::ParallelSqlParser;
use dbx_core::transaction::{TimestampOracle, VersionManager};
use std::path::PathBuf;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════════════════
// Section 1: 더 큰 데이터 크기 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_two_level_cache_large_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("two_level_cache_large");
    
    // L1 캐시 (10MB, 100KB, 1MB 데이터)
    let cache_l1 = TwoLevelCache::new(10 * 1024 * 1024, PathBuf::from("target/bench_cache_l1_large"));
    let _ = cache_l1.clear();
    
    // 10KB 데이터
    group.bench_function("l1_put_10kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 10 * 1024]; // 10KB
            cache_l1.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });
    
    // 100KB 데이터
    group.bench_function("l1_put_100kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 100 * 1024]; // 100KB
            cache_l1.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });
    
    // 1MB 데이터
    group.bench_function("l1_put_1mb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 1024 * 1024]; // 1MB
            cache_l1.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });
    
    // L2 캐시 (작은 L1으로 L2 강제)
    let cache_l2 = TwoLevelCache::new(10, PathBuf::from("target/bench_cache_l2_large"));
    let _ = cache_l2.clear();
    
    // 10KB 데이터 L2
    group.bench_function("l2_put_10kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 10 * 1024]; // 10KB
            cache_l2.put(black_box(key), black_box(value)).unwrap();
            counter += 1;
        })
    });
    
    // 100KB 데이터 L2
    group.bench_function("l2_put_100kb", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter);
            let value = vec![0u8; 100 * 1024]; // 100KB
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
// Section 2: 복잡한 SQL 쿼리 및 큰 배치 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_parallel_sql_parser_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_sql_parser_complex");
    
    let parser = ParallelSqlParser::new();
    
    // 복잡한 JOIN 쿼리
    let complex_join = "SELECT u.id, u.name, o.order_id, o.total, p.product_name \
                        FROM users u \
                        INNER JOIN orders o ON u.id = o.user_id \
                        INNER JOIN order_items oi ON o.order_id = oi.order_id \
                        INNER JOIN products p ON oi.product_id = p.id \
                        WHERE u.created_at > '2024-01-01' AND o.status = 'completed' \
                        ORDER BY o.total DESC LIMIT 100";
    
    group.bench_function("single_complex_join", |b| {
        b.iter(|| {
            parser.parse(black_box(complex_join)).unwrap()
        })
    });
    
    // 서브쿼리
    let subquery = "SELECT * FROM users WHERE id IN \
                    (SELECT user_id FROM orders WHERE total > 1000) \
                    AND status = 'active'";
    
    group.bench_function("single_subquery", |b| {
        b.iter(|| {
            parser.parse(black_box(subquery)).unwrap()
        })
    });
    
    // 큰 배치 (1000개)
    let sqls_1000: Vec<String> = (0..1000)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {} AND status = 'active'", i % 10, i))
        .collect();
    let sqls_1000_refs: Vec<&str> = sqls_1000.iter().map(|s| s.as_str()).collect();
    
    group.bench_function("batch_parse_1000", |b| {
        b.iter(|| {
            parser.parse_batch(black_box(&sqls_1000_refs)).unwrap()
        })
    });
    
    // 매우 큰 배치 (10000개)
    let sqls_10000: Vec<String> = (0..10000)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i % 100, i))
        .collect();
    let sqls_10000_refs: Vec<&str> = sqls_10000.iter().map(|s| s.as_str()).collect();
    
    group.bench_function("batch_parse_10000", |b| {
        b.iter(|| {
            parser.parse_batch(black_box(&sqls_10000_refs)).unwrap()
        })
    });
    
    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3: MVCC 대용량 벤치마크
// ═══════════════════════════════════════════════════════════════════════════

fn bench_version_manager_large_scale(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_manager_large");
    
    let oracle = Arc::new(TimestampOracle::new(1));
    let manager = VersionManager::<String>::new(Arc::clone(&oracle));
    
    // 대용량 데이터 추가 (10000개 버전)
    for i in 0..10000 {
        let key = format!("key_{}", i % 1000).into_bytes();
        let value = format!("value_{}", i);
        let ts = oracle.as_ref().next();
        let _ = manager.add_version(key, value, ts);
    }
    
    let snapshot_ts = oracle.as_ref().read();
    
    // 대용량 조회
    group.bench_function("get_at_snapshot_large", |b| {
        let mut counter = 0;
        b.iter(|| {
            let key = format!("key_{}", counter % 1000);
            let _ = manager.get_at_snapshot(black_box(key.as_bytes()), black_box(snapshot_ts));
            counter += 1;
        })
    });
    
    // 가비지 컬렉션
    group.bench_function("garbage_collection", |b| {
        b.iter(|| {
            let min_ts = oracle.as_ref().read() - 1000;
            let _ = manager.collect_garbage(black_box(min_ts));
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_two_level_cache_large_data,
    bench_parallel_sql_parser_complex,
    bench_version_manager_large_scale
);
criterion_main!(benches);
