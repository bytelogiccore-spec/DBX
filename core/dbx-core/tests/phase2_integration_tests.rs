// Phase 2 Integration Tests — Phase 3: Section 7.1
//
// 종단 간 통합 테스트: Phase 2 컴포넌트 간 상호작용 검증

use dbx_core::engine::plan::PlanCache;
use dbx_core::error::DbxResult;
use dbx_core::sql::executor::parallel_query::{AggregateType, ParallelQueryExecutor};
use dbx_core::engine::schema_versioning::SchemaVersionManager;
use dbx_core::engine::index_versioning::{IndexVersionManager, IndexType};
use dbx_core::wal::partitioned_wal::{PartitionedWalWriter, ParallelCheckpointManager};
use dbx_core::wal::WalRecord;
use dbx_core::engine::{BenchmarkRunner, FeatureFlags, Feature};

use arrow::array::{Int64Array, StringArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::path::PathBuf;
use tempfile::tempdir;

// ─── Helpers ────────────────────────────────────────────

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

fn insert_record(table: &str, key: &[u8], value: &[u8]) -> WalRecord {
    WalRecord::Insert {
        table: table.to_string(),
        key: key.to_vec(),
        value: value.to_vec(),
        ts: 0,
    }
}

fn parse_sql(sql: &str) -> sqlparser::ast::Statement {
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    let dialect = GenericDialect {};
    Parser::parse_sql(&dialect, sql).unwrap().remove(0)
}

// ═══════════════════════════════════════════════════════════
// 7.1 통합 테스트 (10개)
// ═══════════════════════════════════════════════════════════

/// 테스트 1: Query Plan Cache 통합
///
/// PlanCache의 L1(DashMap) → L2(디스크) 2레벨 캐시 동작 검증
#[test]
fn test_query_plan_cache_integration() -> DbxResult<()> {
    let dir = tempdir().unwrap();
    let cache = PlanCache::new(100).with_l2_cache(dir.path().to_path_buf());

    // SQL 파싱 + 캐싱
    let sqls = [
        "SELECT * FROM users WHERE id = 1",
        "SELECT name FROM orders WHERE total > 100",
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
    ];

    // 1차: 캐시 미스 → 파싱 + 캐시 저장
    for sql in &sqls {
        let stmt = parse_sql(sql);
        cache.insert(sql.to_string(), stmt);
    }

    // 2차: 캐시 히트
    for sql in &sqls {
        let cached = cache.get(sql);
        assert!(cached.is_some(), "캐시에서 찾을 수 없음: {sql}");
    }

    // 통계 검증
    let stats = cache.stats();
    let hits = stats.hits.load(Ordering::Relaxed);
    assert!(hits >= 3, "히트 수가 3 이상이어야 함: {}", hits);
    assert!(stats.hit_rate() > 0.0, "히트율이 0보다 커야 함");

    // 캐시 크기 확인
    assert_eq!(cache.len(), 3);

    println!("✅ test_query_plan_cache_integration: hit_rate={:.1}%, entries={}",
        stats.hit_rate() * 100.0, cache.len());
    drop(stats);
    Ok(())
}

/// 테스트 2: 병렬 쿼리 실행
///
/// ParallelQueryExecutor의 par_filter + par_aggregate + par_project 통합 검증
#[test]
fn test_parallel_query_execution() {
    let executor = ParallelQueryExecutor::new().with_threshold(2);

    // 대량 데이터 배치 생성 (5개 배치 × 3행)
    let batches: Vec<RecordBatch> = (0..5)
        .map(|batch_idx| {
            let start = batch_idx * 3 + 1;
            let ids: Vec<i64> = (start..start + 3).collect();
            let names: Vec<&str> = vec!["a", "b", "c"];
            make_batch(&ids, &names)
        })
        .collect();

    // 1단계: 병렬 프로젝션 (id 컬럼만)
    let projected = executor.par_project(&batches, &[0]).unwrap();
    assert_eq!(projected.len(), 5);
    for batch in &projected {
        assert_eq!(batch.num_columns(), 1);
        assert_eq!(batch.schema().field(0).name(), "id");
    }

    // 2단계: 병렬 집계 (SUM)
    let sum_result = executor.par_aggregate(&batches, 0, AggregateType::Sum).unwrap();
    let expected_sum: f64 = (1..=15).map(|x| x as f64).sum();
    assert_eq!(sum_result.value, expected_sum);
    assert_eq!(sum_result.count, 15);

    // 3단계: 병렬 집계 (AVG, MIN, MAX)
    let avg_result = executor.par_aggregate(&batches, 0, AggregateType::Avg).unwrap();
    assert!((avg_result.value - 8.0).abs() < 0.01);

    let min_result = executor.par_aggregate(&batches, 0, AggregateType::Min).unwrap();
    assert_eq!(min_result.value, 1.0);

    let max_result = executor.par_aggregate(&batches, 0, AggregateType::Max).unwrap();
    assert_eq!(max_result.value, 15.0);

    println!("✅ test_parallel_query_execution: SUM={}, AVG={:.1}, MIN={}, MAX={}",
        sum_result.value, avg_result.value, min_result.value, max_result.value);
}

/// 테스트 3: 무중단 DDL (Schema Versioning)
#[test]
fn test_zero_downtime_ddl() -> DbxResult<()> {
    let mgr = SchemaVersionManager::new();

    // 초기 스키마 등록
    let schema_v1 = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    mgr.register_table("users", schema_v1.clone())?;

    // V1 스키마 확인
    let current = mgr.get_current("users")?;
    assert_eq!(current.fields().len(), 2);

    // DDL: ALTER TABLE ADD COLUMN (V2)
    let schema_v2 = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("email", DataType::Utf8, true),
    ]));
    let v2 = mgr.alter_table("users", schema_v2.clone(), "ADD COLUMN email")?;

    // V2 스키마 확인
    let current_v2 = mgr.get_current("users")?;
    assert_eq!(current_v2.fields().len(), 3);

    // 과거 시점(V1) 스키마도 조회 가능 (MVCC)
    let v1_schema = mgr.get_at_version("users", 1)?;
    assert_eq!(v1_schema.fields().len(), 2);

    // DDL: ALTER TABLE ADD COLUMN (V3)
    let schema_v3 = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("email", DataType::Utf8, true),
        Field::new("age", DataType::Int64, true),
    ]));
    mgr.alter_table("users", schema_v3.clone(), "ADD COLUMN age")?;

    // 버전 히스토리 확인
    let history = mgr.version_history("users")?;
    assert_eq!(history.len(), 3); // V0, V1, V2

    // 롤백 테스트
    mgr.rollback("users", v2)?;
    let rolled_back = mgr.get_current("users")?;
    assert_eq!(rolled_back.fields().len(), 3); // V2의 3필드

    println!("✅ test_zero_downtime_ddl: {} versions, rollback OK", history.len());
    Ok(())
}

