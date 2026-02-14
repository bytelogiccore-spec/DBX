---
layout: default
title: Quick Start
parent: Examples
nav_order: 1
---

# Quick Start

Get started with DBX in 5 minutes — the fastest guide to jump in.

## 1. Opening a Database

```rust
use dbx_core::Database;

// In-memory database (for quick testing)
let db = Database::open_in_memory()?;

// Or a persistent database
let db = Database::open("./mydata")?;
```

## 2. Inserting Data

```rust
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;
db.insert("users", b"user:3", b"Charlie")?;
```

## 3. Querying Data

```rust
let value = db.get("users", b"user:1")?;
match value {
    Some(v) => println!("Found: {}", String::from_utf8_lossy(&v)),
    None => println!("Not found"),
}
```

## 4. Deleting Data

```rust
db.delete("users", b"user:1")?;
```

## 5. Complete Example

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create
    db.insert("users", b"user:1", b"Alice")?;
    
    // Read
    if let Some(value) = db.get("users", b"user:1")? {
        println!("User: {}", String::from_utf8_lossy(&value));
    }
    
    // Update (upsert)
    db.insert("users", b"user:1", b"Alice Smith")?;
    
    // Delete
    db.delete("users", b"user:1")?;
    
    Ok(())
}
```

## Next Steps

- [**CRUD Operations Guide**](../guides/crud-operations.md) — Complete CRUD guide
- [**SQL Quick Start**](sql-quick-start.md) — SQL basics
- [**Transactions**](../guides/transactions.md) — Transaction usage
