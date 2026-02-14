---
layout: default
title: Compression
parent: 한국어
nav_order: 14
---

# Compression Quick Start

DBX에서 압축을 사용하는 가장 빠른 방법입니다.

## 1. 압축 활성화

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// 압축 활성화 (기본 레벨 3)
db.enable_compression("logs")?;
```

## 2. 데이터 삽입

```rust
// 데이터 삽입 (자동 압축)
let large_data = vec![b'x'; 10000];  // 10KB
db.insert("logs", b"log:1", &large_data)?;
```

## 3. 압축 레벨 조정

```rust
// 빠른 압축 (레벨 1)
db.set_compression_level("realtime", 1)?;

// 균형 압축 (레벨 6)
db.set_compression_level("data", 6)?;

// 최대 압축 (레벨 15)
db.set_compression_level("archive", 15)?;
```

## 4. 압축률 측정

```rust
use std::fs;

// 데이터 삽입
for i in 0..1000 {
    let key = format!("data:{}", i).into_bytes();
    let value = vec![b'A'; 1000];  // 1KB each
    db.insert("data", &key, &value)?;
}

// 디스크 사용량 확인
let metadata = fs::metadata("./db/wos/db")?;
let compressed_size = metadata.len();

println!("Compressed size: {} bytes", compressed_size);
```

## 5. 완전한 예제

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./compressed_db")?;
    
    // 압축 활성화
    db.enable_compression("logs")?;
    
    // 대용량 데이터 삽입
    for i in 0..1000 {
        let key = format!("log:{}", i).into_bytes();
        let value = vec![b'A'; 1000];  // 1KB each
        db.insert("logs", &key, &value)?;
    }
    
    println!("✓ 1000 rows compressed and stored");
    println!("✓ Compression ratio: ~10x (estimated)");
    println!("✓ Space saved: ~90%");
    
    Ok(())
}
```

## Next Steps

- [**Compression Guide**](../guides/compression.md) — 완전한 압축 가이드
- [**Encryption**](encryption.md) — 데이터 암호화
- [**Quick Start**](quick-start.md) — 기본 CRUD