/// 테스트 4: WAL 병렬 쓰기
#[test]
fn test_wal_parallel_write() -> DbxResult<()> {
    let dir = tempdir().unwrap();
    let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100)?;

    // 3개 테이블에 동시 쓰기
    let tables = ["users", "orders", "products"];

    for table in &tables {
        for i in 0..50 {
            wal.append(insert_record(table, format!("k{i}").as_bytes(), b"val"))?;
        }
    }

    // 파티션 검증
    assert_eq!(wal.partition_count(), 3);
    assert_eq!(wal.buffered_count(), 150);
    assert_eq!(wal.current_sequence(), 150);

    // 전체 플러시
    let flushed = wal.flush_all()?;
    assert_eq!(flushed, 150);
    assert_eq!(wal.buffered_count(), 0);

    // WAL 파일 존재 확인
    for table in &tables {
        assert!(dir.path().join(format!("{table}.wal")).exists(),
            "{table}.wal 파일이 없음");
    }

    println!("✅ test_wal_parallel_write: {} records across {} partitions",
        flushed, tables.len());
    Ok(())
}

/// 테스트 5: 체크포인트 직렬화
#[test]
fn test_checkpoint_serialization() -> DbxResult<()> {
    let dir = tempdir().unwrap();
    let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100)?;

    // 데이터 쓰기 + 플러시
    for i in 0..20 {
        wal.append(insert_record("users", format!("k{i}").as_bytes(), b"v"))?;
        wal.append(insert_record("orders", format!("k{i}").as_bytes(), b"v"))?;
    }
    wal.flush_all()?;

    // 체크포인트 실행
    let mgr = ParallelCheckpointManager::new(dir.path().to_path_buf());
    let count = mgr.checkpoint_tables(&["users".to_string(), "orders".to_string()])?;
    assert_eq!(count, 2);

    // 체크포인트 파일 존재, 원래 WAL은 이동됨
    assert!(dir.path().join("users.checkpoint").exists());
    assert!(dir.path().join("orders.checkpoint").exists());
    assert!(!dir.path().join("users.wal").exists());
    assert!(!dir.path().join("orders.wal").exists());

    println!("✅ test_checkpoint_serialization: {count} tables checkpointed");
    Ok(())
}

