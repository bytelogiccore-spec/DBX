---
layout: default
title: Quick Start
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 2
---

# Quick Start

Get started with DBX in 5 minutes!

## Installation

```bash
pip install dbx-py
```

## First Program

```python
from dbx_py import Database

# Open in-memory database
db = Database.open_in_memory()

# KV operations
db.insert("users", b"user:1", b"Alice")
value = db.get("users", b"user:1")
print(value.decode())  # Alice

# SQL operations
db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT)")
db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")
result = db.execute_sql("SELECT * FROM users")
print(result)

db.close()
```

## Using Context Manager

```python
with Database("mydb.db") as db:
    db.insert("users", b"user:1", b"Alice")
    # Automatically flush() and close()
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [KV Operations](kv-operations) - Key-Value operations
- [API Reference](api-reference) - Complete API
