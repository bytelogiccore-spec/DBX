---
layout: default
title: Quick Start
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 2
---

# Quick Start

Get started with DBX in 5 minutes!

## Installation

```bash
dotnet add package DBX.Dotnet
```

## First Program

```csharp
using DBX.Dotnet;
using System.Text;

// Open in-memory database
using var db = Database.OpenInMemory();

// KV operations
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
var value = db.Get("users", "user:1"u8.ToArray());
Console.WriteLine(Encoding.UTF8.GetString(value));  // Alice

// SQL operations
db.ExecuteSql("CREATE TABLE users (id INTEGER, name TEXT)");
db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice')");
var result = db.ExecuteSql("SELECT * FROM users");
Console.WriteLine(result);
```

## Using Statement

```csharp
using (var db = Database.Open("mydb.db"))
{
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    // Automatically Flush() and Dispose()
}
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [KV Operations](kv-operations) - Key-Value operations
- [API Reference](api-reference) - Complete API
