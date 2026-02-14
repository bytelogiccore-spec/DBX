---
layout: default
title: WAL Recovery
parent: Examples
nav_order: 5
---

# WAL Recovery Quick Start

The fastest way to use WAL recovery in DBX.

## 1. Enabling WAL

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// WAL is enabled by default
```

## 2. Setting Durability Levels

```rust
use dbx_core::storage::wal::DurabilityLevel;

// Maximum performance (Memory only)
db.set_durability(DurabilityLevel::None)?;

// Balanced (Default)
db.set_durability(DurabilityLevel::Normal)?;

// Maximum safety (Immediate fsync)
db.set_durability(DurabilityLevel::Paranoid)?;
```

## 3. Testing Crash Recovery

```rust
// Insert data
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;

// Simulate crash (force drop)
drop(db);

// Restart (Automatic recovery)
let db = Database::open("./db")?;

// Verify data
assert_eq!(db.get("users", b"user:1")?, Some(b"Alice".to_vec()));
assert_eq!(db.get("users", b"user:2")?, Some(b"Bob".to_vec()));
```

## 4. Manual Checkpoints

```rust
// Force flush WAL to disk
db.checkpoint()?;
```

## 5. Complete Example

```rust
use dbx_core::Database;
use dbx_core::storage::wal::DurabilityLevel;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./wal_test")?;
    
    // Set to maximum safety
    db.set_durability(DurabilityLevel::Paranoid)?;
    
    // Insert critical data
    db.insert("transactions", b"tx:1", b"Payment: $100")?;
    db.insert("transactions", b"tx:2", b"Payment: $200")?;
    
    println!("✓ Data written with WAL");
    
    // Checkpoint (Sync to disk)
    db.checkpoint()?;
    
    println!("✓ WAL checkpointed to disk");
    
    // Data preserved even after "crash"
    drop(db);
    let db = Database::open("./wal_test")?;
    
    assert!(db.get("transactions", b"tx:1")?.is_some());
    println!("✓ Data recovered after restart");
    
    Ok(())
}
```

## Next Steps

- [**WAL Recovery Guide**](../guides/wal-recovery.md) — Complete WAL guide
- [**Transactions**](../guides/transactions.md) — ACID transactions
- [**Quick Start**](quick-start.md) — Basic CRUD
