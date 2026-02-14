---
layout: default
title: Indexing
parent: English
nav_order: 15
---

# Indexing Quick Start

The fastest way to use custom indexing in DBX.

## 1. Creating a Bloom Filter Index

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// Create a Bloom Filter index
db.create_index("users", "email")?;
```

## 2. Fast Lookup with Indices

```rust
// Lookup without index (Slower)
let value = db.get("users", b"user:1")?;

// Lookup with index (Fast)
db.create_index("users", "id")?;
let value = db.get("users", b"user:1")?;  // Utilizes Bloom Filter
```

## 3. Rebuilding Indices

```rust
// Rebuild index (after major data changes)
db.rebuild_index("users")?;
```

## 4. Checking Index Statistics

```rust
// Query index info
let stats = db.index_stats("users")?;
println!("Index size: {} bytes", stats.size);
println!("False positive rate: {:.4}%", stats.fpr * 100.0);
```

## 5. Complete Example

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./indexed_db")?;
    
    // Insert data
    for i in 0..10000 {
        let key = format!("user:{}", i).into_bytes();
        let value = format!("User {}", i).into_bytes();
        db.insert("users", &key, &value)?;
    }
    
    // Create Bloom Filter index
    db.create_index("users", "id")?;
    
    println!("✓ Index created for 10,000 users");
    
    // Rapid lookup
    let value = db.get("users", b"user:5000")?;
    println!("✓ Fast lookup: {:?}", value);
    
    Ok(())
}
```

## Next Steps

- [**Indexing Guide**](../guides/indexing.md) — Complete indexing guide
- [**SQL Quick Start**](sql-quick-start.md) — SQL query optimization
- [**Quick Start**](quick-start.md) — Basic CRUD