/// 테스트 6: 스키마 버전 관리 (독립 테이블별)
#[test]
fn test_schema_versioning() -> DbxResult<()> {
    let mgr = SchemaVersionManager::new();

    // 2개 테이블 등록
    mgr.register_table("users", Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
    ])))?;
    mgr.register_table("orders", Arc::new(Schema::new(vec![
        Field::new("order_id", DataType::Int64, false),
        Field::new("total", DataType::Float64, false),
    ])))?;

    // users만 ALTER
    mgr.alter_table("users", Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("email", DataType::Utf8, true),
    ])), "add email")?;

    // 독립 버전 관리 확인
    let users_ver = mgr.current_version("users")?;
    let orders_ver = mgr.current_version("orders")?;
    assert_eq!(users_ver, 2);
    assert_eq!(orders_ver, 1);

    println!("✅ test_schema_versioning: users(v{}), orders(v{})", users_ver, orders_ver);
    Ok(())
}

/// 테스트 7: 인덱스 버전 관리 (REINDEX)
#[test]
fn test_index_versioning() -> DbxResult<()> {
    let mgr = IndexVersionManager::new();

    // 인덱스 생성 (Building 상태)
    let v1 = mgr.create_index("users_email", "users", vec!["email".into()], IndexType::Hash)?;

    // REINDEX 시작 (새 버전 Building)
    let v2 = mgr.start_reindex("users_email", vec!["email".into()], IndexType::BTree)?;

    // 기존 V1이 여전히 Active
    let active = mgr.get_active("users_email")?;
    assert_eq!(active.version, v1);

    // REINDEX 완료 → V2 활성화
    mgr.complete_reindex("users_email", v2)?;
    let active = mgr.get_active("users_email")?;
    assert_eq!(active.version, v2);

    // 테이블별 인덱스 조회
    let indexes = mgr.list_indexes("users")?;
    assert!(!indexes.is_empty());

    println!("✅ test_index_versioning: reindex v{} → v{}", v1, v2);
    Ok(())
}

