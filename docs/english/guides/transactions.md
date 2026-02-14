---
layout: default
title: Transactions
parent: English
nav_order: 22
description: "MVCC transactions and concurrency control in DBX"
---

# Transactions
{: .no_toc }

Complete guide to MVCC transactions and concurrency control in DBX.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX implements **Multi-Version Concurrency Control (MVCC)** with **Snapshot Isolation** to provide ACID guarantees while allowing high concurrency.

### Key Features

- **Snapshot Isolation**: Each transaction sees a consistent snapshot of the database
- **ACID Guarantees**: Atomicity, Consistency, Isolation, Durability
- **No Read Locks**: Readers never block writers, writers never block readers
- **Write Conflict Detection**: Automatic detection and handling of write conflicts
- **Garbage Collection**: Automatic cleanup of old versions

---

## Transaction Basics

### Begin Transaction

Start a new transaction:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Begin a new transaction
    let tx = db.begin_transaction()?;
    
    // Transaction operations...
    
    Ok(())
}
```

### Commit Transaction

Commit all changes:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    let tx = db.begin_transaction()?;
    
    // Perform operations
    tx.insert("users", b"user:1", b"Alice")?;
    tx.insert("users", b"user:2", b"Bob")?;
    
    // Commit changes
    tx.commit()?;
    
    Ok(())
}
```

### Rollback Transaction

Abort and discard all changes:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    let tx = db.begin_transaction()?;
    
    tx.insert("users", b"user:1", b"Alice")?;
    
    // Something went wrong, rollback
    tx.rollback()?;
    
    // Changes are discarded
    Ok(())
}
```

---

## MVCC and Snapshot Isolation

### How It Works

When a transaction begins, it receives a **read timestamp** (`read_ts`). All reads see data as it existed at that timestamp.

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Insert initial data
    db.insert("users", b"user:1", b"Alice")?;
    
    // Transaction 1: Read snapshot
    let tx1 = db.begin_transaction()?;
    let value1 = tx1.get("users", b"user:1")?; // Sees "Alice"
    
    // Transaction 2: Update data
    let tx2 = db.begin_transaction()?;
    tx2.insert("users", b"user:1", b"Bob")?;
    tx2.commit()?;
    
    // Transaction 1 still sees "Alice" (snapshot isolation)
    let value2 = tx1.get("users", b"user:1")?; // Still "Alice"
    
    tx1.commit()?;
    
    Ok(())
}
```

### Versioning

Each record is versioned with timestamps:

```rust
// Internal representation (conceptual)
struct VersionedValue {
    value: Vec<u8>,
    version: u64,      // Transaction timestamp
    deleted: bool,     // Tombstone marker
}
```

**Read Rule**: A transaction with `read_ts = T` sees the latest version where `version <= T`.

---

## Concurrency Patterns

### Read-Write Concurrency

Readers and writers don't block each other:

```rust
use dbx_core::Database;
use std::thread;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Writer thread
    let db_clone = db.clone();
    let writer = thread::spawn(move || {
        let tx = db_clone.begin_transaction().unwrap();
        tx.insert("users", b"user:1", b"Alice").unwrap();
        tx.commit().unwrap();
    });
    
    // Reader thread (concurrent)
    let reader = thread::spawn(move || {
        let tx = db.begin_transaction().unwrap();
        let _ = tx.get("users", b"user:1").unwrap();
        tx.commit().unwrap();
    });
    
    writer.join().unwrap();
    reader.join().unwrap();
    
    Ok(())
}
```

### Write-Write Conflicts

DBX detects write conflicts automatically:

```rust
use dbx_core::{Database, DbxError};

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    db.insert("users", b"user:1", b"Initial")?;
    
    // Transaction 1
    let tx1 = db.begin_transaction()?;
    tx1.insert("users", b"user:1", b"Alice")?;
    
    // Transaction 2 (concurrent)
    let tx2 = db.begin_transaction()?;
    tx2.insert("users", b"user:1", b"Bob")?;
    
    // First commit succeeds
    tx1.commit()?;
    
    // Second commit fails with write conflict
    match tx2.commit() {
        Err(DbxError::WriteConflict) => {
            println!("Write conflict detected!");
        }
        _ => {}
    }
    
    Ok(())
}
```

---

## Transaction Isolation Levels

DBX provides **Snapshot Isolation**, which prevents:

- ✅ **Dirty Reads**: Reading uncommitted data
- ✅ **Non-Repeatable Reads**: Same query returns different results
- ✅ **Phantom Reads**: New rows appearing in range queries

### Anomalies Prevented

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // No dirty reads
    let tx1 = db.begin_transaction()?;
    tx1.insert("users", b"user:1", b"Alice")?;
    // Not yet committed
    
    let tx2 = db.begin_transaction()?;
    let value = tx2.get("users", b"user:1")?;
    // value is None (doesn't see uncommitted data)
    
    tx1.commit()?;
    tx2.commit()?;
    
    Ok(())
}
```

---

## Advanced Patterns

### Optimistic Locking

Implement optimistic locking with version numbers:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Document {
    content: String,
    version: u64,
}

fn update_with_optimistic_lock(
    db: &Database,
    key: &[u8],
    new_content: String,
) -> dbx_core::DbxResult<()> {
    let tx = db.begin_transaction()?;
    
    // Read current version
    let current = match tx.get("documents", key)? {
        Some(data) => serde_json::from_slice::<Document>(&data).unwrap(),
        None => return Err(dbx_core::DbxError::NotFound),
    };
    
    // Update with incremented version
    let updated = Document {
        content: new_content,
        version: current.version + 1,
    };
    
    let value = serde_json::to_vec(&updated).unwrap();
    tx.insert("documents", key, &value)?;
    
    tx.commit()?;
    
    Ok(())
}
```

