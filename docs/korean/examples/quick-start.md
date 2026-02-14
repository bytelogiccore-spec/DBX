---
layout: default
title: Quick Start
parent: Examples
nav_order: 1
---

# Quick Start

5분 안에 DBX 시작하기 — 가장 빠른 시작 가이드입니다.

## 1. 데이터베이스 열기

```rust
use dbx_core::Database;

// 인메모리 데이터베이스 (빠른 테스트용)
let db = Database::open_in_memory()?;

// 또는 영구 저장소
let db = Database::open("./mydata")?;
```

## 2. 데이터 삽입

```rust
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;
db.insert("users", b"user:3", b"Charlie")?;
```

## 3. 데이터 조회

```rust
let value = db.get("users", b"user:1")?;
match value {
    Some(v) => println!("Found: {}", String::from_utf8_lossy(&v)),
    None => println!("Not found"),
}
```

## 4. 데이터 삭제

```rust
db.delete("users", b"user:1")?;
```

## 5. 완전한 예제

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create
    db.insert("users", b"user:1", b"Alice")?;
    
    // Read
    if let Some(value) = db.get("users", b"user:1")? {
        println!("User: {}", String::from_utf8_lossy(&value));
    }
    
    // Update (upsert)
    db.insert("users", b"user:1", b"Alice Smith")?;
    
    // Delete
    db.delete("users", b"user:1")?;
    
    Ok(())
}
```

## Next Steps

- [**CRUD Operations Guide**](../guides/crud-operations.md) — 완전한 CRUD 가이드
- [**SQL Quick Start**](sql-quick-start.md) — SQL 기본 사용법
- [**Transactions**](../guides/transactions.md) — 트랜잭션 사용법
