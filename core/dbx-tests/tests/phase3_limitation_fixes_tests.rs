use dbx_core::engine::Database;
use dbx_core::error::DbxResult;
use dbx_core::storage::compression::CompressionAlgorithm;
use dbx_core::storage::partition::{
    PartitionLifecycle, PartitionMap, PartitionTierHint, PartitionType,
};

// ─────────────────────────────────────────────
// Limitation Fix 1: PartitionStats 자동 갱신
// ─────────────────────────────────────────────

#[test]
fn test_stats_auto_incremented_on_insert() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE events (id INT, msg TEXT)")?;

    let map = PartitionMap {
        table: "events".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 2,
        },
        num_partitions: 2,
    };
    db.create_partition(map)?;

    // 10건 INSERT
    for i in 0u32..10 {
        db.insert("events", format!("{}", i).as_bytes(), b"data")?;
    }

    // 두 파티션에 대한 row_count 합산 = 10
    let all = db.all_partition_stats("events")?;
    let total: usize = all.values().map(|s| s.row_count).sum();
    assert_eq!(
        total, 10,
        "INSERT된 행 수와 stats.row_count 합산이 일치해야 함"
    );
    assert!(!all.is_empty(), "파티션 통계가 자동으로 생성되어야 함");
    Ok(())
}

#[test]
fn test_partition_creation_time_auto_recorded() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE logs (id INT)")?;

    let map = PartitionMap {
        table: "logs".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 2,
        },
        num_partitions: 2,
    };
    db.create_partition(map)?;

    // INSERT 전에는 creation_time 없음
    assert!(db.get_partition_creation_time("logs__p_part_0").is_none());

    // INSERT 후 sub-table에 creation_time 자동 기록
    db.insert("logs", b"0", b"val")?;

    // 적어도 하나의 파티션에 creation_time이 기록되어야 함
    let has_time = db.get_partition_creation_time("logs__p_part_0").is_some()
        || db.get_partition_creation_time("logs__p_part_1").is_some();
    assert!(
        has_time,
        "INSERT 후 partition_creation_times에 자동 기록되어야 함"
    );
    Ok(())
}

// ─────────────────────────────────────────────
// Limitation Fix 2: run_partition_lifecycle 실제 실행
// ─────────────────────────────────────────────

#[test]
fn test_run_lifecycle_archives_old_partition() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE archive_test (id INT)")?;

    let map = PartitionMap {
        table: "archive_test".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 2,
        },
        num_partitions: 2,
    };
    db.create_partition(map)?;

    db.enable_auto_archive(
        "archive_test",
        PartitionLifecycle {
            archive_after_days: 0, // 즉시 아카이브 (0일)
            delete_after_days: 3650,
        },
    )?;

    // INSERT 1건 (creation_time 자동 기록)
    db.insert("archive_test", b"0", b"data")?;

    // 실행
    let (archived, deleted) = db.run_partition_lifecycle("archive_test")?;

    // 아카이브 1개는 되어야 함 (0일이므로 즉시 대상)
    assert!(
        archived >= 1,
        "archive_after_days=0이면 즉시 아카이브 대상. archived={}",
        archived
    );
    assert_eq!(deleted, 0, "delete_after_days=3650이면 삭제 없어야 함");

    // 아카이브된 파티션의 TierHint가 Cold이어야 함
    let all_cold = db.list_partitions_by_tier("archive_test", PartitionTierHint::Cold)?;
    assert!(!all_cold.is_empty(), "아카이브된 파티션은 Cold 티어여야 함");

    // list_partitions_by_tier 결과("archive_test__p_part_N")에서 partition_name 추출
    // 예: "archive_test__p_part_0" → strip prefix "archive_test__" → "p_part_0"
    let cold_key = &all_cold[0];
    let part_short = cold_key.strip_prefix("archive_test__").unwrap_or(cold_key);
    let config = db.get_partition_compression("archive_test", part_short)?;
    assert_eq!(
        config.algorithm(),
        CompressionAlgorithm::Zstd,
        "아카이브 파티션은 ZSTD 압축이어야 함"
    );
    Ok(())
}

#[test]
fn test_run_all_lifecycles() -> DbxResult<()> {
    let db = Database::open_in_memory()?;

    // 두 테이블 설정
    db.execute_sql("CREATE TABLE tbl_a (id INT)")?;
    db.execute_sql("CREATE TABLE tbl_b (id INT)")?;

    for tbl in &["tbl_a", "tbl_b"] {
        let map = PartitionMap {
            table: tbl.to_string(),
            partition_type: PartitionType::Hash {
                column: "id".into(),
                num_partitions: 2,
            },
            num_partitions: 2,
        };
        db.create_partition(map)?;
        db.enable_auto_archive(
            tbl,
            PartitionLifecycle {
                archive_after_days: 0, // 즉시
                delete_after_days: 3650,
            },
        )?;
        db.insert(tbl, b"0", b"v")?;
    }

    let (archived, deleted) = db.run_all_partition_lifecycles()?;
    assert!(
        archived >= 2,
        "두 테이블에서 각각 최소 1개 파티션 아카이브. archived={}",
        archived
    );
    assert_eq!(deleted, 0);
    Ok(())
}

#[test]
fn test_run_lifecycle_no_policy_returns_error() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE no_policy (id INT)")?;

    let result = db.run_partition_lifecycle("no_policy");
    assert!(result.is_err(), "lifecycle 정책 없으면 에러 반환이어야 함");
    Ok(())
}
