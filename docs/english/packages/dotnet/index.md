---
layout: default
title: .NET (DBX.Dotnet)
nav_order: 2
parent: Packages
grand_parent: English
has_children: true
---

# .NET — DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet/)

Official .NET bindings for DBX high-performance embedded database.

## Key Features

- 🚀 **Native Performance**: Rust-based P/Invoke
- 💾 **5-Tier Storage**: WOS → L0 → L1 → L2 → Cold Storage
- 🔒 **MVCC Transactions**: Snapshot isolation support
- 📊 **SQL Support**: Full DDL + DML support
- 🔐 **Encryption**: AES-GCM-SIV, ChaCha20-Poly1305
- 🎯 **.NET Standard 2.0**: .NET Framework, .NET Core, .NET 5+ support

## Quick Start

```bash
dotnet add package DBX.Dotnet
```

```csharp
using DBX.Dotnet;

using (var db = Database.OpenInMemory())
{
    // KV operations
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    var value = db.Get("users", "user:1"u8.ToArray());
    Console.WriteLine(Encoding.UTF8.GetString(value));  // Alice
    
    // SQL operations
    db.ExecuteSql("CREATE TABLE users (id INTEGER, name TEXT)");
    db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice')");
    var result = db.ExecuteSql("SELECT * FROM users");
    Console.WriteLine(result);
}
```

## Documentation

- [Installation](installation) - Setup and configuration
- [Quick Start](quickstart) - Get started in 5 minutes
- [KV Operations](kv-operations) - Key-Value operations guide
- [SQL Guide](sql-guide) - SQL usage
- [Advanced](advanced) - Transactions, encryption, performance tuning
- [API Reference](api-reference) - Complete API documentation
- [Examples](examples) - Real-world examples

## Version Info

- **Current Version**: {{ site.dbx_version }}
- **.NET Requirements**: .NET Standard 2.0+ (.NET Framework 4.6.1+, .NET Core 2.0+, .NET 5+)
- **Platform**: Windows x64 (Linux/macOS planned)

## License

MIT License
