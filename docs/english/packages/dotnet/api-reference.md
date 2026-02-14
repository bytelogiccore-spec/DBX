---
layout: default
title: API Reference
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 6
---

# API Reference

## Database Class

### Static Methods

#### `Database.Open(string path)`

Opens a file-based database.

**Example:**
```csharp
var db = Database.Open("mydb.db");
```

#### `Database.OpenInMemory()`

Opens an in-memory database.

**Example:**
```csharp
var db = Database.OpenInMemory();
```

### Key-Value Methods

#### `void Insert(string table, byte[] key, byte[] value)`

Inserts a key-value pair.

**Example:**
```csharp
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
```

#### `byte[]? Get(string table, byte[] key)`

Gets a value by key.

**Example:**
```csharp
var value = db.Get("users", "user:1"u8.ToArray());
```

#### `void Delete(string table, byte[] key)`

Deletes a key.

**Example:**
```csharp
db.Delete("users", "user:1"u8.ToArray());
```

#### `int Count(string table)`

Returns row count.

**Example:**
```csharp
var count = db.Count("users");
```

### SQL Methods

#### `string ExecuteSql(string sql)`

Executes SQL.

**Example:**
```csharp
db.ExecuteSql("CREATE TABLE users (id INTEGER, name TEXT)");
var result = db.ExecuteSql("SELECT * FROM users");
```

### Transaction Methods

#### `Transaction BeginTransaction()`

Begins a transaction.

**Example:**
```csharp
var tx = db.BeginTransaction();
tx.Commit();
```

### IDisposable

#### `void Dispose()`

Closes database.

**Example:**
```csharp
using var db = Database.Open("mydb.db");
```

## Transaction Class

### Methods

#### `void Commit()`

Commits transaction.

#### `void Rollback()`

Rolls back transaction.

## Next Steps

- [Examples](examples) - Real-world examples
