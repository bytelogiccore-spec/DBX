// Phase 1 고급 기능 테스트: 압축 및 체크섬

use dbx_core::engine::SerializationRegistry;
use dbx_core::error::DbxResult;

#[test]
fn test_compression_zstd() -> DbxResult<()> {
    let registry = SerializationRegistry::new();

    // 원본 데이터
    let original = b"Hello, World! This is a test data for compression. ".repeat(100);

    // 압축
    let compressed = registry.compress(&original, 3)?; // 압축 레벨 3

    // 압축 확인
    println!("Original size: {} bytes", original.len());
    println!("Compressed size: {} bytes", compressed.len());
    println!(
        "Compression ratio: {:.2}%",
        (compressed.len() as f64 / original.len() as f64) * 100.0
    );

    assert!(
        compressed.len() < original.len(),
        "Compressed data should be smaller"
    );

    // 압축 해제
    let decompressed = registry.decompress(&compressed)?;

    // 검증
    assert_eq!(decompressed, original);

    Ok(())
}

#[test]
fn test_checksum_sha256() {
    let registry = SerializationRegistry::new();

    // 데이터
    let data = b"Hello, World!";

    // 체크섬 계산
    let checksum = registry.checksum(data);

    // 체크섬 길이 확인 (SHA256 = 32 bytes)
    assert_eq!(checksum.len(), 32);

    // 체크섬 검증
    assert!(registry.verify_checksum(data, &checksum));

    // 잘못된 체크섬 검증
    let wrong_checksum = vec![0u8; 32];
    assert!(!registry.verify_checksum(data, &wrong_checksum));
}

#[test]
fn test_compression_with_checksum() -> DbxResult<()> {
    let registry = SerializationRegistry::new();

    // 원본 데이터
    let original = b"Important data that needs integrity verification".repeat(50);

    // 압축
    let compressed = registry.compress(&original, 5)?;

    // 체크섬 계산 (압축된 데이터의 체크섬)
    let checksum = registry.checksum(&compressed);

    // 압축 해제
    let decompressed = registry.decompress(&compressed)?;

    // 체크섬 검증
    assert!(registry.verify_checksum(&compressed, &checksum));

    // 원본 데이터 검증
    assert_eq!(decompressed, original);

    Ok(())
}

#[test]
fn test_compression_levels() -> DbxResult<()> {
    let registry = SerializationRegistry::new();

    // 원본 데이터
    let original = b"Test data for compression level comparison. ".repeat(200);

    // 다양한 압축 레벨 테스트
    for level in [1, 3, 5, 10, 15] {
        let compressed = registry.compress(&original, level)?;
        let decompressed = registry.decompress(&compressed)?;

        println!(
            "Level {}: {} bytes -> {} bytes ({:.2}%)",
            level,
            original.len(),
            compressed.len(),
            (compressed.len() as f64 / original.len() as f64) * 100.0
        );

        assert_eq!(decompressed, original);
    }

    Ok(())
}
