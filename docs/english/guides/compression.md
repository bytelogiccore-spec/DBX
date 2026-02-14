---
layout: default
title: Compression
parent: English
nav_order: 26
---

# Compression

DBX는 ZSTD 압축을 지원하여 저장 공간을 절약하고 I/O 성능을 향상시킵니다.

## Overview

DBX 압축의 주요 특징:
- **ZSTD 알고리즘**: 빠른 압축/해제 속도와 높은 압축률
- **투명한 압축**: 자동 압축/해제로 애플리케이션 코드 변경 불필요
- **조정 가능한 레벨**: 압축률과 속도 간 트레이드오프 조정
- **선택적 압축**: 테이블별로 압축 활성화/비활성화

## Quick Start

```rust
use dbx_core::Database;

let db = Database::open("./compressed_db".as_ref())?;

// 압축 활성화 (기본 레벨 3)
db.enable_compression("users")?;

// 데이터 삽입 (자동 압축)
db.insert("users", b"user:1", b"Large data...")?;

// 데이터 조회 (자동 해제)
let value = db.get("users", b"user:1")?;
```

## Compression Levels

ZSTD는 1-22 레벨의 압축을 지원합니다:

| 레벨 | 압축률 | 속도 | 권장 용도 |
|------|--------|------|----------|
| 1-3 | 낮음 | 매우 빠름 | 실시간 데이터, 로그 |
| 3-9 | 중간 | 빠름 | **기본값**, 일반적인 용도 |
| 10-15 | 높음 | 보통 | 아카이브, 백업 |
| 16-22 | 매우 높음 | 느림 | 장기 보관 |

## Step-by-Step Guide

### 1. 기본 압축 사용

기본 레벨(3)로 압축을 활성화합니다:

```rust
use dbx_core::Database;

let db = Database::open("./db".as_ref())?;

// 압축 활성화
db.enable_compression("logs")?;

println!("✓ Compression enabled for 'logs' table");

// 데이터 삽입 (자동 압축)
let large_data = vec![b'x'; 10000];  // 10KB 데이터
db.insert("logs", b"log:1", &large_data)?;

println!("✓ Data compressed and stored");
```

### 2. 압축 레벨 조정

압축률과 속도를 조정합니다:

```rust
// 빠른 압축 (레벨 1)
db.set_compression_level("realtime_logs", 1)?;

// 균형잡힌 압축 (레벨 6)
db.set_compression_level("user_data", 6)?;

// 최대 압축 (레벨 15)
db.set_compression_level("archives", 15)?;
```

### 3. 압축률 측정

압축 전후의 크기를 비교합니다:

```rust
use std::fs;

// 압축 전 크기
let uncompressed_size = 10000 * 1000;  // 10MB

// 데이터 삽입
for i in 0..1000 {
    let key = format!("data:{}", i).into_bytes();
    let value = vec![b'x'; 10000];  // 10KB each
    db.insert("data", &key, &value)?;
}

// 압축 후 디스크 사용량
let metadata = fs::metadata("./db/wos/db")?;
let compressed_size = metadata.len();

let ratio = uncompressed_size as f64 / compressed_size as f64;
println!("Compression ratio: {:.2}x", ratio);
println!("Space saved: {:.1}%", (1.0 - 1.0/ratio) * 100.0);
```

### 4. 선택적 압축

테이블별로 압축을 선택적으로 적용합니다:

```rust
// 대용량 데이터는 압축
db.enable_compression("logs")?;
db.enable_compression("documents")?;

// 소량 데이터는 압축 안 함 (오버헤드 방지)
// "users", "sessions" 테이블은 압축 없음
```

## Complete Example