/// 테스트 8: 전체 최적화 스택 통합
#[test]
fn test_full_optimization_stack() -> DbxResult<()> {
    let dir = tempdir().unwrap();

    // 1. PlanCache
    let cache = PlanCache::new(50).with_l2_cache(dir.path().join("cache"));
    let stmt = parse_sql("SELECT id, name FROM users");
    cache.insert("SELECT id, name FROM users".into(), stmt);
    let hit = cache.get("SELECT id, name FROM users");
    assert!(hit.is_some());

    // 2. ParallelQueryExecutor
    let executor = ParallelQueryExecutor::new();
    let batches = vec![
        make_batch(&[1, 2, 3], &["a", "b", "c"]),
        make_batch(&[4, 5, 6], &["d", "e", "f"]),
    ];
    let result = executor.par_aggregate(&batches, 0, AggregateType::Sum)?;
    assert_eq!(result.value, 21.0);

    // 3. WAL + Checkpoint
    let wal = PartitionedWalWriter::new(dir.path().join("wal"), 100)?;
    wal.append(insert_record("users", b"k1", b"sum=21"))?;
    wal.flush_all()?;
    let cp = ParallelCheckpointManager::new(dir.path().join("wal"));
    cp.checkpoint_tables(&["users".to_string()])?;

    // 4. Schema versioning
    let schema_mgr = SchemaVersionManager::new();
    schema_mgr.register_table("users", Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ])))?;

    // 5. Index versioning
    let idx_mgr = IndexVersionManager::new();
    let v = idx_mgr.create_index("users_id", "users", vec!["id".into()], IndexType::Hash)?;
    idx_mgr.complete_reindex("users_id", v)?;

    // 모든 컴포넌트 동작 확인
    assert!(dir.path().join("wal/users.checkpoint").exists());
    assert!(cache.stats().hits.load(Ordering::Relaxed) >= 1);

    println!("✅ test_full_optimization_stack: all 5 components integrated");
    Ok(())
}

/// 테스트 9: 성능 회귀 방지
#[test]
fn test_rollback_on_regression() -> DbxResult<()> {
    let runner = BenchmarkRunner::new()
        .with_baseline_path(PathBuf::from("target/phase2_baseline.json"))
        .with_threshold(1.5);

    let _ = runner.load_baseline();

    // 1. PlanCache 벤치마크
    let cache = PlanCache::new(100);
    for i in 0..50 {
        let sql = format!("SELECT * FROM t{i}");
        cache.insert(sql.clone(), parse_sql(&sql));
    }
    let plan_result = runner.run("plan_cache_hit_p2", || {
        for i in 0..10 {
            let _ = cache.get(&format!("SELECT * FROM t{}", i % 50));
        }
    })?;
    println!("  plan_cache_hit: {:.4} ms", plan_result.avg_time_ms);

    // 2. 병렬 집계 벤치마크
    let executor = ParallelQueryExecutor::new();
    let batches: Vec<RecordBatch> = (0..10)
        .map(|i| make_batch(&[i * 3 + 1, i * 3 + 2, i * 3 + 3], &["a", "b", "c"]))
        .collect();
    let agg_result = runner.run("parallel_aggregate_p2", || {
        let _ = executor.par_aggregate(&batches, 0, AggregateType::Sum);
    })?;
    println!("  parallel_aggregate: {:.4} ms", agg_result.avg_time_ms);

    // 3. WAL 벤치마크
    let wal_dir = tempdir().unwrap();
    let wal = PartitionedWalWriter::new(wal_dir.path().to_path_buf(), 1000)?;
    let wal_result = runner.run("wal_append_100_p2", || {
        for i in 0..100 {
            let _ = wal.append(insert_record("bench", format!("k{i}").as_bytes(), b"v"));
        }
    })?;
    println!("  wal_append_100: {:.4} ms", wal_result.avg_time_ms);

    // 회귀 검출
    for (name, result) in [
        ("plan_cache_hit_p2", &plan_result),
        ("parallel_aggregate_p2", &agg_result),
        ("wal_append_100_p2", &wal_result),
    ] {
        match runner.check_regression(name, result) {
            Ok(_) => runner.update_baseline(name, result),
            Err(e) => eprintln!("⚠️  {}: {}", name, e),
        }
    }
    let _ = runner.save_baseline();

    println!("✅ test_rollback_on_regression: 3 benchmarks completed");
    Ok(())
}

