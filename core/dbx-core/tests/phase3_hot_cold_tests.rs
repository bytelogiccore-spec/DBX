use dbx_core::engine::Database;
use dbx_core::error::DbxResult;
use dbx_core::storage::partition::{PartitionMap, PartitionTierHint, PartitionType};

#[test]
fn test_set_and_get_tier_hint() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE orders (id INT, ts INT)")?;

    let map = PartitionMap {
        table: "orders".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 3,
        },
        num_partitions: 3,
    };
    db.create_partition(map)?;

    // 최근 파티션 → Hot
    db.set_partition_tier("orders", "orders__p_part_0", PartitionTierHint::Hot)?;
    // 중간 파티션 → Warm
    db.set_partition_tier("orders", "orders__p_part_1", PartitionTierHint::Warm)?;
    // 오래된 파티션 → Cold
    db.set_partition_tier("orders", "orders__p_part_2", PartitionTierHint::Cold)?;

    assert_eq!(
        db.get_partition_tier("orders", "orders__p_part_0")?,
        PartitionTierHint::Hot
    );
    assert_eq!(
        db.get_partition_tier("orders", "orders__p_part_1")?,
        PartitionTierHint::Warm
    );
    assert_eq!(
        db.get_partition_tier("orders", "orders__p_part_2")?,
        PartitionTierHint::Cold
    );
    Ok(())
}

#[test]
fn test_list_hot_partitions() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE sensor (id INT)")?;

    let map = PartitionMap {
        table: "sensor".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 4,
        },
        num_partitions: 4,
    };
    db.create_partition(map)?;

    db.set_partition_tier("sensor", "sensor__p_part_0", PartitionTierHint::Hot)?;
    db.set_partition_tier("sensor", "sensor__p_part_1", PartitionTierHint::Hot)?;
    db.set_partition_tier("sensor", "sensor__p_part_2", PartitionTierHint::Cold)?;
    db.set_partition_tier("sensor", "sensor__p_part_3", PartitionTierHint::Cold)?;

    let hot = db.list_partitions_by_tier("sensor", PartitionTierHint::Hot)?;
    assert_eq!(hot.len(), 2, "Hot 파티션은 2개여야 함");

    let cold = db.list_partitions_by_tier("sensor", PartitionTierHint::Cold)?;
    assert_eq!(cold.len(), 2, "Cold 파티션은 2개여야 함");

    let warm = db.list_partitions_by_tier("sensor", PartitionTierHint::Warm)?;
    assert_eq!(warm.len(), 0, "Warm 파티션은 0개여야 함");
    Ok(())
}

#[test]
fn test_tier_hint_default_is_hot() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE t (id INT)")?;

    // 명시적 설정 없으면 Hot 반환
    let tier = db.get_partition_tier("t", "t__p_part_0")?;
    assert_eq!(tier, PartitionTierHint::Hot, "기본값은 Hot이어야 함");
    Ok(())
}

#[test]
fn test_tier_hint_overwrite() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE archive_table (id INT)")?;

    db.set_partition_tier("archive_table", "archive_table__p_part_0", PartitionTierHint::Hot)?;
    // Hot → Cold로 변경
    db.set_partition_tier("archive_table", "archive_table__p_part_0", PartitionTierHint::Cold)?;

    let tier = db.get_partition_tier("archive_table", "archive_table__p_part_0")?;
    assert_eq!(tier, PartitionTierHint::Cold, "Cold로 업데이트되어야 함");
    Ok(())
}

#[test]
fn test_list_partitions_empty_when_no_tier_set() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE empty_tier (id INT)")?;

    // 티어 힌트 설정 없이 목록 조회
    let hot = db.list_partitions_by_tier("empty_tier", PartitionTierHint::Hot)?;
    assert_eq!(hot.len(), 0, "명시적 설정이 없으면 목록이 비어야 함");
    Ok(())
}
