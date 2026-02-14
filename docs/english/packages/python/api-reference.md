---
layout: default
title: API Reference
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 6
---

# API Reference

## Database Class

### Constructors

#### `Database(path: str)`

Opens a file-based database.

**Parameters:**
- `path` (str): Database file path

**Returns:** `Database` instance

**Example:**
```python
db = Database("mydb.db")
```

#### `Database.open_in_memory() -> Database`

Opens an in-memory database.

**Returns:** `Database` instance

**Example:**
```python
db = Database.open_in_memory()
```

### Key-Value Methods

#### `insert(table: str, key: bytes, value: bytes) -> None`

Inserts a key-value pair.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key (binary)
- `value` (bytes): Value (binary)

**Example:**
```python
db.insert("users", b"user:1", b"Alice")
```

#### `get(table: str, key: bytes) -> bytes | None`

Gets a value by key.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key (binary)

**Returns:** Value (bytes) or None

**Example:**
```python
value = db.get("users", b"user:1")
if value:
    print(value.decode())
```

#### `delete(table: str, key: bytes) -> None`

Deletes a key.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key (binary)

**Example:**
```python
db.delete("users", b"user:1")
```

#### `count(table: str) -> int`

Returns the number of rows in a table.

**Parameters:**
- `table` (str): Table name

**Returns:** Row count (int)

**Example:**
```python
count = db.count("users")
print(f"Total: {count}")
```

### SQL Methods

#### `execute_sql(sql: str) -> str`

Executes a SQL statement.

**Parameters:**
- `sql` (str): SQL statement

**Returns:** Result (string, JSON format)

**Example:**
```python
# DDL
db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT)")

# DML
db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")

# Query
result = db.execute_sql("SELECT * FROM users")
print(result)
```

### Transaction Methods

#### `begin_transaction() -> Transaction`

Begins a transaction.

**Returns:** `Transaction` object

**Example:**
```python
tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    tx.commit()
except:
    tx.rollback()
```

### Utility Methods

#### `flush() -> None`

Flushes the buffer to disk.

**Example:**
```python
db.flush()
```

#### `close() -> None`

Closes the database.

**Example:**
```python
db.close()
```

### Context Manager

#### `__enter__() -> Database`

Context manager entry.

#### `__exit__(exc_type, exc_val, exc_tb) -> None`

Context manager exit. Automatically calls `flush()` and `close()`.

**Example:**
```python
with Database("mydb.db") as db:
    db.insert("users", b"user:1", b"Alice")
# Automatically flush() and close()
```

## Transaction Class

### Methods

#### `commit() -> None`

Commits the transaction.

**Example:**
```python
tx = db.begin_transaction()
db.insert("users", b"user:1", b"Alice")
tx.commit()
```

#### `rollback() -> None`

Rolls back the transaction.

**Example:**
```python
tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    tx.commit()
except:
    tx.rollback()
```

## Exceptions

### `DbxError`

Base class for all DBX-related exceptions.

**Example:**
```python
from dbx_py import Database, DbxError

try:
    db = Database("mydb.db")
    db.insert("users", b"user:1", b"Alice")
except DbxError as e:
    print(f"Error: {e}")
```

## Type Hints

```python
from typing import Optional
from dbx_py import Database

class Database:
    def __init__(self, path: str) -> None: ...
    
    @staticmethod
    def open_in_memory() -> 'Database': ...
    
    def insert(self, table: str, key: bytes, value: bytes) -> None: ...
    
    def get(self, table: str, key: bytes) -> Optional[bytes]: ...
    
    def delete(self, table: str, key: bytes) -> None: ...
    
    def count(self, table: str) -> int: ...
    
    def execute_sql(self, sql: str) -> str: ...
    
    def begin_transaction(self) -> 'Transaction': ...
    
    def flush(self) -> None: ...
    
    def close(self) -> None: ...
    
    def __enter__(self) -> 'Database': ...
    
    def __exit__(self, exc_type, exc_val, exc_tb) -> None: ...
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [KV Operations](kv-operations) - Key-Value operations
- [Examples](examples) - Real-world examples
