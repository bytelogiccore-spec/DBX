---
layout: default
title: WAL Recovery
parent: Examples
nav_order: 5
---

# WAL Recovery Quick Start

DBX에서 WAL 복구를 사용하는 가장 빠른 방법입니다.

## 1. WAL 활성화

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// WAL은 기본적으로 활성화되어 있습니다
```

## 2. 내구성 수준 설정

```rust
use dbx_core::storage::wal::DurabilityLevel;

// 최대 성능 (메모리만)
db.set_durability(DurabilityLevel::None)?;

// 균형 (기본값)
db.set_durability(DurabilityLevel::Normal)?;

// 최대 안전성 (즉시 디스크)
db.set_durability(DurabilityLevel::Paranoid)?;
```

## 3. 크래시 복구 테스트

```rust
// 데이터 삽입
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;

// 강제 종료 시뮬레이션
drop(db);

// 재시작 (자동 복구)
let db = Database::open("./db")?;

// 데이터 확인
assert_eq!(db.get("users", b"user:1")?, Some(b"Alice".to_vec()));
assert_eq!(db.get("users", b"user:2")?, Some(b"Bob".to_vec()));
```

## 4. 수동 체크포인트

```rust
// WAL을 디스크에 강제 플러시
db.checkpoint()?;
```

## 5. 완전한 예제

```rust
use dbx_core::Database;
use dbx_core::storage::wal::DurabilityLevel;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./wal_test")?;
    
    // 최대 안전성 설정
    db.set_durability(DurabilityLevel::Paranoid)?;
    
    // 중요한 데이터 삽입
    db.insert("transactions", b"tx:1", b"Payment: $100")?;
    db.insert("transactions", b"tx:2", b"Payment: $200")?;
    
    println!("✓ Data written with WAL");
    
    // 체크포인트 (디스크 동기화)
    db.checkpoint()?;
    
    println!("✓ WAL checkpointed to disk");
    
    // 크래시 후에도 데이터 보존됨
    drop(db);
    let db = Database::open("./wal_test")?;
    
    assert!(db.get("transactions", b"tx:1")?.is_some());
    println!("✓ Data recovered after restart");
    
    Ok(())
}
```

## Next Steps

- [**WAL Recovery Guide**](../guides/wal-recovery.md) — 완전한 WAL 가이드
- [**Transactions**](../guides/transactions.md) — ACID 트랜잭션
- [**Quick Start**](quick-start.md) — 기본 CRUD
