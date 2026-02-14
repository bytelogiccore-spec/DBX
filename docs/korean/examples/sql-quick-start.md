---
layout: default
title: SQL Quick Start
parent: 한국어
nav_order: 12
---

# SQL Quick Start

DBX에서 SQL을 사용하는 가장 빠른 방법입니다.

## 1. 테이블 생성

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

db.execute_sql("CREATE TABLE users (
    id INT,
    name TEXT,
    age INT
)")?;
```

## 2. 데이터 삽입

```rust
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 30)")?;
db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 25)")?;
db.execute_sql("INSERT INTO users VALUES (3, 'Charlie', 35)")?;
```

## 3. 기본 쿼리

```rust
// 모든 데이터 조회
let results = db.execute_sql("SELECT * FROM users")?;

// 조건 필터링
let results = db.execute_sql("SELECT * FROM users WHERE age > 25")?;

// 정렬
let results = db.execute_sql("SELECT * FROM users ORDER BY age DESC")?;
```

## 4. 집계 함수

```rust
// 총 개수
let count = db.execute_sql("SELECT COUNT(*) FROM users")?;

// 평균 나이
let avg = db.execute_sql("SELECT AVG(age) FROM users")?;

// 그룹별 집계
let results = db.execute_sql("
    SELECT age, COUNT(*) as count
    FROM users
    GROUP BY age
")?;
```

## 5. 완전한 예제

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // 테이블 생성
    db.execute_sql("CREATE TABLE products (
        id INT,
        name TEXT,
        price REAL
    )")?;
    
    // 데이터 삽입
    db.execute_sql("INSERT INTO products VALUES (1, 'Laptop', 999.99)")?;
    db.execute_sql("INSERT INTO products VALUES (2, 'Mouse', 29.99)")?;
    
    // 쿼리 실행
    let results = db.execute_sql("
        SELECT name, price 
        FROM products 
        WHERE price > 50
        ORDER BY price DESC
    ")?;
    
    println!("Found {} products", results.batches[0].num_rows());
    
    Ok(())
}
```

## Next Steps

- [**SQL Reference**](../guides/sql-reference.md) — 완전한 SQL 레퍼런스
- [**GPU Acceleration**](../guides/gpu-acceleration.md) — SQL 쿼리 가속화
- [**Indexing**](indexing.md) — 쿼리 최적화
