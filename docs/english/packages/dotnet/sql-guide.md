---
layout: default
title: SQL Guide
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 3
---

# SQL Guide

Complete SQL guide for .NET developers.

## CREATE TABLE

```csharp
using DBX.Dotnet;

using var db = Database.Open("mydb.db");

// Basic table
db.ExecuteSql(@"
    CREATE TABLE users (
        id INTEGER,
        name TEXT,
        email TEXT,
        age INTEGER
    )
");

// With PRIMARY KEY
db.ExecuteSql(@"
    CREATE TABLE products (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        price REAL
    )
");
```

## INSERT

```csharp
// Basic INSERT
db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// Specify columns
db.ExecuteSql(@"
    INSERT INTO users (id, name, email) 
    VALUES (2, 'Bob', 'bob@example.com')
");

// Multiple rows
var users = new[]
{
    (1, "Alice", "alice@example.com", 25),
    (2, "Bob", "bob@example.com", 30),
    (3, "Carol", "carol@example.com", 28)
};

foreach (var (id, name, email, age) in users)
{
    db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{name}', '{email}', {age})");
}
```

## SELECT

```csharp
// All rows
var result = db.ExecuteSql("SELECT * FROM users");
Console.WriteLine(result);

// WHERE clause
var adults = db.ExecuteSql("SELECT * FROM users WHERE age >= 18");

// ORDER BY
var sorted = db.ExecuteSql("SELECT * FROM users ORDER BY age DESC");

// LIMIT
var top10 = db.ExecuteSql("SELECT * FROM users LIMIT 10");
```

## UPDATE

```csharp
// Update single column
db.ExecuteSql("UPDATE users SET age = 26 WHERE id = 1");

// Update multiple columns
db.ExecuteSql(@"
    UPDATE users 
    SET name = 'Alice Smith', email = 'alice.smith@example.com'
    WHERE id = 1
");
```

## DELETE

```csharp
// Delete specific row
db.ExecuteSql("DELETE FROM users WHERE id = 1");

// Delete with condition
db.ExecuteSql("DELETE FROM users WHERE age < 18");
```

## Transactions

```csharp
var tx = db.BeginTransaction();
try
{
    db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");
    db.ExecuteSql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)");
    tx.Commit();
}
catch (Exception ex)
{
    tx.Rollback();
    Console.WriteLine($"Transaction failed: {ex.Message}");
}
```

## Practical Example

```csharp
public class UserManager : IDisposable
{
    private readonly Database _db;

    public UserManager(string dbPath)
    {
        _db = Database.Open(dbPath);
        InitSchema();
    }

    private void InitSchema()
    {
        _db.ExecuteSql(@"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                created_at INTEGER
            )
        ");
    }

    public int CreateUser(string username, string email)
    {
        var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        _db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{username}', '{email}', {id})");
        return id;
    }

    public string GetUser(int userId)
    {
        return _db.ExecuteSql($"SELECT * FROM users WHERE id = {userId}");
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}

// Usage
using var mgr = new UserManager("users.db");
var userId = mgr.CreateUser("alice", "alice@example.com");
Console.WriteLine($"Created user: {userId}");
```

## Performance Tips

```csharp
// ❌ Slow
for (int i = 0; i < 1000; i++)
{
    db.ExecuteSql($"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)");
}

// ✅ Fast
var tx = db.BeginTransaction();
for (int i = 0; i < 1000; i++)
{
    db.ExecuteSql($"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)");
}
tx.Commit();
```

## Next Steps

- [KV Operations](kv-operations) - Key-Value operations
- [API Reference](api-reference) - Complete API
