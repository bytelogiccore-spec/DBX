// Phase 2 성능 벤치마크
//
// Section 1: Query Plan Cache (L1 DashMap, L2 디스크)
// Section 2: 병렬 쿼리 집계 (ParallelQueryExecutor)
// Section 3: WAL 병렬 쓰기 (PartitionedWalWriter)
// Section 4: 스키마/인덱스 버전 관리

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dbx_core::engine::plan::PlanCache;
use dbx_core::sql::executor::parallel_query::{AggregateType, ParallelQueryExecutor};
use dbx_core::engine::schema_versioning::SchemaVersionManager;
use dbx_core::engine::index_versioning::{IndexVersionManager, IndexType};
use dbx_core::wal::partitioned_wal::PartitionedWalWriter;
use dbx_core::wal::WalRecord;

use arrow::array::{Int64Array, StringArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
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
// Section 1: Query Plan Cache
// ═══════════════════════════════════════════════════════════════════════════

fn bench_plan_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("plan_cache");

    let dir = tempdir().unwrap();
    let cache = PlanCache::new(1000).with_l2_cache(dir.path().to_path_buf());

    // 캐시 워밍업
    for i in 0..200 {
        let sql = format!("SELECT * FROM t{} WHERE id = {}", i % 20, i);
        cache.insert(sql.clone(), parse_sql(&sql));
    }

    // L1 캐시 히트 벤치마크
    group.bench_function("l1_cache_hit", |b| {
        let mut counter = 0;
        b.iter(|| {
            let sql = format!("SELECT * FROM t{} WHERE id = {}", counter % 20, counter % 200);
            let _ = cache.get(black_box(&sql));
            counter += 1;
        })
    });

    // insert 벤치마크
    group.bench_function("insert", |b| {
        let mut counter = 1000;
        b.iter(|| {
            let sql = format!("SELECT col_{} FROM table_{}", counter, counter % 50);
            let stmt = parse_sql(&sql);
            cache.insert(black_box(sql), black_box(stmt));
            counter += 1;
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2: 병렬 쿼리 집계
// ═══════════════════════════════════════════════════════════════════════════

fn bench_parallel_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_query");

    let executor = ParallelQueryExecutor::new().with_threshold(2);

    // 소규모 배치 (5 × 3행 = 15행)
    let batches_small: Vec<RecordBatch> = (0..5)
        .map(|i| make_batch(&[i * 3 + 1, i * 3 + 2, i * 3 + 3], &["a", "b", "c"]))
        .collect();

    // 대규모 배치 (50 × 3행 = 150행)
    let batches_large: Vec<RecordBatch> = (0..50)
        .map(|i| make_batch(&[i * 3 + 1, i * 3 + 2, i * 3 + 3], &["a", "b", "c"]))
        .collect();

    group.bench_function("sum_15rows", |b| {
        b.iter(|| {
            executor.par_aggregate(black_box(&batches_small), 0, AggregateType::Sum).unwrap()
        })
    });

    group.bench_function("sum_150rows", |b| {
        b.iter(|| {
            executor.par_aggregate(black_box(&batches_large), 0, AggregateType::Sum).unwrap()
        })
    });

    group.bench_function("project_150rows", |b| {
        b.iter(|| {
            executor.par_project(black_box(&batches_large), &[0]).unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3: WAL 병렬 쓰기
// ═══════════════════════════════════════════════════════════════════════════

fn bench_wal_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("wal_write");

    group.bench_function("append_100", |b| {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 10000).unwrap();
        b.iter(|| {
            for i in 0..100 {
                let _ = wal.append(black_box(WalRecord::Insert {
                    table: "bench".to_string(),
                    key: format!("k{i}").into_bytes(),
                    value: b"val".to_vec(),
                    ts: 0,
                }));
            }
        })
    });

    group.bench_function("flush_100", |b| {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 10000).unwrap();
        for i in 0..100 {
            wal.append(WalRecord::Insert {
                table: "bench".to_string(),
                key: format!("k{i}").into_bytes(),
                value: b"val".to_vec(),
                ts: 0,
            }).unwrap();
        }
        b.iter(|| {
            let _ = wal.flush_all();
        })
    });

    group.bench_function("multi_table_append", |b| {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 10000).unwrap();
        b.iter(|| {
            for t in 0..5 {
                for i in 0..20 {
                    let _ = wal.append(black_box(WalRecord::Insert {
                        table: format!("table_{t}"),
                        key: format!("k{i}").into_bytes(),
                        value: b"val".to_vec(),
                        ts: 0,
                    }));
                }
            }
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4: 스키마/인덱스 버전 관리
// ═══════════════════════════════════════════════════════════════════════════

fn bench_schema_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_index");

    // 스키마 등록 + ALTER
    group.bench_function("schema_alter", |b| {
        let mgr = SchemaVersionManager::new();
        mgr.register_table("bench", Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
        ]))).unwrap();
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new(format!("col_{counter}"), DataType::Utf8, true),
            ]));
            mgr.alter_table("bench", black_box(schema), "bench alter").unwrap();
        })
    });

    // 스키마 get_current
    group.bench_function("schema_get_current", |b| {
        let mgr = SchemaVersionManager::new();
        mgr.register_table("bench", Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
        ]))).unwrap();
        for i in 0..50 {
            mgr.alter_table("bench", Arc::new(Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new(format!("col_{i}"), DataType::Utf8, true),
            ])), "alter").unwrap();
        }
        b.iter(|| {
            let _ = mgr.get_current(black_box("bench"));
        })
    });

    // 인덱스 생성
    group.bench_function("index_create", |b| {
        let mgr = IndexVersionManager::new();
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let name = format!("idx_{counter}");
            let _ = mgr.create_index(black_box(&name), "bench", vec!["col".into()], IndexType::Hash);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_plan_cache,
    bench_parallel_query,
    bench_wal_write,
    bench_schema_index,
);
criterion_main!(benches);
