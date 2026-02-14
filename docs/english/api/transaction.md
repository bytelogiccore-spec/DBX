---
layout: default
title: Transaction API
nav_order: 42
parent: English
---

# Transaction API

MVCC transaction management for DBX.

---

## Overview

DBX provides **MVCC (Multi-Version Concurrency Control)** transactions with **Snapshot Isolation**. Transactions use the **Typestate pattern** for compile-time safety.

**Transaction States:**
- `Active` - Transaction is active, can perform operations
- `Committed` - Transaction has been committed (terminal state)
- `Aborted` - Transaction has been aborted (terminal state)

---

## Creating Transactions

### `Database::begin() -> DbxResult<Transaction<'_, Active>>`

Begins a new MVCC transaction.

**Returns:**
- `DbxResult<Transaction<'_, Active>>` - Active transaction

**Example:**
```rust
let tx = db.begin()?;
```

---

## Transaction Operations

### `insert(table: &str, key: &[u8], value: &[u8]) -> DbxResult<()>`

Inserts a key-value pair within the transaction.

**Parameters:**
- `table` - Table name
- `key` - Key bytes
- `value` - Value bytes

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
let tx = db.begin()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.commit()?;
```

---

### `get(table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>>`

Retrieves a value by key within the transaction's snapshot.

**Parameters:**
- `table` - Table name
- `key` - Key bytes

**Returns:**
- `DbxResult<Option<Vec<u8>>>` - Value if found, None otherwise

**Example:**
```rust
let tx = db.begin()?;
if let Some(value) = tx.get("users", b"user:1")? {
    println!("Found: {:?}", value);
}
```

---

### `delete(table: &str, key: &[u8]) -> DbxResult<bool>`

Deletes a key within the transaction.

**Parameters:**
- `table` - Table name
- `key` - Key bytes

**Returns:**
- `DbxResult<bool>` - true if deleted, false if not found

**Example:**
```rust
let tx = db.begin()?;
tx.delete("users", b"user:1")?;
tx.commit()?;
```

---

## Finalizing Transactions

### `commit(self) -> DbxResult<Transaction<'_, Committed>>`

Commits the transaction, making all changes permanent.

**Returns:**
- `DbxResult<Transaction<'_, Committed>>` - Committed transaction

**Example:**
```rust
let tx = db.begin()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.commit()?;
```

---

### `abort(self) -> DbxResult<Transaction<'_, Aborted>>`

Aborts the transaction, discarding all changes.

**Returns:**
- `DbxResult<Transaction<'_, Aborted>>` - Aborted transaction

**Example:**
```rust
let tx = db.begin()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.abort()?; // Changes discarded
```

---

## MVCC Snapshot Isolation

### How It Works

1. **Snapshot Creation**: When `begin()` is called, a snapshot of the database is created
2. **Read Consistency**: All reads within the transaction see the same consistent snapshot
3. **Write Isolation**: Writes are isolated until commit
4. **Conflict Detection**: Write-write conflicts are detected at commit time

**Example:**
```rust
// Thread 1
let tx1 = db.begin()?;
tx1.insert("users", b"user:1", b"Alice")?;

// Thread 2 (concurrent)
let tx2 = db.begin()?;
let value = tx2.get("users", b"user:1")?; // None (snapshot isolation)

// Thread 1 commits
tx1.commit()?;

// Thread 2 still sees old snapshot
let value = tx2.get("users", b"user:1")?; // Still None
```

---

## Transaction Patterns

### Pattern 1: Simple Transaction

```rust
let tx = db.begin()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.insert("users", b"user:2", b"Bob")?;
tx.commit()?;
```

### Pattern 2: Conditional Commit

```rust
let tx = db.begin()?;
tx.insert("users", b"user:1", b"Alice")?;

if some_condition {
    tx.commit()?;
} else {
    tx.abort()?;
}
```

### Pattern 3: Error Handling

```rust
let tx = db.begin()?;

match tx.insert("users", b"user:1", b"Alice") {
    Ok(()) => tx.commit()?,
    Err(e) => {
        eprintln!("Error: {}", e);
        tx.abort()?;
    }
}
```

### Pattern 4: Read-Modify-Write

```rust
let tx = db.begin()?;

if let Some(value) = tx.get("counter", b"count")? {
    let count = u64::from_be_bytes(value.try_into().unwrap());
    let new_count = count + 1;
    tx.insert("counter", b"count", &new_count.to_be_bytes())?;
}

tx.commit()?;
```

---

## Typestate Pattern

DBX uses the **Typestate pattern** to enforce transaction state at compile time.

**Compile-Time Safety:**
```rust
let tx = db.begin()?; // Transaction<'_, Active>

// ✅ Can call insert/get/delete on Active transaction
tx.insert("users", b"key", b"value")?;

let tx = tx.commit()?; // Transaction<'_, Committed>

// ❌ Compile error: cannot call insert on Committed transaction
// tx.insert("users", b"key", b"value")?; // ERROR!
```

**State Transitions:**
```
Active
  ├─ commit() → Committed (terminal)
  └─ abort()  → Aborted (terminal)
```

---

## Garbage Collection

DBX automatically performs **garbage collection** of old transaction versions.

**Configuration:**
```rust
// Automatic GC runs every 1000 transactions (default)
// Old versions are cleaned up after no active transactions reference them
```

**Manual GC:**
```rust
// GC is automatic, but you can trigger flush to help:
db.flush()?;
```

---

## Performance Considerations

### Best Practices

1. **Keep transactions short** - Long transactions hold snapshots and prevent GC
2. **Batch operations** - Group related operations in a single transaction
3. **Avoid read-modify-write conflicts** - Use optimistic locking patterns
4. **Commit or abort quickly** - Don't hold transactions open unnecessarily

### Performance Metrics

| Operation | Latency |
|-----------|---------|
| `begin()` | ~1-2 µs |
| `insert()` | ~3-5 µs |
| `get()` | ~2-4 µs |
| `commit()` | ~10-20 µs |

---

## Concurrency

### Thread Safety

- ✅ **Database is thread-safe** - Can be shared across threads
- ✅ **Transactions are NOT thread-safe** - Use one transaction per thread
- ✅ **MVCC allows concurrent reads and writes** - No blocking

**Example:**
```rust
use std::sync::Arc;
use std::thread;

let db = Arc::new(Database::open_in_memory()?);

let handles: Vec<_> = (0..10)
    .map(|i| {
        let db = Arc::clone(&db);
        thread::spawn(move || {
            let tx = db.begin().unwrap();
            tx.insert("users", format!("user:{}", i).as_bytes(), b"value").unwrap();
            tx.commit().unwrap();
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap();
}
```

---

## Error Handling

### Transaction Errors

- `DbxError::TransactionConflict` - Write-write conflict detected
- `DbxError::TransactionAborted` - Transaction was aborted
- `DbxError::InvalidTransactionState` - Invalid state transition

**Example:**
```rust
match tx.commit() {
    Ok(_) => println!("Committed"),
    Err(DbxError::TransactionConflict) => {
        eprintln!("Conflict detected, retry");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## See Also

- [Database API](database) - Core database operations
- [Transactions Guide](../guides/transactions) - Detailed transaction patterns
- [CRUD Operations Guide](../guides/crud-operations) - Basic CRUD operations
