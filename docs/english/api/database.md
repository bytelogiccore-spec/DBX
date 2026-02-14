---
layout: default
title: Database API
nav_order: 40
parent: English
---

# Database API

Core database operations for DBX.

---

## Constructors

### `Database::open(path: &Path) -> DbxResult<Database>`

Opens or creates a database at the specified path.

**Parameters:**
- `path` - Path to the database directory

**Returns:**
- `DbxResult<Database>` - Database instance

**Example:**
```rust
use dbx_core::Database;
use std::path::Path;

let db = Database::open(Path::new("./data"))?;
```

---

### `Database::open_in_memory() -> DbxResult<Database>`

Creates an in-memory database for testing or temporary storage.

**Returns:**
- `DbxResult<Database>` - In-memory database instance

**Example:**
```rust
let db = Database::open_in_memory()?;
```

---

### `Database::open_encrypted(path: &Path, encryption: EncryptionConfig) -> DbxResult<Database>`

Opens an encrypted database with the specified encryption configuration.

**Parameters:**
- `path` - Path to the database directory
- `encryption` - Encryption configuration (AES-GCM-SIV or ChaCha20-Poly1305)

**Returns:**
- `DbxResult<Database>` - Encrypted database instance

**Example:**
```rust
use dbx_core::storage::encryption::EncryptionConfig;

let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted(Path::new("./data"), enc)?;
```

---

### `Database::open_safe(path: impl AsRef<Path>) -> DbxResult<Database>`

Opens a database with **Full durability** (maximum safety).

Every write operation is immediately synced to disk (fsync). Recommended for financial or medical applications where data loss is unacceptable.

**Parameters:**
- `path` - Path to the database directory

**Returns:**
- `DbxResult<Database>` - Database instance with Full durability

**Example:**
```rust
let db = Database::open_safe("./financial.db")?;
```

---

### `Database::open_fast(path: impl AsRef<Path>) -> DbxResult<Database>`

Opens a database with **No durability** (maximum performance).

WAL is disabled for maximum speed. Suitable for caches, temporary data, or benchmarks.

**Parameters:**
- `path` - Path to the database directory

**Returns:**
- `DbxResult<Database>` - Database instance with No durability

**Example:**
```rust
let db = Database::open_fast("./cache.db")?;
```

---

## CRUD Operations

### `insert(table: &str, key: &[u8], value: &[u8]) -> DbxResult<()>`

Inserts a key-value pair into a table.

**Parameters:**
- `table` - Table name
- `key` - Key bytes
- `value` - Value bytes

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
db.insert("users", b"user:1", b"Alice")?;
```

---

### `insert_batch(table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()>`

Inserts multiple key-value pairs in a single operation (high performance).

**Parameters:**
- `table` - Table name
- `rows` - Vector of (key, value) pairs

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
let rows = vec![
    (b"user:1".to_vec(), b"Alice".to_vec()),
    (b"user:2".to_vec(), b"Bob".to_vec()),
];
db.insert_batch("users", rows)?;
```

---

### `get(table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>>`

Retrieves a value by key from a table.

**Parameters:**
- `table` - Table name
- `key` - Key bytes

**Returns:**
- `DbxResult<Option<Vec<u8>>>` - Value if found, None otherwise

**Example:**
```rust
if let Some(value) = db.get("users", b"user:1")? {
    println!("Found: {:?}", value);
}
```

---

### `delete(table: &str, key: &[u8]) -> DbxResult<bool>`

Deletes a key from a table.

**Parameters:**
- `table` - Table name
- `key` - Key bytes

**Returns:**
- `DbxResult<bool>` - true if deleted, false if not found

**Example:**
```rust
let deleted = db.delete("users", b"user:1")?;
```

---

### `count(table: &str) -> DbxResult<usize>`

Counts the number of rows in a table.

**Parameters:**
- `table` - Table name

**Returns:**
- `DbxResult<usize>` - Number of rows

**Example:**
```rust
let count = db.count("users")?;
println!("Total users: {}", count);
```

---

## SQL Operations

### `execute_sql(sql: &str) -> DbxResult<RecordBatch>`

Executes a SQL query and returns the result as an Arrow RecordBatch.

**Supported SQL:**
- `SELECT` - Column selection and projection
- `WHERE` - Filtering with predicates
- `JOIN` - Inner joins
- `GROUP BY` - Aggregation
- `ORDER BY` - Sorting

**Parameters:**
- `sql` - SQL query string

**Returns:**
- `DbxResult<RecordBatch>` - Query results

**Example:**
```rust
let result = db.execute_sql("SELECT name, age FROM users WHERE age > 18")?;
```

---

## Transaction Operations

### `begin() -> DbxResult<Transaction<'_, Active>>`

Begins a new MVCC transaction with Snapshot Isolation.

**Returns:**
- `DbxResult<Transaction<'_, Active>>` - Active transaction

**Example:**
```rust
let tx = db.begin()?;
// Perform operations within transaction
tx.commit()?;
```

See [Transaction API](transaction) for detailed transaction operations.

---

## Maintenance Operations

### `flush() -> DbxResult<()>`

Flushes all in-memory data (Delta Store) to persistent storage (WOS).

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
db.flush()?;
```

---

### `set_durability(level: DurabilityLevel)`

Sets the durability level for write operations.

**Parameters:**
- `level` - Durability level:
  - `DurabilityLevel::Full` - Sync every write (safest)
  - `DurabilityLevel::Lazy` - Background sync (balanced)
  - `DurabilityLevel::None` - No WAL (fastest)

**Example:**
```rust
db.set_durability(DurabilityLevel::Lazy);
```

---

## Schema Operations

### `create_table_from_arrow(table: &str, schema: Schema) -> DbxResult<()>`

Creates a table with an Arrow schema.

**Parameters:**
- `table` - Table name
- `schema` - Arrow schema definition

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
use arrow::datatypes::{Schema, Field, DataType};

let schema = Schema::new(vec![
    Field::new("id", DataType::Int64, false),
    Field::new("name", DataType::Utf8, false),
]);

db.create_table_from_arrow("users", schema)?;
```

---

### `drop_table(table: &str) -> DbxResult<()>`

Drops a table and all its data.

**Parameters:**
- `table` - Table name

**Returns:**
- `DbxResult<()>` - Success or error

**Example:**
```rust
db.drop_table("old_table")?;
```

---

## Error Handling

All methods return `DbxResult<T>`, which is an alias for `Result<T, DbxError>`.

**Common Error Types:**
- `DbxError::Io` - I/O errors
- `DbxError::Sled` - Storage backend errors
- `DbxError::Arrow` - Arrow processing errors
- `DbxError::Sql` - SQL parsing or execution errors
- `DbxError::Transaction` - Transaction errors

**Example:**
```rust
match db.insert("users", b"key", b"value") {
    Ok(()) => println!("Success"),
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Performance Tips

1. **Use `insert_batch`** for bulk inserts (10-100x faster)
2. **Set durability to Lazy** for write-heavy workloads
3. **Use transactions** for multiple related operations
4. **Call `flush()` periodically** to prevent Delta Store from growing too large
5. **Use SQL** for complex queries instead of manual iteration

---

## See Also

- [Transaction API](transaction) - MVCC transaction operations
- [SQL API](sql) - SQL query execution
- [CRUD Operations Guide](../guides/crud-operations) - Detailed CRUD guide
- [Transactions Guide](../guides/transactions) - Transaction patterns