```rust
use dbx_core::{Database, DbxResult};
use std::time::Instant;

fn main() -> DbxResult<()> {
    println!("=== DBX Compression Example ===\n");
    
    let db = Database::open("./compression_test".as_ref())?;
    
    // 1. 압축 없이 데이터 삽입
    println!("--- Without Compression ---");
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("data:{}", i).into_bytes();
        let value = vec![b'A'; 1000];  // 1KB each
        db.insert("uncompressed", &key, &value)?;
    }
    let time_uncompressed = start.elapsed();
    println!("✓ Inserted 1000 rows in {:?}\n", time_uncompressed);
    
    // 2. 압축으로 데이터 삽입
    println!("--- With Compression (Level 3) ---");
    db.enable_compression("compressed")?;
    
    let start = Instant::now();
    for i in 0..1000 {
        let key = format!("data:{}", i).into_bytes();
        let value = vec![b'A'; 1000];  // 1KB each
        db.insert("compressed", &key, &value)?;
    }
    let time_compressed = start.elapsed();
    println!("✓ Inserted 1000 rows in {:?}\n", time_compressed);
    
    // 3. 압축률 비교
    println!("--- Compression Stats ---");
    println!("Original size: 1 MB");
    println!("Compressed size: ~100 KB (estimated)");
    println!("Compression ratio: ~10x");
    println!("Space saved: ~90%\n");
    
    // 4. 읽기 성능 비교
    println!("--- Read Performance ---");
    
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("data:{}", i).into_bytes();
        let _ = db.get("uncompressed", &key)?;
    }
    let read_uncompressed = start.elapsed();
    println!("Uncompressed read: {:?}", read_uncompressed);
    
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("data:{}", i).into_bytes();
        let _ = db.get("compressed", &key)?;
    }
    let read_compressed = start.elapsed();
    println!("Compressed read: {:?}", read_compressed);
    
    println!("\n=== Example Complete ===");
    Ok(())
}
```

## Performance Characteristics

### 압축률 (Compression Ratio)

실제 데이터에 따라 다르지만 일반적인 압축률:

| 데이터 타입 | 압축률 | 예시 |
|------------|--------|------|
| 텍스트 (반복) | 10-20x | 로그 파일 |
| JSON | 5-10x | API 응답 |
| 바이너리 | 2-5x | 이미지, 비디오 |
| 랜덤 데이터 | 1x | 암호화된 데이터 |

### 성능 영향

```
쓰기 성능: -10% ~ -20% (압축 오버헤드)
읽기 성능: -5% ~ -10% (해제 오버헤드)
디스크 I/O: +50% ~ +80% (압축된 데이터가 작아서)
```

## Best Practices

### 1. 압축 대상 선택

```rust
// ✅ GOOD: 대용량 텍스트 데이터
db.enable_compression("logs")?;
db.enable_compression("documents")?;

// ❌ BAD: 이미 압축된 데이터
// db.enable_compression("images")?;  // JPEG, PNG는 이미 압축됨
// db.enable_compression("videos")?;  // MP4는 이미 압축됨
```

### 2. 레벨 선택

```rust
// 실시간 데이터: 낮은 레벨
db.set_compression_level("realtime", 1)?;

// 일반 데이터: 기본 레벨
db.set_compression_level("users", 3)?;

// 아카이브: 높은 레벨
db.set_compression_level("archive", 15)?;
```

### 3. 메모리 vs 디스크

압축은 메모리를 절약하지만 CPU를 사용합니다:

```rust
// CPU가 충분하고 디스크가 부족한 경우
db.enable_compression("data")?;

// CPU가 부족하고 디스크가 충분한 경우
// 압축 비활성화 (기본값)
```

## Troubleshooting

### 압축률이 낮은 경우

```rust
// 데이터가 이미 압축되어 있거나 랜덤한 경우
// 압축을 비활성화하는 것이 더 효율적
db.disable_compression("random_data")?;
```

### 성능 저하

```rust
// 압축 레벨을 낮춰서 속도 향상
db.set_compression_level("data", 1)?;  // 레벨 3 → 1
```

## Next Steps

- [Encryption](./encryption.md) - 압축과 암호화 함께 사용
- [GPU Acceleration](./gpu-acceleration.md) - 압축된 데이터의 GPU 처리
- [CRUD Operations](./crud-operations.md) - 기본 데이터 작업
