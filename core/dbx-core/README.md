# dbx-core

[![Crates.io](https://img.shields.io/crates/v/dbx-core.svg)](https://crates.io/crates/dbx-core)
[![docs.rs](https://docs.rs/dbx-core/badge.svg)](https://docs.rs/dbx-core)
[![License](https://img.shields.io/crates/l/dbx-core.svg)](LICENSE)
[![Guide](https://img.shields.io/badge/guide-GitHub%20Pages-blue)](https://bytelogiccore-spec.github.io/DBX/english/packages/rust)

> **29x faster** than SQLite • Pure Rust • MVCC Transactions • 5-Tier Hybrid Storage

**dbx-core** is a high-performance embedded database engine built on a 5-Tier Hybrid Storage architecture.

## Installation

```toml
[dependencies]
dbx-core = "0.0.1-beta"
```

## Quick Start

```rust
use dbx_core::Database;

fn main() -> dbx_core::error::DbxResult<()> {
    // Open an in-memory database
    let db = Database::open_in_memory()?;

    // Insert data
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;

    // Get data
    if let Some(value) = db.get("users", b"user:1")? {
        println!("{}", String::from_utf8_lossy(&value));
    }

    // Delete data
    db.delete("users", b"user:2")?;

    Ok(())
}
```

## SQL Interface

```rust
use dbx_core::Database;

fn main() -> dbx_core::error::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // SQL DDL & DML
    db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT, email TEXT)")?;
    db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com')")?;

    // Query
    let result = db.execute_sql("SELECT * FROM users WHERE id = 1")?;
    println!("{:?}", result);

    Ok(())
}
```

## Features

- **5-Tier Hybrid Storage**: WOS → L0 → L1 → L2 → Cold Storage
- **MVCC Transactions**: Snapshot isolation with optimistic concurrency
- **SQL Engine**: CREATE TABLE, INSERT, SELECT, UPDATE, DELETE
- **WAL**: Write-Ahead Logging for crash recovery
- **Encryption**: AES-GCM-SIV and ChaCha20-Poly1305
- **Arrow/Parquet**: Native columnar format support

## Feature Flags

| Flag | Description |
|------|-------------|
| `simd` | SIMD-accelerated operations |
| `gpu` | GPU acceleration via CUDA |
| `logging` | Enable tracing output |

## License

Dual-licensed under:
- **MIT License** — for open-source projects
- **Commercial License** — for proprietary/commercial use

See [LICENSE](https://github.com/bytelogiccore-spec/DBX/blob/main/LICENSE) for details.

For commercial licensing inquiries, contact: [ByteLogicCore](https://github.com/bytelogiccore-spec)
