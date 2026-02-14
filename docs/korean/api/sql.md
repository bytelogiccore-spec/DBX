---
layout: default
title: SQL API
nav_order: 41
parent: 한국어
---

# SQL API
{: .no_toc }

DBX의 SQL 쿼리 실행을 위한 API 레퍼런스입니다.
{: .fs-6 .fw-300 }

---

## 개요

DBX는 Apache Arrow 기반의 쿼리 실행 엔진을 사용하여 복잡한 분석 쿼리를 지원합니다.

### 지원하는 주요 구문
- `SELECT`: 컬럼 선택 및 Alias
- `WHERE`: 비교 및 논리 연산 필터링
- `JOIN`: 내부 조인 (Inner Join)
- `GROUP BY`: 집계 연산 (`COUNT`, `SUM`, `AVG` 등)
- `ORDER BY`: 결과 정렬 (`ASC`, `DESC`)
- `LIMIT`: 결과 개수 제한

---

## 쿼리 실행

### `execute_sql(sql: &str) -> DbxResult<RecordBatch>`
SQL 쿼리를 실행하고 결과를 Arrow `RecordBatch` 형식으로 반환합니다.

```rust
let result = db.execute_sql("SELECT name, age FROM users WHERE age > 18")?;
```

---

## 결과 처리 (RecordBatch)

조회 결과는 Apache Arrow의 `RecordBatch` 형식으로 제공되므로, 메모리 효율적인 데이터 접근이 가능합니다.

```rust
use arrow::array::StringArray;

let result = db.execute_sql("SELECT name FROM users")?;
let name_col = result.column(0).as_any().downcast_ref::<StringArray>().unwrap();

for i in 0..result.num_rows() {
    println!("Name: {}", name_col.value(i));
}
```

---

## 쿼리 최적화

DBX는 다음과 같은 최적화를 자동으로 수행합니다:

1. **Projection Pushdown**: 필요한 컬럼만 선택적으로 로드
2. **Predicate Pushdown**: 필터 조건을 데이터 소스 단계에서 미리 적용
3. **GPU 가속**: 100만 건 이상의 대규모 데이터에 대해 자동으로 GPU를 활용하여 연산

---

## 주의 사항 및 한계
- **DML 제한**: 현재 SQL을 통한 `UPDATE` 및 `DELETE`는 지원하지 않습니다 (CRUD API를 사용하세요).
- **조인 유형**: `INNER JOIN`만 지원하며, `LEFT`/`RIGHT JOIN`은 향후 업데이트 예정입니다.
- **서브쿼리**: 아직 서브쿼리 구문은 지원하지 않습니다.

---

## 다음 단계
- [SQL 레퍼런스 가이드](../guides/sql-reference) — 상세 SQL 구문 및 예제
- [GPU 가속 가이드](../guides/gpu-acceleration) — GPU를 활용한 쿼리 최적화
- [데이터베이스 API](database) — 기본 CRUD 작업 API
