---
layout: default
title: Indexing
parent: Examples
nav_order: 4
---

# Indexing Quick Start

DBX에서 인덱스를 사용하는 가장 빠른 방법입니다.

## 1. Bloom Filter 인덱스 생성

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// Bloom Filter 인덱스 생성
db.create_index("users", "email")?;
```

## 2. 인덱스를 활용한 빠른 조회

```rust
// 인덱스 없이 조회 (느림)
let value = db.get("users", b"user:1")?;

// 인덱스로 조회 (빠름)
db.create_index("users", "id")?;
let value = db.get("users", b"user:1")?;  // Bloom Filter 활용
```

## 3. 인덱스 재구축

```rust
// 인덱스 재구축 (데이터 변경 후)
db.rebuild_index("users")?;
```

## 4. 인덱스 통계 확인

```rust
// 인덱스 정보 조회
let stats = db.index_stats("users")?;
println!("Index size: {} bytes", stats.size);
println!("False positive rate: {:.4}%", stats.fpr * 100.0);
```

## 5. 완전한 예제

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./indexed_db")?;
    
    // 데이터 삽입
    for i in 0..10000 {
        let key = format!("user:{}", i).into_bytes();
        let value = format!("User {}", i).into_bytes();
        db.insert("users", &key, &value)?;
    }
    
    // Bloom Filter 인덱스 생성
    db.create_index("users", "id")?;
    
    println!("✓ Index created for 10,000 users");
    
    // 빠른 조회
    let value = db.get("users", b"user:5000")?;
    println!("✓ Fast lookup: {:?}", value);
    
    Ok(())
}
```

## Next Steps

- [**Indexing Guide**](../guides/indexing.md) — 완전한 인덱싱 가이드
- [**SQL Quick Start**](sql-quick-start.md) — SQL 쿼리 최적화
- [**Quick Start**](quick-start.md) — 기본 CRUD