### Retry on Conflict

Automatically retry on write conflicts:

```rust
use dbx_core::{Database, DbxError};

fn retry_on_conflict<F>(db: &Database, mut f: F) -> dbx_core::DbxResult<()>
where
    F: FnMut(&dbx_core::Transaction) -> dbx_core::DbxResult<()>,
{
    const MAX_RETRIES: usize = 3;
    
    for attempt in 0..MAX_RETRIES {
        let tx = db.begin_transaction()?;
        
        match f(&tx) {
            Ok(_) => {
                match tx.commit() {
                    Ok(_) => return Ok(()),
                    Err(DbxError::WriteConflict) if attempt < MAX_RETRIES - 1 => {
                        // Retry
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            Err(e) => {
                tx.rollback()?;
                return Err(e);
            }
        }
    }
    
    Err(DbxError::WriteConflict)
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    retry_on_conflict(&db, |tx| {
        tx.insert("users", b"user:1", b"Alice")?;
        Ok(())
    })?;
    
    Ok(())
}
```

### Read-Modify-Write

Safe read-modify-write pattern:

```rust
use dbx_core::Database;

fn increment_counter(db: &Database, key: &[u8]) -> dbx_core::DbxResult<u64> {
    let tx = db.begin_transaction()?;
    
    // Read
    let current = match tx.get("counters", key)? {
        Some(data) => {
            let bytes: [u8; 8] = data.try_into().unwrap();
            u64::from_le_bytes(bytes)
        }
        None => 0,
    };
    
    // Modify
    let new_value = current + 1;
    
    // Write
    tx.insert("counters", key, &new_value.to_le_bytes())?;
    
    tx.commit()?;
    
    Ok(new_value)
}
```

---

## Garbage Collection

DBX automatically removes old versions that are no longer visible to any transaction.

### How It Works

1. **Version Tracking**: Each version has a timestamp
2. **Active Transactions**: Track the oldest active transaction
3. **Cleanup**: Remove versions older than the oldest active transaction

### Configuration

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Garbage collection runs automatically in the background
    // No manual configuration needed
    
    Ok(())
}
```

---

## Performance Considerations

### Transaction Overhead

- **Begin**: O(1) - Allocate timestamp
- **Read**: O(log n) - Version lookup
- **Write**: O(log n) - Version insertion
- **Commit**: O(m) - m = number of writes

### Best Practices

#### 1. Keep Transactions Short

```rust
// Good: Short transaction
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.commit()?;

// Avoid: Long-running transaction
let tx = db.begin_transaction()?;
// ... lots of work ...
std::thread::sleep(std::time::Duration::from_secs(60));
tx.commit()?; // Blocks garbage collection
```

#### 2. Batch Related Operations

```rust
// Good: Single transaction for related operations
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.insert("profiles", b"profile:1", b"...")?;
tx.insert("settings", b"settings:1", b"...")?;
tx.commit()?;
```

#### 3. Handle Conflicts Gracefully

```rust
match tx.commit() {
    Ok(_) => println!("Success"),
    Err(DbxError::WriteConflict) => {
        // Retry or handle conflict
        println!("Conflict, retrying...");
    }
    Err(e) => return Err(e),
}
```

---

## Error Handling

### Common Errors

```rust
use dbx_core::{Database, DbxError};

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    let tx = db.begin_transaction()?;
    
    match tx.commit() {
        Ok(_) => println!("Committed"),
        Err(DbxError::WriteConflict) => {
            println!("Write conflict - retry needed");
        }
        Err(DbxError::TransactionAborted) => {
            println!("Transaction was aborted");
        }
        Err(e) => {
            eprintln!("Unexpected error: {}", e);
        }
    }
    
    Ok(())
}
```

### Automatic Rollback

Transactions automatically rollback on drop if not committed:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    {
        let tx = db.begin_transaction()?;
        tx.insert("users", b"user:1", b"Alice")?;
        // tx dropped here - automatic rollback
    }
    
    // Changes are not persisted
    let value = db.get("users", b"user:1")?;
    assert!(value.is_none());
    
    Ok(())
}
```

---

## Comparison with Other Systems

### DBX vs Traditional Locking

| Feature | DBX (MVCC) | Traditional Locks |
|---------|------------|-------------------|
| Read-Write Blocking | No | Yes |
| Write-Write Blocking | Conflict Detection | Lock Waiting |
| Deadlocks | No | Possible |
| Read Performance | High | Medium |
| Write Performance | High | Medium |

### Isolation Level Comparison

| Isolation Level | Dirty Read | Non-Repeatable Read | Phantom Read |
|----------------|------------|---------------------|--------------|
| Read Uncommitted | ❌ | ❌ | ❌ |
| Read Committed | ✅ | ❌ | ❌ |
| Repeatable Read | ✅ | ✅ | ❌ |
| **Snapshot Isolation (DBX)** | ✅ | ✅ | ✅ |
| Serializable | ✅ | ✅ | ✅ |

---

## Next Steps

- [CRUD Operations](crud-operations) — Basic database operations
- [SQL Reference](sql-reference) — Use SQL with transactions
- [Performance Tuning](../operations/performance-tuning) — Optimize transaction performance
- [API Reference](../api/transaction) — Complete transaction API
