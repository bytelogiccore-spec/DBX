// Phase 2 Before/After 비교 벤치마크
//
// 각 섹션에서 "기존 방식(Before)" vs "Phase 2 방식(After)"을 직접 비교
//
// 1. SQL 파싱: 매번 파싱 vs PlanCache 히트
// 2. 집계 쿼리: 순차 처리 vs ParallelQueryExecutor
// 3. WAL 쓰기: 단일 파일 append vs PartitionedWalWriter
// 4. 스키마 조회: HashMap 직접 조회 vs SchemaVersionManager (MVCC 포함)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use dbx_core::engine::plan::PlanCache;
use dbx_core::sql::executor::parallel_query::{AggregateType, ParallelQueryExecutor};
use dbx_core::engine::schema_versioning::SchemaVersionManager;
use dbx_core::wal::partitioned_wal::PartitionedWalWriter;
use dbx_core::wal::WalRecord;

use arrow::array::{Int64Array, StringArray, RecordBatch, Array};
use arrow::datatypes::{DataType, Field, Schema};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;

fn make_batch(ids: &[i64], names: &[&str]) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(Int64Array::from(ids.to_vec())),
            Arc::new(StringArray::from(names.to_vec())),
        ],
    )
    .unwrap()
}

fn parse_sql(sql: &str) -> sqlparser::ast::Statement {
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    let dialect = GenericDialect {};
    Parser::parse_sql(&dialect, sql).unwrap().remove(0)
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. SQL 파싱: 매번 파싱 vs PlanCache
// ═══════════════════════════════════════════════════════════════════════════

fn bench_before_after_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("1_sql_parse");

    let sql = "SELECT id, name, email FROM users WHERE id > 100 AND status = 'active' ORDER BY name";

    // BEFORE: 매번 파싱
    group.bench_function("before_parse_every_time", |b| {
        b.iter(|| {
            parse_sql(black_box(sql))
        })
    });

    // AFTER: PlanCache (캐시 히트)
    let dir = tempdir().unwrap();
    let cache = PlanCache::new(1000).with_l2_cache(dir.path().to_path_buf());
    cache.insert(sql.to_string(), parse_sql(sql));

    group.bench_function("after_plan_cache_hit", |b| {
        b.iter(|| {
            cache.get(black_box(sql)).unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. 집계: 순차 vs 병렬
// ═══════════════════════════════════════════════════════════════════════════

fn bench_before_after_aggregate(c: &mut Criterion) {
    let mut group = c.benchmark_group("2_aggregate");

    // 50 배치 × 3행 = 150행
    let batches: Vec<RecordBatch> = (0..50)
        .map(|i| make_batch(&[i * 3 + 1, i * 3 + 2, i * 3 + 3], &["a", "b", "c"]))
        .collect();

    // BEFORE: 순차 SUM (손으로 루프)
    group.bench_function("before_sequential_sum", |b| {
        b.iter(|| {
            let mut total: f64 = 0.0;
            let mut count: usize = 0;
            for batch in black_box(&batches) {
                let col = batch.column(0);
                let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                for i in 0..arr.len() {
                    total += arr.value(i) as f64;
                    count += 1;
                }
            }
            (total, count)
        })
    });

    // AFTER: ParallelQueryExecutor (Rayon 병렬)
    let executor = ParallelQueryExecutor::new().with_threshold(2);
    group.bench_function("after_parallel_sum", |b| {
        b.iter(|| {
            executor.par_aggregate(black_box(&batches), 0, AggregateType::Sum).unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. WAL: 단일 파일 vs 파티셔닝
// ═══════════════════════════════════════════════════════════════════════════

fn bench_before_after_wal(c: &mut Criterion) {
    let mut group = c.benchmark_group("3_wal");

    // BEFORE: 단일 파일에 직렬 기록 (std::fs::OpenOptions simulated)
    group.bench_function("before_single_file_append", |b| {
        let dir = tempdir().unwrap();
        let path = dir.path().join("single.wal");
        b.iter(|| {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap();
            for i in 0..100 {
                let record = format!("INSERT|table_{t}|k{i}|val\n", t = i % 5, i = i);
                f.write_all(black_box(record.as_bytes())).unwrap();
            }
        })
    });

    // AFTER: PartitionedWalWriter (테이블별 파티셔닝)
    group.bench_function("after_partitioned_wal", |b| {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 10000).unwrap();
        b.iter(|| {
            for i in 0..100 {
                let _ = wal.append(black_box(WalRecord::Insert {
                    table: format!("table_{}", i % 5),
                    key: format!("k{i}").into_bytes(),
                    value: b"val".to_vec(),
                    ts: 0,
                }));
            }
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. 스키마 조회: HashMap vs SchemaVersionManager
// ═══════════════════════════════════════════════════════════════════════════

fn bench_before_after_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("4_schema");

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]));

    // BEFORE: 단순 HashMap
    let mut map = HashMap::new();
    map.insert("users".to_string(), schema.clone());
    for i in 0..50 {
        map.insert(format!("table_{i}"), schema.clone());
    }

    group.bench_function("before_hashmap_get", |b| {
        b.iter(|| {
            map.get(black_box("users")).unwrap().clone()
        })
    });

    // AFTER: SchemaVersionManager (MVCC + 버전 히스토리 포함)
    let mgr = SchemaVersionManager::new();
    mgr.register_table("users", schema.clone()).unwrap();
    for i in 0..50 {
        mgr.register_table(&format!("table_{i}"), schema.clone()).unwrap();
    }

    group.bench_function("after_schema_mgr_get", |b| {
        b.iter(|| {
            mgr.get_current(black_box("users")).unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. 반복 SQL: 매번 파싱 10회 vs PlanCache 10회
// ═══════════════════════════════════════════════════════════════════════════

fn bench_before_after_repeated_sql(c: &mut Criterion) {
    let mut group = c.benchmark_group("5_repeated_sql_10x");

    let sqls: Vec<String> = (0..10)
        .map(|i| format!("SELECT * FROM users WHERE id = {i}"))
        .collect();

    // BEFORE: 10개 SQL 매번 파싱
    group.bench_function("before_parse_10_sqls", |b| {
        b.iter(|| {
            for sql in black_box(&sqls) {
                let _ = parse_sql(sql);
            }
        })
    });

    // AFTER: PlanCache에서 10개 캐시 히트
    let cache = PlanCache::new(100);
    for sql in &sqls {
        cache.insert(sql.clone(), parse_sql(sql));
    }

    group.bench_function("after_cache_10_sqls", |b| {
        b.iter(|| {
            for sql in black_box(&sqls) {
                let _ = cache.get(sql).unwrap();
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_before_after_parse,
    bench_before_after_aggregate,
    bench_before_after_wal,
    bench_before_after_schema,
    bench_before_after_repeated_sql,
);
criterion_main!(benches);
