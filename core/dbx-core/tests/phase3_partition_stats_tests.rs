use dbx_core::engine::Database;
use dbx_core::error::DbxResult;
use dbx_core::storage::partition::{PartitionMap, PartitionStats, PartitionType};

#[test]
fn test_partition_stats_update_and_query() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE orders (id INT, amount INT)")?;

    let map = PartitionMap {
        table: "orders".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 3,
        },
        num_partitions: 3,
    };
    db.create_partition(map)?;

    db.update_partition_stats(
        "orders",
        "orders__p_part_0",
        PartitionStats {
            row_count: 1000,
            min_value: 0,
            max_value: 999,
            null_count: 0,
            distinct_count: 1000,
        },
    )?;

    let stats = db.get_partition_stats("orders", "orders__p_part_0")?;
    assert_eq!(stats.row_count, 1000);
    assert_eq!(stats.min_value, 0);
    assert_eq!(stats.max_value, 999);
    assert_eq!(stats.null_count, 0);
    assert_eq!(stats.distinct_count, 1000);
    Ok(())
}

#[test]
fn test_all_partition_stats() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE logs (id INT, msg TEXT)")?;

    let map = PartitionMap {
        table: "logs".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 2,
        },
        num_partitions: 2,
    };
    db.create_partition(map)?;

    db.update_partition_stats(
        "logs",
        "logs__p_part_0",
        PartitionStats {
            row_count: 500,
            min_value: 0,
            max_value: 499,
            null_count: 5,
            distinct_count: 490,
        },
    )?;
    db.update_partition_stats(
        "logs",
        "logs__p_part_1",
        PartitionStats {
            row_count: 600,
            min_value: 500,
            max_value: 1099,
            null_count: 0,
            distinct_count: 600,
        },
    )?;

    let all = db.all_partition_stats("logs")?;
    assert_eq!(all.len(), 2);
    let total: usize = all.values().map(|s| s.row_count).sum();
    assert_eq!(total, 1100);
    Ok(())
}

#[test]
fn test_partition_stats_missing_returns_error() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE t (id INT)")?;

    let result = db.get_partition_stats("t", "t__p_part_0");
    assert!(result.is_err(), "미설정 파티션 통계 조회는 에러여야 함");
    Ok(())
}

#[test]
fn test_partition_stats_update_overwrites() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE data (id INT)")?;

    db.update_partition_stats(
        "data",
        "data__p_part_0",
        PartitionStats { row_count: 100, ..Default::default() },
    )?;
    // 덮어쓰기
    db.update_partition_stats(
        "data",
        "data__p_part_0",
        PartitionStats { row_count: 200, ..Default::default() },
    )?;

    let stats = db.get_partition_stats("data", "data__p_part_0")?;
    assert_eq!(stats.row_count, 200, "마지막 값으로 덮어쓰여야 함");
    Ok(())
}
