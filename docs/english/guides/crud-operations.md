---
layout: default
title: CRUD Operations
parent: Guides
nav_order: 1
description: "Complete guide to Create, Read, Update, Delete operations in DBX"
---

# CRUD Operations
{: .no_toc }

Complete guide to performing Create, Read, Update, and Delete operations in DBX.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX provides a simple and efficient API for basic database operations. All CRUD operations are performed through the `Database` instance and support both single-record and batch operations.

### Key Features

- **High Performance**: In-memory Delta Store for hot data
- **ACID Guarantees**: All operations are atomic and durable
- **Concurrent Access**: Lock-free reads, safe concurrent writes
- **Automatic Flushing**: Delta Store automatically flushes to persistent storage

---

## Insert Operations

### Single Insert

Insert a single key-value pair into a table:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Insert a record
    db.insert("users", b"user:1", b"Alice")?;
    
    Ok(())
}
```

**Parameters:**
- `table`: Table name (string slice)
- `key`: Unique key (byte slice)
- `value`: Value to store (byte slice)

**Returns:**
- `DbxResult<()>`: Success or error

### Batch Insert

Insert multiple records efficiently:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Prepare batch data
    let records = vec![
        (b"user:1".to_vec(), b"Alice".to_vec()),
        (b"user:2".to_vec(), b"Bob".to_vec()),
        (b"user:3".to_vec(), b"Charlie".to_vec()),
    ];
    
    // Batch insert
    for (key, value) in records {
        db.insert("users", &key, &value)?;
    }
    
    Ok(())
}
```

### Insert with Serialization

Store structured data using serialization:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    // Serialize to JSON
    let value = serde_json::to_vec(&user).unwrap();
    let key = format!("user:{}", user.id);
    
    db.insert("users", key.as_bytes(), &value)?;
    
    Ok(())
}
```

---

## Read Operations

### Get Single Record

Retrieve a value by key:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Get a record
    let value = db.get("users", b"user:1")?;
    
    match value {
        Some(data) => {
            let name = String::from_utf8(data).unwrap();
            println!("Found: {}", name);
        }
        None => println!("Not found"),
    }
    
    Ok(())
}
```

**Returns:**
- `DbxResult<Option<Vec<u8>>>`: Value if found, None if not found

### Get with Deserialization

Retrieve and deserialize structured data:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u32,
    name: String,
    email: String,
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let key = b"user:1";
    if let Some(data) = db.get("users", key)? {
        let user: User = serde_json::from_slice(&data).unwrap();
        println!("User: {:?}", user);
    }
    
    Ok(())
}
```

### Count Records

Get the total number of records in a table:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let count = db.count("users")?;
    println!("Total users: {}", count);
    
    Ok(())
}
```

### Scan All Records

Iterate through all records in a table:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Note: Direct scan API may vary based on implementation
    // This is a conceptual example
    let count = db.count("users")?;
    println!("Found {} records", count);
    
    Ok(())
}
```

---

## Delete Operations

### Single Delete

Delete a record by key:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Delete a record
    db.delete("users", b"user:1")?;
    
    // Verify deletion
    let value = db.get("users", b"user:1")?;
    assert!(value.is_none());
    
    Ok(())
}
```

### Batch Delete

Delete multiple records:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let keys_to_delete = vec![
        b"user:1".to_vec(),
        b"user:2".to_vec(),
        b"user:3".to_vec(),
    ];
    
    for key in keys_to_delete {
        db.delete("users", &key)?;
    }
    
    Ok(())
}
```

### Conditional Delete

Delete records based on conditions:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Example: Delete users with specific criteria
    // This requires reading first, then deleting
    let key = b"user:1";
    if let Some(data) = db.get("users", key)? {
        // Check condition
        if should_delete(&data) {
            db.delete("users", key)?;
        }
    }
    
    Ok(())
}

fn should_delete(data: &[u8]) -> bool {
    // Your deletion logic here
    true
}
```

---

## Performance Optimization

### Delta Store Caching

