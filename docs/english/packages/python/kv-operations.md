---
layout: default
title: KV Operations
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 4
---

# Key-Value Operations

DBX can be used as a high-performance Key-Value store in addition to SQL.

## Basic CRUD

### Insert

```python
from dbx_py import Database

db = Database.open_in_memory()

# Basic insert
db.insert("users", b"user:1", b"Alice")

# JSON data
import json
user = {"id": 1, "name": "Alice", "email": "alice@example.com"}
db.insert("users", b"user:1", json.dumps(user).encode())
```

### Get

```python
# Single get
value = db.get("users", b"user:1")
if value:
    print(value.decode())  # Alice

# JSON parsing
user_bytes = db.get("users", b"user:1")
if user_bytes:
    user = json.loads(user_bytes.decode())
    print(user["name"])
```

### Delete

```python
db.delete("users", b"user:1")

# Check before delete
if db.get("users", b"user:1"):
    db.delete("users", b"user:1")
    print("Deleted")
```

### Count

```python
count = db.count("users")
print(f"Total users: {count}")
```

## Batch Operations

```python
# Bulk insert
for i in range(10000):
    key = f"user:{i}".encode()
    value = f"User {i}".encode()
    db.insert("users", key, value)

# Flush
db.flush()
```

## Practical Examples

### Session Store

```python
import time
import json

class SessionStore:
    def __init__(self, db_path):
        self.db = Database(db_path)
    
    def create_session(self, session_id, data, ttl_seconds=3600):
        session = {
            "data": data,
            "created_at": time.time(),
            "expires_at": time.time() + ttl_seconds
        }
        self.db.insert("sessions", session_id.encode(), json.dumps(session).encode())
    
    def get_session(self, session_id):
        value = self.db.get("sessions", session_id.encode())
        if not value:
            return None
        
        session = json.loads(value.decode())
        
        # Check expiration
        if time.time() > session["expires_at"]:
            self.db.delete("sessions", session_id.encode())
            return None
        
        return session["data"]
    
    def delete_session(self, session_id):
        self.db.delete("sessions", session_id.encode())
    
    def close(self):
        self.db.close()

# Usage
store = SessionStore("sessions.db")
store.create_session("sess_abc123", {"user_id": 42, "username": "alice"}, 3600)
session = store.get_session("sess_abc123")
if session:
    print(f"User: {session['username']}")
store.close()
```

### Cache System

```python
class Cache:
    def __init__(self, db_path, default_ttl=300):
        self.db = Database(db_path)
        self.default_ttl = default_ttl
    
    def set(self, key, value, ttl=None):
        ttl = ttl or self.default_ttl
        cache_data = {
            "value": value,
            "expires_at": time.time() + ttl
        }
        self.db.insert("cache", key.encode(), json.dumps(cache_data).encode())
    
    def get(self, key):
        data = self.db.get("cache", key.encode())
        if not data:
            return None
        
        cache_data = json.loads(data.decode())
        
        # Check expiration
        if time.time() > cache_data["expires_at"]:
            self.db.delete("cache", key.encode())
            return None
        
        return cache_data["value"]
    
    def delete(self, key):
        self.db.delete("cache", key.encode())
    
    def close(self):
        self.db.close()

# Usage
cache = Cache("cache.db", 300)
cache.set("user:1", {"name": "Alice", "email": "alice@example.com"})
user = cache.get("user:1")
if user:
    print(f"Cached user: {user['name']}")
cache.close()
```

## Performance Optimization

### 1. Batch Operations + Flush

```python
# ❌ Slow
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
    db.flush()  # Flush every time

# ✅ Fast
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
db.flush()  # Flush once
```

### 2. Use Transactions

```python
tx = db.begin_transaction()
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
tx.commit()
db.flush()
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [Advanced](advanced) - Transactions, encryption
- [API Reference](api-reference) - Complete API
