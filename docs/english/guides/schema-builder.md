---
layout: default
title: Schema Builder
parent: English
nav_order: 38
---

# Schema Builder
{: .no_toc }

A fluent API for conveniently constructing Apache Arrow schemas.
{: .fs-6 .fw-300 }

---

## Overview

`SchemaBuilder` lets you define schemas intuitively through method chaining, eliminating the need to manually create Arrow `Field` objects and combine them into a `Schema`.

---

## Basic Usage

```rust
use dbx_core::SchemaBuilder;
use arrow::datatypes::DataType;

let schema = SchemaBuilder::new()
    .column("id", DataType::Int64, false)
    .column("name", DataType::Utf8, true)
    .build();
```

---

## Database Integration (Table Creation)

Use the `create_table_with_builder` method on `Database` for convenient table creation.

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// Create table with schema builder
db.create_table_with_builder("users", |builder| {
    builder
        .id("id")
        .text("name").not_null()
        .text("email").not_null()
        .int32("age")
})?;
```

---

## Convenience Methods

Shorthand methods are provided for common data types:

- `id(name)`: `Int64`, NOT NULL
- `text(name)`: `Utf8`, nullable by default
- `int32(name)`: `Int32`, nullable by default
- `int64(name)`: `Int64`, nullable by default
- `float64(name)`: `Float64`, nullable by default
- `boolean(name)`: `Boolean`, nullable by default
- `timestamp(name)`: `Timestamp(Millisecond)`, nullable by default

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

## Nullability Control

Columns created with convenience methods are nullable by default (except `id`). Use `not_null()` or `nullable()` to change this.

```rust
let schema = SchemaBuilder::new()
    .text("email").not_null()     // required field
    .text("nickname").nullable()  // optional field
    .build();
```

---

## Next Steps

- [Query Builder](query-builder) — Fluent API for querying data
- [SQL Reference](sql-reference) — Standard SQL usage in DBX