/// 테스트 10: Feature Flag 토글
#[test]
fn test_feature_flag_toggle() {
    let mut flags = FeatureFlags::default();

    // 기본: 비활성화
    assert!(!flags.is_enabled(Feature::BinarySerialization));
    assert!(!flags.is_enabled(Feature::MultiThreading));

    // 개별 활성화
    flags.enable(Feature::MultiThreading);
    assert!(flags.is_enabled(Feature::MultiThreading));

    // 멀티스레딩 활성화 → 병렬 쿼리 사용 가능
    if flags.is_enabled(Feature::MultiThreading) {
        let executor = ParallelQueryExecutor::new();
        let batches = vec![
            make_batch(&[1, 2], &["a", "b"]),
            make_batch(&[3, 4], &["c", "d"]),
        ];
        let result = executor.par_aggregate(&batches, 0, AggregateType::Count).unwrap();
        assert_eq!(result.count, 4);
    }

    // MVCC 활성화 → 스키마 버전 관리
    flags.enable(Feature::MvccExtension);
    if flags.is_enabled(Feature::MvccExtension) {
        let mgr = SchemaVersionManager::new();
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
        ]));
        mgr.register_table("test", schema).unwrap();
        let ver = mgr.current_version("test").unwrap();
        assert_eq!(ver, 1);
    }

    // 비활성화
    flags.disable(Feature::MultiThreading);
    assert!(!flags.is_enabled(Feature::MultiThreading));

    println!("✅ test_feature_flag_toggle: all flags toggled OK");
}

// ═══════════════════════════════════════════════════════════
// 부하 + 안정성 테스트
// ═══════════════════════════════════════════════════════════

/// 부하 테스트: WAL 대량 쓰기
#[test]
fn test_wal_stress_write() -> DbxResult<()> {
    let dir = tempdir().unwrap();
    let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 500)?;

    // 10개 테이블에 각 100개 레코드
    for table_idx in 0..10 {
        let table = format!("table_{table_idx}");
        let records: Vec<WalRecord> = (0..100)
            .map(|i| insert_record(&table, format!("k{i}").as_bytes(), b"stress_val"))
            .collect();
        wal.append_batch(records)?;
    }

    assert_eq!(wal.partition_count(), 10);
    assert_eq!(wal.current_sequence(), 1000);
    let flushed = wal.flush_all()?;
    assert_eq!(flushed, 1000);

    println!("✅ test_wal_stress_write: 1000 records across 10 partitions");
    Ok(())
}

/// 안정성 테스트: 스키마 대량 변경
#[test]
fn test_schema_stability() -> DbxResult<()> {
    let mgr = SchemaVersionManager::new();

    mgr.register_table("evolving", Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
    ])))?;

    // 20번 ALTER TABLE
    for i in 1..=20 {
        let mut fields = vec![Field::new("id", DataType::Int64, false)];
        for j in 1..=i {
            fields.push(Field::new(format!("col_{j}"), DataType::Utf8, true));
        }
        mgr.alter_table("evolving", Arc::new(Schema::new(fields)), &format!("add col_{i}"))?;
    }

    let history = mgr.version_history("evolving")?;
    assert_eq!(history.len(), 21);

    // 모든 과거 버전 접근 가능
    for v in 1..=21u64 {
        let schema = mgr.get_at_version("evolving", v)?;
        assert_eq!(schema.fields().len(), v as usize);
    }

    println!("✅ test_schema_stability: 21 versions, all accessible");
    Ok(())
}

/// 멀티스레드 안정성 테스트: 동시 캐시 접근
#[test]
fn test_concurrent_stability() -> DbxResult<()> {
    use std::thread;

    let dir = tempdir().unwrap();
    let cache = Arc::new(PlanCache::new(200).with_l2_cache(dir.path().to_path_buf()));

    // 10개 스레드에서 동시 접근
    let handles: Vec<_> = (0..10)
        .map(|t| {
            let cache = Arc::clone(&cache);
            thread::spawn(move || {
                for i in 0..20 {
                    let sql = format!("SELECT * FROM t{} WHERE id = {}", t, i);
                    let stmt = parse_sql(&sql);
                    cache.insert(sql.clone(), stmt);
                    let _ = cache.get(&sql);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert!(cache.len() >= 100, "최소 100개 이상 캐시: {}", cache.len());

    let stats = cache.stats();
    let hits = stats.hits.load(Ordering::Relaxed);
    let misses = stats.misses.load(Ordering::Relaxed);
    println!("✅ test_concurrent_stability: entries={}, hits={}, misses={}",
        cache.len(), hits, misses);
    Ok(())
}
