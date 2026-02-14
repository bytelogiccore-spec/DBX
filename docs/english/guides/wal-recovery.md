---
layout: default
title: WAL Recovery
parent: Examples
nav_order: 7
---

# WAL Recovery

DBX는 Write-Ahead Log (WAL)를 사용하여 크래시 복구와 데이터 무결성을 보장합니다.

## Overview

WAL의 주요 특징:
- **크래시 복구**: 시스템 장애 시 자동으로 데이터 복구
- **원자성 보장**: 트랜잭션의 All-or-Nothing 보장
- **순차 쓰기**: 디스크 I/O 최적화
- **체크포인트**: 주기적으로 WAL을 WOS에 반영

## How WAL Works

```
1. 쓰기 요청
   ↓
2. WAL에 기록 (순차 쓰기)
   ↓
3. 메모리에 반영
   ↓
4. 주기적으로 WOS에 플러시
   ↓
5. WAL 정리
```

## Quick Start

```rust
use dbx_core::Database;

// WAL은 자동으로 활성화됨
let db = Database::open("./db".as_ref())?;

// 데이터 삽입 (자동으로 WAL에 기록)
db.insert("users", b"user:1", b"Alice")?;

// 크래시 시뮬레이션
drop(db);

// 재시작 시 자동 복구
let db = Database::open("./db".as_ref())?;
let value = db.get("users", b"user:1")?;
assert_eq!(value, Some(b"Alice".to_vec()));
```

## Step-by-Step Guide

### 1. 기본 WAL 사용

WAL은 기본적으로 활성화되어 있습니다:

```rust
use dbx_core::Database;

// WAL이 자동으로 생성됨
let db = Database::open("./my_db".as_ref())?;

println!("✓ Database opened with WAL enabled");

// 데이터 삽입 (WAL에 기록)
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;

println!("✓ Data written to WAL");
```

### 2. 크래시 복구 시뮬레이션

시스템 장애를 시뮬레이션하고 복구합니다:

```rust
use dbx_core::Database;

// 1. 데이터 삽입
{
    let db = Database::open("./crash_test".as_ref())?;
    
    for i in 0..100 {
        let key = format!("key:{}", i).into_bytes();
        let value = format!("value:{}", i).into_bytes();
        db.insert("data", &key, &value)?;
    }
    
    println!("✓ Inserted 100 rows");
    
    // 2. 크래시 시뮬레이션 (flush 없이 종료)
    // drop(db) - WAL에만 기록되고 WOS에는 미반영
}

// 3. 재시작 및 자동 복구
{
    let db = Database::open("./crash_test".as_ref())?;
    
    // WAL 재생으로 데이터 복구
    let count = db.count("data")?;
    println!("✓ Recovered {} rows from WAL", count);
    
    // 데이터 검증
    let value = db.get("data", b"key:50")?;
    assert_eq!(value, Some(b"value:50".to_vec()));
    println!("✓ Data integrity verified");
}
```

### 3. 수동 플러시

WAL을 WOS에 수동으로 반영합니다:

```rust
use dbx_core::Database;

let db = Database::open("./db".as_ref())?;

// 데이터 삽입
for i in 0..1000 {
    let key = format!("key:{}", i).into_bytes();
    db.insert("data", &key, b"value")?;
}

println!("✓ Data in WAL");

// 수동 플러시
db.flush()?;

println!("✓ WAL flushed to WOS");
```

### 4. Durability 레벨 조정

데이터 안전성과 성능 간 트레이드오프를 조정합니다:

```rust
use dbx_core::{Database, DurabilityLevel};

// 최대 안전성 (모든 쓰기마다 fsync)
let db = Database::open_safe("./critical_db")?;

// 최고 성능 (WAL 없음, 크래시 시 데이터 손실 가능)
let db = Database::open_fast("./cache_db")?;

// 균형 (기본값, 주기적 fsync)
let db = Database::open_with_durability(
    "./balanced_db",
    DurabilityLevel::Lazy
)?;
```

## Complete Example

