---
layout: default
title: Indexing
parent: English
nav_order: 27
---

# Indexing

DBX는 Bloom Filter 기반 인덱스를 제공하여 쿼리 성능을 크게 향상시킵니다.

## Overview

DBX 인덱스의 주요 특징:
- **Bloom Filter 기반**: 메모리 효율적인 확률적 자료구조
- **빠른 조회**: O(1) 시간 복잡도로 존재 여부 확인
- **낮은 False Positive**: 정확도 99% 이상
- **자동 업데이트**: 데이터 삽입 시 자동으로 인덱스 갱신

## Quick Start

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// 인덱스 생성
db.create_index("users", "email")?;

// 데이터 삽입 (자동으로 인덱스 업데이트)
db.insert("users", b"user:1", b"alice@example.com")?;

// 빠른 조회
let row_ids = db.index_lookup("users", "email", b"alice@example.com")?;
```

## Step-by-Step Guide

### 1. 인덱스 생성

테이블과 컬럼에 대한 인덱스를 생성합니다:

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// 'users' 테이블의 'email' 컬럼에 인덱스 생성
db.create_index("users", "email")?;

println!("✓ Index created on users.email");
```

### 2. 데이터 삽입

데이터를 삽입하면 인덱스가 자동으로 업데이트됩니다:

```rust
// 데이터 삽입
for i in 0..1000 {
    let key = format!("user:{}", i).into_bytes();
    let email = format!("user{}@example.com", i).into_bytes();
    db.insert("users", &key, &email)?;
}

println!("✓ Inserted 1000 users (index auto-updated)");
```

### 3. 인덱스 조회

인덱스를 사용하여 빠르게 데이터를 찾습니다:

```rust
// 특정 이메일을 가진 사용자 찾기
let email = b"user500@example.com";
let row_ids = db.index_lookup("users", "email", email)?;

println!("✓ Found {} matching rows", row_ids.len());

// 실제 데이터 조회
for row_id in row_ids {
    let key = format!("user:{}", row_id).into_bytes();
    if let Some(value) = db.get("users", &key)? {
        println!("  - {}: {}", row_id, String::from_utf8_lossy(&value));
    }
}
```

### 4. 성능 비교

인덱스 사용 전후의 성능을 비교합니다:

```rust
use std::time::Instant;

// 인덱스 없이 조회 (전체 스캔)
let start = Instant::now();
let mut found = 0;
for i in 0..1000 {
    let key = format!("user:{}", i).into_bytes();
    if let Some(_) = db.get("users", &key)? {
        found += 1;
    }
}
let without_index = start.elapsed();
println!("Without index: {:?} ({} rows)", without_index, found);

// 인덱스로 조회
let start = Instant::now();
let row_ids = db.index_lookup("users", "email", b"user500@example.com")?;
let with_index = start.elapsed();
println!("With index: {:?} ({} rows)", with_index, row_ids.len());

let speedup = without_index.as_secs_f64() / with_index.as_secs_f64();
println!("✓ Speedup: {:.2}x faster", speedup);
```

## Complete Example

```rust
use dbx_core::{Database, DbxResult};
use std::time::Instant;

fn main() -> DbxResult<()> {
    println!("=== DBX Indexing Example ===\n");
    
    let db = Database::open_in_memory()?;
    
    // 1. 인덱스 생성
    println!("--- Creating Index ---");
    db.create_index("users", "email")?;
    println!("✓ Index created\n");
    
    // 2. 대량 데이터 삽입
    println!("--- Inserting Data ---");
    let start = Instant::now();
    for i in 0..10000 {
        let key = format!("user:{}", i).into_bytes();
        let email = format!("user{}@example.com", i).into_bytes();
        db.insert("users", &key, &email)?;
    }
    let insert_time = start.elapsed();
    println!("✓ Inserted 10,000 users in {:?}\n", insert_time);
    
    // 3. 인덱스 조회
    println!("--- Index Lookup ---");
    let start = Instant::now();
    let row_ids = db.index_lookup("users", "email", b"user5000@example.com")?;
    let lookup_time = start.elapsed();
    println!("✓ Found {} rows in {:?}\n", row_ids.len(), lookup_time);
    
    // 4. 실제 데이터 조회
    println!("--- Fetching Data ---");
    for row_id in &row_ids {
        let key = format!("user:{}", row_id).into_bytes();
        if let Some(value) = db.get("users", &key)? {
            println!("  Row {}: {}", row_id, String::from_utf8_lossy(&value));
        }
    }
    
    println!("\n=== Example Complete ===");
    Ok(())
}
```

## Running the Example

```bash
cargo run --example index_test
```

## Expected Output

```
=== DBX Indexing Example ===

--- Creating Index ---
✓ Index created

--- Inserting Data ---
✓ Inserted 10,000 users in 45ms

--- Index Lookup ---
✓ Found 1 rows in 12μs

--- Fetching Data ---
  Row 5000: user5000@example.com

=== Example Complete ===
```

## Performance Characteristics

### Bloom Filter 특성

| 작업 | 시간 복잡도 | 메모리 사용량 |
|------|------------|--------------|
| 인덱스 생성 | O(1) | ~10 bytes/key |
| 삽입 | O(1) | - |
| 조회 | O(1) | - |
| False Positive | < 1% | - |

### 성능 벤치마크

10,000개 행에서의 성능:

```
전체 스캔:    ~5ms
인덱스 조회:  ~12μs
속도 향상:    ~400x
```

## Best Practices

### 1. 인덱스 생성 시점

```rust
// ✅ GOOD: 데이터 삽입 전에 인덱스 생성
db.create_index("users", "email")?;
for i in 0..10000 {
    db.insert("users", &key, &value)?;
}

// ⚠️ OK: 데이터 삽입 후에도 가능 (재구축 필요)
for i in 0..10000 {
    db.insert("users", &key, &value)?;
}
db.create_index("users", "email")?;  // 기존 데이터 인덱싱
```

### 2. 인덱스 선택

자주 조회하는 컬럼에만 인덱스를 생성하세요:

```rust
// ✅ GOOD: WHERE 절에 자주 사용되는 컬럼
db.create_index("users", "email")?;
db.create_index("orders", "user_id")?;

// ❌ BAD: 거의 조회하지 않는 컬럼
db.create_index("logs", "timestamp")?;  // 불필요
```

### 3. 메모리 관리

인덱스는 메모리를 사용하므로 필요한 것만 생성하세요:

```rust
// 인덱스 메모리 사용량 추정
// ~10 bytes per key
// 1M keys = ~10MB
```

## Limitations

### False Positives

Bloom Filter는 확률적 자료구조이므로 False Positive가 발생할 수 있습니다:

```rust
// 인덱스 조회 결과는 "아마도 존재함"
let row_ids = db.index_lookup("users", "email", b"test@example.com")?;

// 실제 데이터 확인 필요
for row_id in row_ids {
    if let Some(value) = db.get("users", &key)? {
        // 실제로 존재하는 데이터
    }
}
```

### 지원하지 않는 기능

- 범위 쿼리 (range queries)
- 정렬 (sorting)
- 부분 일치 (partial matching)

## Next Steps

- [SQL Reference](./sql-reference.md) - SQL에서 인덱스 활용
- [GPU Acceleration](./gpu-acceleration.md) - GPU를 활용한 대용량 데이터 처리
- [CRUD Operations](./crud-operations.md) - 기본 데이터 작업
