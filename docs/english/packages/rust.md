---
layout: default
title: Rust (dbx-core)
parent: Packages
grand_parent: English
nav_order: 1
---

# Rust — dbx-core

[![Crates.io](https://img.shields.io/crates/v/dbx-core.svg)](https://crates.io/crates/dbx-core)
[![docs.rs](https://docs.rs/dbx-core/badge.svg)](https://docs.rs/dbx-core)

The core Rust crate for DBX — a high-performance embedded database with 5-Tier Hybrid Storage.

## Installation

```toml
[dependencies]
dbx-core = "0.0.2-beta"
```

## Quick Start

```rust
use dbx_core::Database;

fn main() -> dbx_core::error::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Insert
    db.insert("users", b"user:1", b"Alice")?;

    // Get
    if let Some(value) = db.get("users", b"user:1")? {
        println!("{}", String::from_utf8_lossy(&value));
    }

    // Delete
    db.delete("users", b"user:1")?;

    Ok(())
}
```

## SQL Interface

```rust
let db = Database::open_in_memory()?;

db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT, email TEXT)")?;
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com')")?;

let result = db.execute_sql("SELECT * FROM users WHERE id = 1")?;
println!("{:?}", result);
```

## Derive Macro

Use `#[derive(Table)]` for automatic schema generation:

```rust
use dbx_derive::Table;

#[derive(Table)]
#[dbx(table_name = "users")]
pub struct User {
    #[dbx(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}

// Auto-generated:
// User::TABLE_NAME -> "users"
// User::schema() -> Arrow Schema
// FromRow trait implementation
```

## Features

| Feature | Description |
|---------|-------------|
| 5-Tier Storage | WOS → L0 → L1 → L2 → Cold Storage |
| MVCC | Snapshot isolation transactions |
| SQL Engine | DDL + DML support |
| WAL | Crash recovery |
| Encryption | AES-GCM-SIV, ChaCha20-Poly1305 |
| Arrow/Parquet | Native columnar format |

## Feature Flags

```toml
dbx-core = { version = "0.0.2-beta", features = ["simd", "logging"] }
```

| Flag | Description |
|------|-------------|
| `simd` | SIMD-accelerated operations |
| `gpu` | GPU acceleration via CUDA |
| `logging` | Enable tracing output |

## API Documentation

Full API docs: [docs.rs/dbx-core](https://docs.rs/dbx-core)
