---
layout: default
title: Advanced
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 5
---

# Advanced Features

## Transactions

### Basic Transaction

```python
from dbx_py import Database

db = Database("mydb.db")

tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    db.insert("users", b"user:2", b"Bob")
    tx.commit()
except Exception as e:
    tx.rollback()
    print(f"Transaction failed: {e}")
```

## Performance Tuning

### Batch Operations

```python
# Use transactions
tx = db.begin_transaction()
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
tx.commit()
db.flush()
```

### Buffer Flush

```python
# Explicit flush
db.flush()
```

## Multithreading

```python
import threading

def worker(db_path, thread_id):
    db = Database(db_path)
    for i in range(1000):
        key = f"thread:{thread_id}:key:{i}".encode()
        value = f"value:{i}".encode()
        db.insert("data", key, value)
    db.close()

threads = []
for i in range(4):
    t = threading.Thread(target=worker, args=("mydb.db", i))
    threads.append(t)
    t.start()

for t in threads:
    t.join()
```

## Feature Flags

```python
# Enable/disable features at runtime
db.enable_feature("parallel_query")
db.enable_feature("query_plan_cache")
db.disable_feature("parallel_query")

if db.is_feature_enabled("parallel_query"):
    print("Parallel query enabled")
```

## Query Plan Cache

```python
db.enable_feature("query_plan_cache")

# Repeated queries skip parsing (7.3x faster)
for _ in range(100):
    results = db.execute_sql("SELECT * FROM users WHERE age > 20")
```

## Schema Versioning

```python
db.execute_sql("CREATE TABLE users (id INT, name TEXT)")      # v1
db.execute_sql("ALTER TABLE users ADD COLUMN email TEXT")      # v2

version = db.schema_version("users")  # → 2
```

## UDF (User-Defined Functions)

```python
def double_value(x):
    return x * 2

db.register_scalar_udf("double", double_value)
results = db.execute_sql("SELECT double(price) FROM products")
```

## Triggers

```python
def on_user_insert(event):
    print(f"New user: {event.new_values}")

db.register_trigger("users", "after_insert", on_user_insert)
```

## Scheduler

```python
db.schedule_job("cleanup", "0 0 * * *", lambda: db.execute_sql(
    "DELETE FROM sessions WHERE expired = 1"
))
```

## Next Steps

- [Examples](examples) - More examples
- [API Reference](api-reference) - Complete API
