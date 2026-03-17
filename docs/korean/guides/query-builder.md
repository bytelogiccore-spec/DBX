---
layout: default
title: 쿼리 빌더 (Query Builder)
parent: 한국어
nav_order: 37
---

# 쿼리 빌더 및 파라미터 바인딩
{: .no_toc }

안전하고 편리하게 SQL을 구성하고 파라미터를 바인딩할 수 있는 API를 제공합니다.
{: .fs-6 .fw-300 }

---

## 1. 파라미터 바인딩 (Parameterized Queries)

문자열 보간(String interpolation) 대신 파라미터 바인딩을 사용하면 SQL Injection을 방지하고 성능을 향상시킬 수 있습니다. DBX는 Positional 파라미터와 Named 파라미터를 모두 지원합니다.

### Positional 바인딩 (`$1`, `$2`, ...)

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// 여러 행 반환
let users = db.query("SELECT * FROM users WHERE age > $1 AND status = $2")
    .bind(18)
    .bind("active")
    .fetch_all()?;

// 단일 행 반환
let user = db.query_one("SELECT * FROM users WHERE id = $1")
    .bind(42)
    .fetch()?;
```

### Named 바인딩 (`:name`, `:age`, ...)

```rust
let users = db.query("SELECT * FROM users WHERE age > :min_age AND role = :role")
    .param("min_age", 18)
    .param("role", "admin")
    .fetch_all()?;
```

### 지원되는 실행 메서드

- `fetch_all()`: 쿼리 조건에 맞는 모든 행을 반환합니다 (`Vec<T>`).
- `fetch_first()`: 조건에 맞는 첫 번째 행을 반환합니다 (`Option<T>`).
- `fetch()`: 
  - `query_one()` 사용 시 정확히 1개의 행을 반환하며, 없거나 많으면 에러를 발생시킵니다.
  - `query_optional()` 사용 시 1개 또는 0개(`Option<T>`)를 반환합니다.
- `execute().run()`: INSERT, UPDATE, DELETE 쿼리에 대해 영향받은 행 수를 반환합니다.

```rust
// INSERT 실행
let affected_rows = db.execute("INSERT INTO users (id, name) VALUES ($1, $2)")
    .bind(1)
    .bind("Alice")
    .run()?;
```

---

## 2. Fluent 쿼리 빌더 (QueryBuilder)

SQL 문자열을 직접 작성하지 않고 체이닝 방식으로 쿼리를 구성할 수 있습니다. 동적으로 쿼리를 생성해야 할 때 유용합니다.

### 기본 조회

```rust
let results = db.query_builder()
    .select(&["id", "name", "email"])
    .from("users")
    .where_("age", ">=", "18")
    .and("status", "=", "'active'")
    .order_by("created_at", "DESC")
    .limit(10)
    .offset(20)
    .execute()?;
```

### 조인 (JOIN)

```rust
let results = db.query_builder()
    .select(&["users.name", "orders.amount"])
    .from("users")
    .inner_join("orders", "users.id", "orders.user_id")
    .execute()?;
```

### 집계 함수

```rust
// COUNT
let count_results = db.query_builder()
    .from("orders")
    .count("*")
    .execute()?;

// SUM
let sum_results = db.query_builder()
    .from("orders")
    .sum("amount")
    .execute()?;
```

지원하는 집계 함수: `count`, `sum`, `avg`, `min`, `max`

---

## 다음 단계

- [스키마 빌더](schema-builder) — 타입 안전한 테이블 스키마 생성
- [SQL 레퍼런스](sql-reference) — DBX의 전체 SQL 지원 범위 확인
