---
layout: default
title: SQL Guide
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 3
---

# SQL Guide

DBX supports standard SQL. You can use it via the `execute_sql` method in Python.

## CREATE TABLE

```python
from dbx_py import Database

db = Database("mydb.db")

# Basic table
db.execute_sql("""
    CREATE TABLE users (
        id INTEGER,
        name TEXT,
        email TEXT,
        age INTEGER
    )
""")

# With PRIMARY KEY
db.execute_sql("""
    CREATE TABLE products (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        price REAL
    )
""")
```

## INSERT

```python
# Basic INSERT
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)")

# Specify columns
db.execute_sql("""
    INSERT INTO users (id, name, email) 
    VALUES (2, 'Bob', 'bob@example.com')
""")

# Multiple rows
users = [
    (1, 'Alice', 'alice@example.com', 25),
    (2, 'Bob', 'bob@example.com', 30),
    (3, 'Carol', 'carol@example.com', 28)
]

for user in users:
    db.execute_sql(f"INSERT INTO users VALUES {user}")
```

## SELECT

```python
# All rows
result = db.execute_sql("SELECT * FROM users")
print(result)

# WHERE clause
adults = db.execute_sql("SELECT * FROM users WHERE age >= 18")

# ORDER BY
sorted_users = db.execute_sql("SELECT * FROM users ORDER BY age DESC")

# LIMIT
top10 = db.execute_sql("SELECT * FROM users LIMIT 10")

# Aggregation
count = db.execute_sql("SELECT COUNT(*) FROM users")
stats = db.execute_sql("""
    SELECT AVG(age), MIN(age), MAX(age) FROM users
""")
```

## UPDATE

```python
# Update single column
db.execute_sql("UPDATE users SET age = 26 WHERE id = 1")

# Update multiple columns
db.execute_sql("""
    UPDATE users 
    SET name = 'Alice Smith', email = 'alice.smith@example.com'
    WHERE id = 1
""")
```

## DELETE

```python
# Delete specific row
db.execute_sql("DELETE FROM users WHERE id = 1")

# Delete with condition
db.execute_sql("DELETE FROM users WHERE age < 18")
```

## Transactions

```python
tx = db.begin_transaction()
try:
    db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)")
    db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)")
    tx.commit()
except Exception as e:
    tx.rollback()
    print(f"Transaction failed: {e}")
```

## Practical Example

```python
class UserManager:
    def __init__(self, db_path):
        self.db = Database(db_path)
        self._init_schema()
    
    def _init_schema(self):
        self.db.execute_sql("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                created_at INTEGER
            )
        """)
    
    def create_user(self, username, email):
        import time
        user_id = int(time.time())
        self.db.execute_sql(
            f"INSERT INTO users VALUES ({user_id}, '{username}', '{email}', {user_id})"
        )
        return user_id
    
    def get_user(self, user_id):
        return self.db.execute_sql(f"SELECT * FROM users WHERE id = {user_id}")
    
    def list_users(self, limit=100):
        return self.db.execute_sql(f"SELECT * FROM users LIMIT {limit}")
    
    def close(self):
        self.db.close()

# Usage
mgr = UserManager("users.db")
user_id = mgr.create_user("alice", "alice@example.com")
print(f"Created user: {user_id}")
user = mgr.get_user(user_id)
print(f"User: {user}")
mgr.close()
```

## Performance Tips

### 1. Use Transactions for Batch Operations

```python
# ❌ Slow
for i in range(1000):
    db.execute_sql(f"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)")

# ✅ Fast
tx = db.begin_transaction()
for i in range(1000):
    db.execute_sql(f"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)")
tx.commit()
```

## Limitations

- **JOIN**: Not currently supported (planned)
- **Subqueries**: Limited support
- **Foreign Keys**: Not currently supported

## Next Steps

- [KV Operations](kv-operations) - Key-Value operations
- [Advanced](advanced) - Transactions, performance
- [API Reference](api-reference) - Complete API
