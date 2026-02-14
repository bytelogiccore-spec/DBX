---
layout: default
title: Python (dbx-py)
parent: Packages
grand_parent: English
nav_order: 3
---

# Python — dbx-py

[![PyPI](https://img.shields.io/pypi/v/dbx-py.svg)](https://pypi.org/project/dbx-py/)

High-performance Python bindings for DBX embedded database via FFI with Pythonic API design.

## Installation

```bash
pip install dbx-py
```

## Quick Start

```python
from dbx_py import Database

# Open in-memory database
db = Database.open_in_memory()

# Insert
db.insert("users", b"user:1", b"Alice")
db.insert("users", b"user:2", b"Bob")

# Get
value = db.get("users", b"user:1")
print(value.decode('utf-8'))  # Alice

# Delete
db.delete("users", b"user:2")

# Count
count = db.count("users")
print(f"Total users: {count}")

# Close
db.close()
```

## Context Manager (Recommended)

```python
with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
# Auto-closed and flushed
```

## Advanced Usage

### Working with JSON

```python
import json

with Database.open_in_memory() as db:
    # Store JSON data
    user = {"id": 1, "name": "Alice", "email": "alice@example.com"}
    db.insert("users", b"user:1", json.dumps(user).encode())
    
    # Retrieve JSON data
    data = db.get("users", b"user:1")
    user = json.loads(data.decode('utf-8'))
    print(user["name"])  # Alice
```

### Batch Operations

```python
with Database("data.db") as db:
    # Batch insert
    for i in range(1000):
        key = f"item:{i}".encode()
        value = f"value_{i}".encode()
        db.insert("items", key, value)
    
    # Flush to disk
    db.flush()
```

### Error Handling

```python
from dbx_py import Database, DbxError

try:
    db = Database("my.db")
    db.insert("users", b"key1", b"value1")
    db.flush()
except DbxError as e:
    print(f"Database error: {e}")
finally:
    db.close()
```

### Iteration (if supported)

```python
with Database("data.db") as db:
    # Insert test data
    for i in range(10):
        db.insert("test", f"key{i}".encode(), f"val{i}".encode())
    
    # Iterate (implementation-dependent)
    # Note: Check actual API for iteration support
    count = db.count("test")
    print(f"Total items: {count}")
```

## API Reference

### Database Class

#### Constructor

| Method | Returns | Description |
|--------|---------|-------------|
| `Database(path: str)` | `Database` | Opens file-based database |
| `Database.open_in_memory()` | `Database` | Opens in-memory database |

#### Core Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `insert` | `table: str, key: bytes, value: bytes` | `None` | Inserts key-value pair |
| `get` | `table: str, key: bytes` | `bytes \| None` | Gets value by key |
| `delete` | `table: str, key: bytes` | `None` | Deletes key |
| `count` | `table: str` | `int` | Counts rows in table |
| `flush` | - | `None` | Flushes to disk |
| `close` | - | `None` | Closes database |

#### Context Manager

```python
def __enter__(self) -> Database
def __exit__(self, exc_type, exc_val, exc_tb) -> None
```

## Type Hints

```python
from typing import Optional

class Database:
    def __init__(self, path: str) -> None: ...
    
    @staticmethod
    def open_in_memory() -> 'Database': ...
    
    def insert(self, table: str, key: bytes, value: bytes) -> None: ...
    
    def get(self, table: str, key: bytes) -> Optional[bytes]: ...
    
    def delete(self, table: str, key: bytes) -> None: ...
    
    def count(self, table: str) -> int: ...
    
    def flush(self) -> None: ...
    
    def close(self) -> None: ...
```

## Performance Tips

1. **Use Context Manager**: Ensures proper cleanup
2. **Batch Operations**: Group multiple inserts before flush
3. **Binary Keys**: Use `bytes` for keys to avoid encoding overhead
4. **In-Memory for Testing**: Faster for unit tests

## Requirements

- Python 3.8+
- **Windows x64 only** (Linux/macOS support planned)

## Troubleshooting

### Import Error

```python
# If you see "No module named 'dbx_py'"
pip install --upgrade dbx-py
```

### Performance Issues

```python
# Enable batch mode
with Database("data.db") as db:
    for i in range(10000):
        db.insert("bulk", f"k{i}".encode(), f"v{i}".encode())
    db.flush()  # Flush once at the end
```

## Examples

### Simple Key-Value Store

```python
from dbx_py import Database

class KVStore:
    def __init__(self, path: str):
        self.db = Database(path)
    
    def set(self, key: str, value: str):
        self.db.insert("kv", key.encode(), value.encode())
    
    def get(self, key: str) -> str | None:
        data = self.db.get("kv", key.encode())
        return data.decode() if data else None
    
    def close(self):
        self.db.close()

# Usage
store = KVStore("store.db")
store.set("name", "Alice")
print(store.get("name"))  # Alice
store.close()
```

### Session Cache

```python
import time
from dbx_py import Database

class SessionCache:
    def __init__(self):
        self.db = Database.open_in_memory()
    
    def set(self, session_id: str, data: dict, ttl: int = 3600):
        import json
        payload = {
            "data": data,
            "expires": time.time() + ttl
        }
        self.db.insert("sessions", session_id.encode(), 
                      json.dumps(payload).encode())
    
    def get(self, session_id: str) -> dict | None:
        import json
        raw = self.db.get("sessions", session_id.encode())
        if not raw:
            return None
        
        payload = json.loads(raw.decode())
        if time.time() > payload["expires"]:
            self.db.delete("sessions", session_id.encode())
            return None
        
        return payload["data"]

# Usage
cache = SessionCache()
cache.set("sess_123", {"user_id": 42, "role": "admin"})
print(cache.get("sess_123"))
```
