---
layout: default
title: API Reference
nav_order: 7
parent: English
---

# API Reference

## Rust API Documentation

The complete Rust API documentation is available on **docs.rs**:

[![docs.rs](https://docs.rs/dbx-core/badge.svg)](https://docs.rs/dbx-core)

**[→ View Full API Documentation on docs.rs](https://docs.rs/dbx-core)**

---

## Quick Reference

### Core Types

| Type | Description |
|------|-------------|
| [`Database`](https://docs.rs/dbx-core/latest/dbx_core/struct.Database.html) | Main database handle |
| [`Transaction`](https://docs.rs/dbx-core/latest/dbx_core/transaction/struct.Transaction.html) | MVCC transaction |
| [`Table`](https://docs.rs/dbx-core/latest/dbx_core/derive.Table.html) | Derive macro for schema |

### Key Methods

```rust
// Database operations
Database::open(path) -> Result<Database>
Database::open_in_memory() -> Result<Database>
db.insert(table, key, value) -> Result<()>
db.get(table, key) -> Result<Option<Vec<u8>>>
db.delete(table, key) -> Result<()>

// SQL interface
db.execute_sql(sql) -> Result<SqlResult>

// Transactions
db.begin_transaction() -> Result<Transaction>
tx.commit() -> Result<()>
tx.rollback() -> Result<()>
```

### Feature Flags

| Flag | Description |
|------|-------------|
| `simd` | SIMD-accelerated operations |
| `gpu` | GPU acceleration via CUDA |
| `logging` | Enable tracing output |

---

## Other Language Bindings

- [.NET API Reference](../packages/dotnet#api-reference)
- [Python API Reference](../packages/python#api-reference)
- [Node.js API Reference](../packages/nodejs#api-reference)
- [C/C++ API Reference](../packages/cpp#c-api-reference)
