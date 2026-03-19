---
layout: default
title: Query Builder
parent: English
nav_order: 37
---

# Query Builder & Parameter Binding
{: .no_toc }

A safe and convenient API for constructing SQL queries and binding parameters.
{: .fs-6 .fw-300 }

---

## 1. Parameter Binding (Parameterized Queries)

Using parameter binding instead of string interpolation prevents SQL injection and improves performance. DBX supports both positional and named parameters.

### Positional Binding (`$1`, `$2`, ...)

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// Returns multiple rows
let users = db.query("SELECT * FROM users WHERE age > $1 AND status = $2")
    .bind(18)
    .bind("active")
    .fetch_all()?;

// Returns a single row
let user = db.query_one("SELECT * FROM users WHERE id = $1")
    .bind(42)
    .fetch()?;
```

### Named Binding (`:name`, `:age`, ...)

```rust
let users = db.query("SELECT * FROM users WHERE age > :min_age AND role = :role")
    .param("min_age", 18)
    .param("role", "admin")
    .fetch_all()?;
```

### Execution Methods

- `fetch_all()`: Returns all matching rows (`Vec<T>`).
- `fetch_first()`: Returns the first matching row (`Option<T>`).
- `fetch()`:
  - With `query_one()`: returns exactly 1 row; errors if none or multiple.
  - With `query_optional()`: returns 0 or 1 row (`Option<T>`).
- `execute().run()`: For INSERT/UPDATE/DELETE; returns affected row count.

```rust
// Execute INSERT
let affected_rows = db.execute("INSERT INTO users (id, name) VALUES ($1, $2)")
    .bind(1)
    .bind("Alice")
    .run()?;
```

---

## 2. Fluent Query Builder (QueryBuilder)

Build queries through method chaining without writing raw SQL strings. Useful for constructing dynamic queries.

### Basic Select

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

### JOIN

```rust
let results = db.query_builder()
    .select(&["users.name", "orders.amount"])
    .from("users")
    .inner_join("orders", "users.id", "orders.user_id")
    .execute()?;
```

### Aggregate Functions

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

Supported aggregates: `count`, `sum`, `avg`, `min`, `max`

---

## Next Steps

- [Schema Builder](schema-builder) — Type-safe table schema creation
- [SQL Reference](sql-reference) — Full SQL support in DBX
