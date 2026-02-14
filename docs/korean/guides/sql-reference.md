---
layout: default
title: SQL 레퍼런스
parent: Guides
nav_order: 3
description: "DBX를 위한 전체 SQL 구문 레퍼런스"
---

# SQL 레퍼런스
{: .no_toc }

DBX의 SQL 쿼리에 대한 전체 레퍼런스입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX는 Apache Arrow 및 DataFusion 통합을 통해 표준 SQL 쿼리를 지원합니다. SQL 쿼리는 최적의 분석 성능을 위해 **Columnar Cache(컬럼형 캐시)** 계층에서 작동합니다.

### 지원 기능

- ✅ **SELECT** - 컬럼 프로젝션 및 필터링
- ✅ **WHERE** - 조건 필터링
- ✅ **JOIN** - Inner, Left, Right, Full Outer 조인
- ✅ **GROUP BY** - 집계 및 그룹화
- ✅ **ORDER BY** - 결과 정렬
- ✅ **LIMIT** - 결과 개수 제한
- ✅ **집계 함수 (Aggregate Functions)** - SUM, COUNT, MIN, MAX, AVG
- ✅ **스칼라 함수 (Scalar Functions)** - 문자열, 수학, 날짜 함수

---

## 기본 쿼리

### SELECT 문

모든 컬럼 선택:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... 데이터가 포함된 테이블 등록 ...
    
    let results = db.execute_sql("SELECT * FROM users")?;
    
    Ok(())
}
```

특정 컬럼 선택:

```rust
let results = db.execute_sql("SELECT id, name, email FROM users")?;
```

컬럼 별칭 (Alias):

```rust
let results = db.execute_sql(
    "SELECT id AS user_id, name AS full_name FROM users"
)?;
```

### WHERE 절

기본 필터링:

```rust
let results = db.execute_sql(
    "SELECT * FROM users WHERE age > 30"
)?;
```

다중 조건:

```rust
let results = db.execute_sql(
    "SELECT * FROM users WHERE age > 30 AND city = 'Seoul'"
)?;
```

비교 연산자:

```rust
// 같음
"SELECT * FROM users WHERE status = 'active'"

// 같지 않음
"SELECT * FROM users WHERE status != 'deleted'"

// 크다 / 작다
"SELECT * FROM orders WHERE amount > 1000"
"SELECT * FROM orders WHERE amount <= 500"

// LIKE 패턴 매칭
"SELECT * FROM users WHERE email LIKE '%@gmail.com'"

// IN 연산자
"SELECT * FROM users WHERE city IN ('Seoul', 'Busan', 'Incheon')"

// BETWEEN (범위)
"SELECT * FROM orders WHERE created_at BETWEEN '2024-01-01' AND '2024-12-31'"
```

### ORDER BY 절

오름차순 정렬:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY age ASC"
)?;
```

내림차순 정렬:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY created_at DESC"
)?;
```

다중 컬럼 정렬:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY city ASC, age DESC"
)?;
```

### LIMIT 절

결과 개수 제한:

```rust
let results = db.execute_sql(
    "SELECT * FROM users LIMIT 10"
)?;
```

Offset과 함께 사용:

```rust
let results = db.execute_sql(
    "SELECT * FROM users LIMIT 10 OFFSET 20"
)?;
```

---

## 집계 함수 (Aggregate Functions)

### COUNT

모든 행 개수:

```rust
let results = db.execute_sql(
    "SELECT COUNT(*) FROM users"
)?;
```

Null이 아닌 값 개수:

```rust
let results = db.execute_sql(
    "SELECT COUNT(email) FROM users"
)?;
```

중복 제거 후 개수:

```rust
let results = db.execute_sql(
    "SELECT COUNT(DISTINCT city) FROM users"
)?;
```

### SUM

숫자 컬럼 합계:

```rust
let results = db.execute_sql(
    "SELECT SUM(amount) FROM orders"
)?;
```

### AVG

평균값:

```rust
let results = db.execute_sql(
    "SELECT AVG(age) FROM users"
)?;
```

---

## GROUP BY

### 기본 그룹화

단일 컬럼 기준 그룹화:

```rust
let results = db.execute_sql(
    "SELECT city, COUNT(*) FROM users GROUP BY city"
)?;
```

다중 컬럼 기준 그룹화:

```rust
let results = db.execute_sql(
    "SELECT city, status, COUNT(*) 
     FROM users 
     GROUP BY city, status"
)?;
```

### HAVING 절

그룹화된 결과 필터링:

```rust
let results = db.execute_sql(
    "SELECT city, COUNT(*) as user_count
     FROM users 
     GROUP BY city
     HAVING user_count > 100"
)?;
```

---

## JOIN 연산

### INNER JOIN

두 테이블 조인:

```rust
let results = db.execute_sql(
    "SELECT u.id, u.name, o.order_id, o.amount
     FROM users u
     INNER JOIN orders o ON u.id = o.user_id"
 )?;
```

---

## 스칼라 함수 (Scalar Functions)

### 문자열 함수

```rust
"SELECT UPPER(name), LOWER(email) FROM users"
"SELECT name, LENGTH(name) FROM users"
"SELECT CONCAT(first_name, ' ', last_name) FROM users"
```

### 수학 함수

```rust
"SELECT ABS(balance) FROM accounts"
"SELECT ROUND(price, 2) FROM products"
```

---

## 쿼리 최적화

### Projection Pushdown

DBX는 저장소 계층에서 필요한 컬럼만 읽도록 자동으로 최적화합니다.

### Predicate Pushdown

WHERE 절의 필터가 스캔 단계에서 즉시 적용됩니다.

### 벡터화 실행 (Vectorized Execution)

쿼리 실행 시 SIMD 벡터화를 자동으로 활용합니다.

---

## GPU 가속 활용

GPU 기능이 활성화된 경우, 특정 작업들이 자동으로 가속화됩니다:

- 집계 (SUM, COUNT 등)
- 필터링 (WHERE 절)
- GROUP BY 작업
- Hash 조인

---

## 다음 단계

- [CRUD 작업](crud-operations) — 기본 데이터베이스 작업
- [트랜잭션](transactions) — SQL과 트랜잭션 함께 사용하기
- [GPU 가속](gpu-acceleration) — SQL 쿼리 가속화하기
- [API 레퍼런스](../api/sql) — 전체 SQL API 문서
