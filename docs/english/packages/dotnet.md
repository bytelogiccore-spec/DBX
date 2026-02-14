---
layout: default
title: .NET (DBX.Dotnet)
parent: Packages
grand_parent: English
nav_order: 2
---

# .NET — DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet)

High-performance .NET bindings for DBX via CsBindgen with near-zero overhead.

## Installation

```bash
dotnet add package DBX.Dotnet
```

## Quick Start

```csharp
using DBX.Dotnet;

// Open database
using var db = Database.OpenInMemory();

// Insert
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());

// Get
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
    Console.WriteLine(Encoding.UTF8.GetString(value)); // Alice

// Delete
db.Delete("users", "user:1"u8.ToArray());
```

## File-based Database

```csharp
using var db = Database.Open("my_database.db");

db.Insert("config", "key"u8.ToArray(), "value"u8.ToArray());
```

## Transactions

```csharp
using var db = Database.OpenInMemory();

using var tx = db.BeginTransaction();
tx.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
tx.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());
tx.Commit(); // Atomic batch write
```

## API Reference

### Database

| Method | Returns | Description |
|--------|---------|-------------|
| `Database.OpenInMemory()` | `Database` | Open in-memory database |
| `Database.Open(path)` | `Database` | Open file-based database |
| `Insert(table, key, value)` | `void` | Insert key-value pair |
| `Get(table, key)` | `byte[]?` | Get value (null if not found) |
| `Delete(table, key)` | `void` | Delete key |
| `BeginTransaction()` | `Transaction` | Start transaction |
| `Dispose()` | `void` | Close and release |

### Transaction

| Method | Returns | Description |
|--------|---------|-------------|
| `Insert(table, key, value)` | `void` | Buffered insert |
| `Delete(table, key)` | `void` | Buffered delete |
| `Commit()` | `void` | Apply all operations |

## Requirements

- .NET 9.0+
- Windows x64