```rust
use dbx_core::{Database, DbxResult};

fn main() -> DbxResult<()> {
    println!("=== DBX WAL Recovery Example ===\n");
    
    // 1. 초기 데이터 삽입
    println!("--- Initial Write ---");
    {
        let db = Database::open("./wal_demo".as_ref())?;
        
        for i in 0..50 {
            let key = format!("user:{}", i).into_bytes();
            let value = format!("User {}", i).into_bytes();
            db.insert("users", &key, &value)?;
        }
        
        println!("✓ Inserted 50 users");
        println!("✓ Data in WAL (not yet flushed)\n");
        
        // 크래시 시뮬레이션 (drop without flush)
    }
    
    // 2. 재시작 및 복구
    println!("--- Recovery After Crash ---");
    {
        let db = Database::open("./wal_demo".as_ref())?;
        
        // WAL 자동 재생
        let count = db.count("users")?;
        println!("✓ Recovered {} users from WAL", count);
        
        // 데이터 검증
        let value = db.get("users", b"user:25")?;
        match value {
            Some(v) => println!("✓ Data verified: {}", String::from_utf8_lossy(&v)),
            None => println!("✗ Data lost!"),
        }
        
        println!();
    }
    
    // 3. 추가 데이터 및 플러시
    println!("--- Additional Write + Flush ---");
    {
        let db = Database::open("./wal_demo".as_ref())?;
        
        for i in 50..100 {
            let key = format!("user:{}", i).into_bytes();
            let value = format!("User {}", i).into_bytes();
            db.insert("users", &key, &value)?;
        }
        
        println!("✓ Inserted 50 more users");
        
        // 명시적 플러시
        db.flush()?;
        println!("✓ Flushed to WOS\n");
    }
    
    // 4. 최종 검증
    println!("--- Final Verification ---");
    {
        let db = Database::open("./wal_demo".as_ref())?;
        let count = db.count("users")?;
        println!("✓ Total users: {}", count);
        assert_eq!(count, 100);
    }
    
    println!("\n=== Example Complete ===");
    Ok(())
}
```

## Running the Example

```bash
cargo run --example simple_crud
```

## Expected Output

```
=== DBX WAL Recovery Example ===

--- Initial Write ---
✓ Inserted 50 users
✓ Data in WAL (not yet flushed)

--- Recovery After Crash ---
✓ Recovered 50 users from WAL
✓ Data verified: User 25

--- Additional Write + Flush ---
✓ Inserted 50 more users
✓ Flushed to WOS

--- Final Verification ---
✓ Total users: 100

=== Example Complete ===
```

## WAL Architecture

### 파일 구조

```
db/
├── wos/           # Write-Optimized Store
│   ├── conf
│   └── db
└── wal.log        # Write-Ahead Log
```

### WAL 레코드 타입

```rust
enum WalRecord {
    Insert { table, key, value, ts },
    Delete { table, key, ts },
    Batch { table, rows, ts },
    Checkpoint { ts },
}
```

## Durability Levels

| 레벨 | fsync 빈도 | 성능 | 안전성 | 권장 용도 |
|------|-----------|------|--------|----------|
| None | 없음 | ★★★★★ | ★☆☆☆☆ | 캐시, 임시 데이터 |
| Lazy | 주기적 | ★★★★☆ | ★★★★☆ | **기본값**, 일반 용도 |
| Full | 매 쓰기 | ★★☆☆☆ | ★★★★★ | 금융, 의료 데이터 |

## Best Practices

### 1. Durability 레벨 선택

```rust
// 중요한 데이터: Full
let db = Database::open_safe("./financial.db")?;

// 일반 데이터: Lazy (기본값)
let db = Database::open("./app.db".as_ref())?;

// 캐시: None
let db = Database::open_fast("./cache.db")?;
```

### 2. 주기적 플러시

```rust
use std::time::Duration;
use std::thread;

// 백그라운드 플러시 스레드
thread::spawn(move || {
    loop {
        thread::sleep(Duration::from_secs(60));
        let _ = db.flush();
    }
});
```

### 3. 체크포인트 관리

```rust
// 대량 삽입 후 체크포인트
for i in 0..1000000 {
    db.insert("data", &key, &value)?;
}
db.flush()?;  // WAL을 WOS에 반영
```

## Troubleshooting

### WAL 파일이 계속 커지는 경우

```rust
// 주기적으로 플러시하여 WAL 정리
db.flush()?;
```

### 복구 시간이 오래 걸리는 경우

```rust
// 더 자주 플러시하여 WAL 크기 감소
db.set_flush_interval(Duration::from_secs(30))?;
```

## Next Steps

- [Transactions](./transactions.md) - WAL과 MVCC 트랜잭션
- [Basic CRUD](./basic-crud.md) - 기본 데이터 작업
- [Encryption](./encryption.md) - 암호화된 WAL
