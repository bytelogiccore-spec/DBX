---
layout: default
title: 스키마 빌더 (Schema Builder)
parent: 한국어
nav_order: 38
---

# 스키마 빌더 (Schema Builder)
{: .no_toc }

Apache Arrow 스키마를 편리하게 구성하기 위한 Fluent API를 제공합니다.
{: .fs-6 .fw-300 }

---

## 개요

`SchemaBuilder`를 사용하면 Arrow의 `Field` 객체를 수동으로 생성하고 `Schema`로 조합하는 복잡한 과정 없이, 체이닝 방식으로 직관적이고 안전하게 스키마를 정의할 수 있습니다.

---

## 기본 사용법

```rust
use dbx_core::SchemaBuilder;
use arrow::datatypes::DataType;

let schema = SchemaBuilder::new()
    .column("id", DataType::Int64, false)
    .column("name", DataType::Utf8, true)
    .build();
```

---

## Database 연동 (테이블 생성)

`Database` 객체의 `create_table_with_builder` 메서드를 사용하면 스키마 빌더로 테이블을 간편하게 생성할 수 있습니다.

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// 스키마 빌더를 이용한 테이블 생성
db.create_table_with_builder("users", |builder| {
    builder
        .id("id")
        .text("name").not_null()
        .text("email").not_null()
        .int32("age")
})?;
```

---

## 편의 메서드

자주 사용하는 데이터 타입에 대해서는 별도의 편의 메서드가 제공됩니다:

- `id(name)`: `Int64`, NOT NULL
- `text(name)`: `Utf8`, 기본값 Nullable
- `int32(name)`: `Int32`, 기본값 Nullable
- `int64(name)`: `Int64`, 기본값 Nullable
- `float64(name)`: `Float64`, 기본값 Nullable
- `boolean(name)`: `Boolean`, 기본값 Nullable
- `timestamp(name)`: `Timestamp(Millisecond)`, 기본값 Nullable

```rust
let schema = SchemaBuilder::new()
    .id("user_id")
    .text("username")
    .int32("age")
    .boolean("is_active")
    .timestamp("created_at")
    .build();
```

---

## Null 제어 (Nullability)

편의 메서드로 생성된 컬럼은 `id`를 제외하고 기본적으로 Nullable(NULL 허용)입니다. 이를 변경하려면 `not_null()` 또는 `nullable()` 메서드를 사용합니다.

```rust
let schema = SchemaBuilder::new()
    .text("email").not_null()     // 이메일은 필수 (NOT NULL)
    .text("nickname").nullable()  // 닉네임은 선택 (Nullable)
    .build();
```

---

## 다음 단계

- [쿼리 빌더](query-builder) — 데이터를 조회하기 위한 Fluent API
- [SQL 레퍼런스](sql-reference) — 데이터베이스 내의 표준 SQL 사용법
