---
layout: default
title: Python (dbx-py)
parent: Packages
grand_parent: English
nav_order: 3
---

# Python — dbx-py

[![PyPI](https://img.shields.io/pypi/v/dbx-py.svg)](https://pypi.org/project/dbx-py/)

Python bindings for DBX via FFI with Pythonic API design.

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

## Context Manager

```python
with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
# Auto-closed
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `Database(path)` | `Database` | Open file-based database |
| `Database.open_in_memory()` | `Database` | Open in-memory database |
| `insert(table, key, value)` | `None` | Insert key-value pair |
| `get(table, key)` | `bytes \| None` | Get value |
| `delete(table, key)` | `None` | Delete key |
| `count(table)` | `int` | Count rows |
| `flush()` | `None` | Flush to disk |
| `close()` | `None` | Close database |

## Requirements

- Python 3.8+
- Windows x64
