# DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Guide](https://img.shields.io/badge/guide-GitHub%20Pages-blue)](https://bytelogiccore-spec.github.io/DBX/english/packages/dotnet)

> High-performance .NET bindings for DBX embedded database

**DBX.Dotnet** provides native C# bindings to the DBX database engine via CsBindgen, delivering near-zero overhead access to the high-performance Rust core.

## Installation

```bash
dotnet add package DBX.Dotnet
```

## Quick Start

```csharp
using DBX.Dotnet;

// Open an in-memory database
using var db = Database.OpenInMemory();

// Insert data
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
db.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());

// Get data
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
    Console.WriteLine(Encoding.UTF8.GetString(value)); // Alice

// Delete data
db.Delete("users", "user:2"u8.ToArray());
```

## Transactions

```csharp
using var db = Database.Open("my_database.db");

using var tx = db.BeginTransaction();
tx.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
tx.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());
tx.Commit();
```

## API Reference

### Database

| Method | Description |
|--------|-------------|
| `Database.OpenInMemory()` | Open an in-memory database |
| `Database.Open(path)` | Open a file-based database |
| `Insert(table, key, value)` | Insert a key-value pair |
| `Get(table, key)` | Get value by key (returns `byte[]?`) |
| `Delete(table, key)` | Delete a key |
| `BeginTransaction()` | Start a new transaction |
| `Dispose()` | Close and free resources |

### Transaction

| Method | Description |
|--------|-------------|
| `Insert(table, key, value)` | Buffered insert |
| `Delete(table, key)` | Buffered delete |
| `Commit()` | Commit all buffered operations |

## Requirements

- .NET Standard 2.0+
  - .NET Framework 4.6.1+
  - .NET Core 2.0+
  - .NET 5, 6, 7, 8, 9+
- Windows x64 (native DLL included)

## License

MIT License — see [LICENSE](https://github.com/bytelogiccore-spec/DBX/blob/main/LICENSE) for details.
