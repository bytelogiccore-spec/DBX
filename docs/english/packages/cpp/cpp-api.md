---
layout: default
title: C++ API
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 4
---

# C++ API Reference

C++ wrapper provides RAII and exception handling.

## Database Class

### Constructors

#### `Database::open(const std::string& path)`

Opens a file-based database.

**Example:**
```cpp
auto db = dbx::Database::open("mydb.db");
```

#### `Database::openInMemory()`

Opens an in-memory database.

**Example:**
```cpp
auto db = dbx::Database::openInMemory();
```

### Key-Value Methods

#### `insert(const std::string& table, const std::string& key, const std::string& value)`

Inserts a key-value pair.

**Example:**
```cpp
db.insert("users", "user:1", "Alice");
```

#### `get(const std::string& table, const std::string& key) -> std::optional<std::string>`

Gets a value by key.

**Example:**
```cpp
auto value = db.get("users", "user:1");
if (value) {
    std::cout << *value << std::endl;
}
```

#### `remove(const std::string& table, const std::string& key)`

Deletes a key.

**Example:**
```cpp
db.remove("users", "user:1");
```

### SQL Methods

#### `executeSql(const std::string& sql) -> std::string`

Executes SQL.

**Example:**
```cpp
db.executeSql("CREATE TABLE users (id INTEGER, name TEXT)");
auto result = db.executeSql("SELECT * FROM users");
```

### Transaction Methods

#### `beginTransaction() -> Transaction`

Begins a transaction.

**Example:**
```cpp
auto tx = db.beginTransaction();
tx.commit();
```

## RAII Pattern

```cpp
{
    auto db = dbx::Database::open("mydb.db");
    db.insert("users", "user:1", "Alice");
    // Automatically flush() and close()
}
```

## Next Steps

- [C API](c-api) - C function reference
- [Examples](examples) - More examples
