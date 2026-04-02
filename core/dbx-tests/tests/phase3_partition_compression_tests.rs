use dbx_core::engine::Database;
use dbx_core::error::DbxResult;
use dbx_core::storage::compression::{CompressionAlgorithm, CompressionConfig};
use dbx_core::storage::partition::{PartitionMap, PartitionType};

#[test]
fn test_set_and_get_partition_compression() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE events (id INT, ts INT)")?;

    let map = PartitionMap {
        table: "events".into(),
        partition_type: PartitionType::Hash {
            column: "id".into(),
            num_partitions: 2,
        },
        num_partitions: 2,
    };
    db.create_partition(map)?;

    // 최근 파티션 = 저압축 (level 3)
    db.set_partition_compression(
        "events",
        "events__p_part_0",
        CompressionConfig::zstd_level(3),
    )?;

    // 오래된 파티션 = 고압축 (level 9)
    db.set_partition_compression(
        "events",
        "events__p_part_1",
        CompressionConfig::zstd_level(9),
    )?;

    let config0 = db.get_partition_compression("events", "events__p_part_0")?;
    let config1 = db.get_partition_compression("events", "events__p_part_1")?;

    assert_eq!(config0.algorithm(), CompressionAlgorithm::Zstd);
    assert_eq!(config0.level(), Some(3));
    assert_eq!(config1.algorithm(), CompressionAlgorithm::Zstd);
    assert_eq!(config1.level(), Some(9));
    Ok(())
}

#[test]
fn test_partition_compression_default_fallback() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE t (id INT)")?;

    // 명시적 설정 없으면 기본값(Snappy) 반환
    let config = db.get_partition_compression("t", "t__p_part_0")?;
    assert_eq!(
        config.algorithm(),
        CompressionAlgorithm::Snappy,
        "기본 알고리즘은 Snappy"
    );
    Ok(())
}

#[test]
fn test_partition_compression_lz4() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE stream (id INT)")?;

    db.set_partition_compression("stream", "stream__p_part_0", CompressionConfig::lz4())?;

    let config = db.get_partition_compression("stream", "stream__p_part_0")?;
    assert_eq!(config.algorithm(), CompressionAlgorithm::Lz4);
    Ok(())
}

#[test]
fn test_partition_compression_overwrite() -> DbxResult<()> {
    let db = Database::open_in_memory()?;
    db.execute_sql("CREATE TABLE archive (id INT)")?;

    db.set_partition_compression("archive", "archive__p_part_0", CompressionConfig::snappy())?;
    // 고압축으로 업그레이드
    db.set_partition_compression(
        "archive",
        "archive__p_part_0",
        CompressionConfig::zstd_level(9),
    )?;

    let config = db.get_partition_compression("archive", "archive__p_part_0")?;
    assert_eq!(config.algorithm(), CompressionAlgorithm::Zstd);
    assert_eq!(config.level(), Some(9));
    Ok(())
}
