use dbx_core::engine::Database;
use dbx_core::error::DbxResult;
use dbx_core::storage::partition::PartitionLifecycle;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn test_enable_auto_archive_and_query() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE logs (id INT, ts INT)")?;

    db.enable_auto_archive(
        "logs",
        PartitionLifecycle {
            archive_after_days: 90,
            delete_after_days: 365,
        },
    )?;

    let lc = db.get_partition_lifecycle("logs")?;
    assert_eq!(lc.archive_after_days, 90);
    assert_eq!(lc.delete_after_days, 365);
    Ok(())
}

#[test]
fn test_should_archive_partition() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE events (id INT)")?;

    db.enable_auto_archive(
        "events",
        PartitionLifecycle {
            archive_after_days: 90,
            delete_after_days: 365,
        },
    )?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 91일 전 타임스탬프 = 아카이브 대상
    let old_ts = now - (91 * 24 * 3600);
    assert!(
        db.partition_needs_archive("events", old_ts)?,
        "91일된 파티션은 아카이브 대상이어야 함"
    );

    // 10일 전 타임스탬프 = 아카이브 불필요
    let recent_ts = now - (10 * 24 * 3600);
    assert!(
        !db.partition_needs_archive("events", recent_ts)?,
        "10일된 파티션은 아카이브 대상이 아니어야 함"
    );
    Ok(())
}

#[test]
fn test_should_delete_partition() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE old_data (id INT)")?;

    db.enable_auto_archive(
        "old_data",
        PartitionLifecycle {
            archive_after_days: 90,
            delete_after_days: 365,
        },
    )?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 366일 전 = 삭제 대상
    let very_old_ts = now - (366 * 24 * 3600);
    assert!(
        db.partition_needs_delete("old_data", very_old_ts)?,
        "366일된 파티션은 삭제 대상이어야 함"
    );

    // 200일 전 = 삭제 불필요 (아카이브 대상)
    let ok_ts = now - (200 * 24 * 3600);
    assert!(
        !db.partition_needs_delete("old_data", ok_ts)?,
        "200일된 파티션은 삭제 대상이 아니어야 함"
    );
    Ok(())
}

#[test]
fn test_lifecycle_missing_returns_error() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE t (id INT)")?;

    let result = db.get_partition_lifecycle("t");
    assert!(result.is_err(), "lifecycle 미설정 시 에러 반환이어야 함");
    Ok(())
}

#[test]
fn test_lifecycle_boundary_exact_days() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE boundary (id INT)")?;

    db.enable_auto_archive(
        "boundary",
        PartitionLifecycle {
            archive_after_days: 30,
            delete_after_days: 90,
        },
    )?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 정확히 30일 전 → 아카이브 경계값
    let boundary_ts = now - (30 * 24 * 3600);
    // 경계값 포함(>=)이므로 true
    assert!(db.partition_needs_archive("boundary", boundary_ts)?);
    Ok(())
}