DBX uses an in-memory Delta Store for hot data:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // First write goes to Delta Store (fast)
    db.insert("users", b"user:1", b"Alice")?;
    
    // Immediate read from Delta Store (very fast)
    let value = db.get("users", b"user:1")?;
    
    Ok(())
}
```

**Performance Characteristics:**
- **Insert**: O(log n) - BTreeMap insertion
- **Get**: O(1) - Delta Store hit, O(log n) - WOS/ROS lookup
- **Delete**: O(log n) - Tombstone insertion

### Batch Operations

For better performance, batch your operations:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Prepare all data first
    let mut records = Vec::new();
    for i in 0..1000 {
        let key = format!("user:{}", i);
        let value = format!("User {}", i);
        records.push((key, value));
    }
    
    // Batch insert
    for (key, value) in records {
        db.insert("users", key.as_bytes(), value.as_bytes())?;
    }
    
    Ok(())
}
```

**Benefits:**
- Reduced overhead per operation
- Better cache utilization
- Automatic flush optimization

---

## Error Handling

### Basic Error Handling

```rust
use dbx_core::{Database, DbxError};

fn main() {
    let db = match Database::open("./data") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to open database: {}", e);
            return;
        }
    };
    
    match db.insert("users", b"user:1", b"Alice") {
        Ok(_) => println!("Insert successful"),
        Err(DbxError::DuplicateKey) => println!("Key already exists"),
        Err(e) => eprintln!("Insert failed: {}", e),
    }
}
```

### Using the ? Operator

```rust
use dbx_core::Database;

fn insert_user(db: &Database, id: u32, name: &str) -> dbx_core::DbxResult<()> {
    let key = format!("user:{}", id);
    db.insert("users", key.as_bytes(), name.as_bytes())?;
    Ok(())
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    insert_user(&db, 1, "Alice")?;
    Ok(())
}
```

---

## Best Practices

### 1. Use Meaningful Keys

```rust
// Good: Structured keys
db.insert("users", b"user:1", b"Alice")?;
db.insert("orders", b"order:2023-001", b"...")?;

// Avoid: Random or unclear keys
db.insert("data", b"abc123", b"...")?;
```

### 2. Serialize Complex Data

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
    created_at: i64,
}

// Store as JSON or bincode
let user = User { /* ... */ };
let value = serde_json::to_vec(&user)?;
db.insert("users", b"user:1", &value)?;
```

### 3. Handle Errors Gracefully

```rust
match db.get("users", b"user:1")? {
    Some(data) => {
        // Process data
    }
    None => {
        // Handle missing data
        println!("User not found");
    }
}
```

### 4. Use Transactions for Related Operations

For operations that must succeed or fail together, use transactions:

```rust
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.insert("profiles", b"profile:1", b"...")?;
tx.commit()?;
```

See [Transactions Guide](transactions) for more details.

---

## Common Patterns

### Upsert (Insert or Update)

```rust
fn upsert(db: &Database, table: &str, key: &[u8], value: &[u8]) 
    -> dbx_core::DbxResult<()> 
{
    // DBX insert overwrites existing values
    db.insert(table, key, value)?;
    Ok(())
}
```

### Get or Insert Default

```rust
fn get_or_insert_default(
    db: &Database, 
    table: &str, 
    key: &[u8], 
    default: &[u8]
) -> dbx_core::DbxResult<Vec<u8>> {
    match db.get(table, key)? {
        Some(value) => Ok(value),
        None => {
            db.insert(table, key, default)?;
            Ok(default.to_vec())
        }
    }
}
```

### Increment Counter

```rust
fn increment_counter(db: &Database, key: &[u8]) -> dbx_core::DbxResult<u64> {
    let current = match db.get("counters", key)? {
        Some(data) => {
            let bytes: [u8; 8] = data.try_into().unwrap();
            u64::from_le_bytes(bytes)
        }
        None => 0,
    };
    
    let new_value = current + 1;
    db.insert("counters", key, &new_value.to_le_bytes())?;
    
    Ok(new_value)
}
```

---

## Next Steps

- [Transactions Guide](transactions) — Learn about MVCC transactions
- [SQL Reference](sql-reference) — Use SQL for complex queries
- [Performance Tuning](../operations/performance-tuning) — Optimize your database
- [API Reference](../api/database) — Complete API documentation
