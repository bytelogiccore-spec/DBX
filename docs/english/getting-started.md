---
layout: default
title: Getting Started
nav_order: 2
description: "Quick start guide for DBX database"
---

# Getting Started
{: .no_toc }

This guide will help you install DBX and run your first queries.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Installation

### Rust

Add DBX to your `Cargo.toml`:

```toml
[dependencies]
dbx-core = "0.0.1-beta"
```

### .NET (C#, VB.NET, F#)

Install via NuGet:

```bash
dotnet add package DBX.Client
```

---

## Basic Usage

### Opening a Database

```rust
use dbx_core::Database;

// In-memory database
let db = Database::open_in_memory()?;

// Persistent database
let db = Database::open("./mydata")?;
```

### CRUD Operations

#### Insert

```rust
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;
```

#### Get

```rust
let value = db.get("users", b"user:1")?;
assert_eq!(value, Some(b"Alice".to_vec()));
```

#### Delete

```rust
db.delete("users", b"user:1")?;
```

#### Count

```rust
let count = db.count("users")?;
println!("Total users: {}", count);
```

---

## MVCC Transactions

DBX supports ACID transactions with Snapshot Isolation:

```rust
use dbx_core::Database;

let db = Database::open("./data")?;

// Begin transaction
let tx = db.begin_transaction()?;

// Consistent reads with Snapshot Isolation
tx.insert("users", b"user:3", b"Charlie")?;
tx.insert("users", b"user:4", b"David")?;

// Commit (or rollback)
tx.commit()?;
```

---

## SQL Queries

DBX supports standard SQL queries:

```rust
use dbx_core::Database;
use arrow::array::{Int32Array, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

let db = Database::open_in_memory()?;

// Create table schema
let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("age", DataType::Int32, false),
]));

// Create data
let batch = RecordBatch::try_new(
    schema.clone(),
    vec![
        Arc::new(Int32Array::from(vec![1, 2, 3])),
        Arc::new(Int32Array::from(vec![25, 30, 35])),
    ],
).unwrap();

// Register table
db.register_table("users", vec![batch]);

// Execute SQL
let results = db.execute_sql("SELECT id, age FROM users WHERE age > 28")?;
```

---

## Encryption

DBX supports AES-256-GCM-SIV and ChaCha20-Poly1305 encryption:

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

// Create encrypted database
let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted("./secure-data", enc)?;

// Use normally
db.insert("secrets", b"key1", b"sensitive-data")?;
```

### Key Rotation

```rust
// Rotate encryption key
let new_enc = EncryptionConfig::from_password("new-password");
let count = db.rotate_key(new_enc)?;
println!("Rotated {} records", count);
```

---

## GPU Acceleration (Optional)

Enable GPU features in `Cargo.toml`:

```toml
[dependencies]
dbx-core = { version = "0.0.1-beta", features = ["gpu"] }
```

Use GPU acceleration:

```rust
let db = Database::open_in_memory()?;

// ... register data ...

// Sync GPU cache
db.sync_gpu_cache("users")?;

// GPU-accelerated operations
if let Some(gpu) = db.gpu_manager() {
    let sum = gpu.sum("users", "age")?;
    let filtered = gpu.filter_gt("users", "age", 30)?;
}
```

---

## Next Steps

- [Architecture Guide](architecture) — Learn about the 5-Tier Hybrid Storage
- [Benchmarks](benchmarks) — See performance comparisons
- [Examples](examples/basic-crud) — Explore more code examples
- [API Documentation](https://docs.rs/dbx-core) — Full Rust API reference
