---
layout: default
title: .NET (DBX.Dotnet)
parent: Packages
grand_parent: English
nav_order: 2
---

# .NET — DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet)

High-performance .NET bindings for DBX embedded database with idiomatic C# API.

## Installation

```bash
dotnet add package DBX.Dotnet
```

Or via Package Manager:

```powershell
Install-Package DBX.Dotnet
```

## Quick Start

```csharp
using DBX.Dotnet;

// Open in-memory database
using var db = Database.OpenInMemory();

// Insert
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
db.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());

// Get
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
{
    Console.WriteLine(Encoding.UTF8.GetString(value)); // Alice
}

// Delete
db.Delete("users", "user:2"u8.ToArray());

// Count
var count = db.Count("users");
Console.WriteLine($"Total users: {count}");
```

## Advanced Usage

### Using Statement (Recommended)

```csharp
using DBX.Dotnet;
using System.Text;

using (var db = Database.Open("my_database.db"))
{
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    
    var value = db.Get("users", "user:1"u8.ToArray());
    if (value != null)
    {
        Console.WriteLine(Encoding.UTF8.GetString(value));
    }
} // Auto-disposed and flushed
```

### Working with JSON

```csharp
using DBX.Dotnet;
using System.Text.Json;

public record User(int Id, string Name, string Email);

using var db = Database.OpenInMemory();

// Store JSON data
var user = new User(1, "Alice", "alice@example.com");
var json = JsonSerializer.Serialize(user);
db.Insert("users", "user:1"u8.ToArray(), Encoding.UTF8.GetBytes(json));

// Retrieve JSON data
var data = db.Get("users", "user:1"u8.ToArray());
if (data != null)
{
    var retrievedUser = JsonSerializer.Deserialize<User>(Encoding.UTF8.GetString(data));
    Console.WriteLine(retrievedUser?.Name); // Alice
}
```

### Batch Operations

```csharp
using DBX.Dotnet;

using var db = Database.Open("data.db");

// Batch insert
for (int i = 0; i < 1000; i++)
{
    var key = Encoding.UTF8.GetBytes($"item:{i}");
    var value = Encoding.UTF8.GetBytes($"value_{i}");
    db.Insert("items", key, value);
}

// Flush to disk
db.Flush();
```

### Error Handling

```csharp
using DBX.Dotnet;

try
{
    using var db = Database.Open("my.db");
    db.Insert("users", "key1"u8.ToArray(), "value1"u8.ToArray());
    db.Flush();
}
catch (DbxException ex)
{
    Console.WriteLine($"Database error: {ex.Message}");
}
```

### Async Pattern

```csharp
using DBX.Dotnet;
using System.Threading.Tasks;

public class DataService
{
    private readonly Database _db;
    
    public DataService(string path)
    {
        _db = Database.Open(path);
    }
    
    public async Task<byte[]?> GetAsync(string table, byte[] key)
    {
        return await Task.Run(() => _db.Get(table, key));
    }
    
    public async Task InsertAsync(string table, byte[] key, byte[] value)
    {
        await Task.Run(() => _db.Insert(table, key, value));
    }
    
    public void Dispose()
    {
        _db?.Dispose();
    }
}
```

### ASP.NET Core Integration

```csharp
using DBX.Dotnet;
using Microsoft.AspNetCore.Mvc;

// Program.cs
var builder = WebApplication.CreateBuilder(args);
builder.Services.AddSingleton(Database.Open("sessions.db"));

var app = builder.Build();

// SessionController.cs
[ApiController]
[Route("api/[controller]")]
public class SessionController : ControllerBase
{
    private readonly Database _db;
    
    public SessionController(Database db)
    {
        _db = db;
    }
    
    [HttpPost]
    public IActionResult CreateSession([FromBody] SessionData data)
    {
        var sessionId = Guid.NewGuid().ToString();
        var json = JsonSerializer.Serialize(data);
        _db.Insert("sessions", 
            Encoding.UTF8.GetBytes(sessionId), 
            Encoding.UTF8.GetBytes(json));
        
        return Ok(new { SessionId = sessionId });
    }
    
    [HttpGet("{id}")]
    public IActionResult GetSession(string id)
    {
        var data = _db.Get("sessions", Encoding.UTF8.GetBytes(id));
        if (data == null)
            return NotFound();
        
        var session = JsonSerializer.Deserialize<SessionData>(
            Encoding.UTF8.GetString(data));
        return Ok(session);
    }
}

public record SessionData(int UserId, string Role);
```

## API Reference

### Database Class

#### Static Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `Database.Open(path)` | `string path` | `Database` | Opens file-based database |
| `Database.OpenInMemory()` | - | `Database` | Opens in-memory database |

#### Instance Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `Insert` | `string table, byte[] key, byte[] value` | `void` | Inserts key-value pair |
| `Get` | `string table, byte[] key` | `byte[]?` | Gets value by key |
| `Delete` | `string table, byte[] key` | `void` | Deletes key |
| `Count` | `string table` | `int` | Counts rows in table |
| `Flush` | - | `void` | Flushes to disk |
| `Dispose` | - | `void` | Closes database (IDisposable) |

## Type Definitions

```csharp
namespace DBX.Dotnet;

public class Database : IDisposable
{
    public static Database Open(string path);
    public static Database OpenInMemory();
    
    public void Insert(string table, byte[] key, byte[] value);
    public byte[]? Get(string table, byte[] key);
    public void Delete(string table, byte[] key);
    public int Count(string table);
    public void Flush();
    public void Dispose();
}

public class DbxException : Exception
{
    public DbxException(string message);
}
```

## Performance Tips

1. **Use `using` Statement**: Ensures proper disposal
2. **Batch Operations**: Group inserts before calling `Flush()`
3. **UTF-8 Literals**: Use `"text"u8.ToArray()` for efficient encoding
4. **Singleton Pattern**: Reuse database instances in DI container

## Examples

### Simple Key-Value Store

```csharp
using DBX.Dotnet;
using System.Text;

public class KVStore : IDisposable
{
    private readonly Database _db;
    
    public KVStore(string path)
    {
        _db = Database.Open(path);
    }
    
    public void Set(string key, string value)
    {
        _db.Insert("kv", 
            Encoding.UTF8.GetBytes(key), 
            Encoding.UTF8.GetBytes(value));
    }
    
    public string? Get(string key)
    {
        var data = _db.Get("kv", Encoding.UTF8.GetBytes(key));
        return data != null ? Encoding.UTF8.GetString(data) : null;
    }
    
    public void Delete(string key)
    {
        _db.Delete("kv", Encoding.UTF8.GetBytes(key));
    }
    
    public void Dispose()
    {
        _db?.Dispose();
    }
}

// Usage
using var store = new KVStore("store.db");
store.Set("name", "Alice");
Console.WriteLine(store.Get("name")); // Alice
```

### Session Manager

```csharp
using DBX.Dotnet;
using System.Text.Json;

public class SessionManager : IDisposable
{
    private readonly Database _db;
    
    public SessionManager()
    {
        _db = Database.OpenInMemory();
    }
    
    public void CreateSession(string sessionId, object data, int ttl = 3600)
    {
        var payload = new
        {
            Data = data,
            Expires = DateTimeOffset.UtcNow.AddSeconds(ttl).ToUnixTimeSeconds()
        };
        
        var json = JsonSerializer.Serialize(payload);
        _db.Insert("sessions", 
            Encoding.UTF8.GetBytes(sessionId), 
            Encoding.UTF8.GetBytes(json));
    }
    
    public T? GetSession<T>(string sessionId)
    {
        var raw = _db.Get("sessions", Encoding.UTF8.GetBytes(sessionId));
        if (raw == null) return default;
        
        using var doc = JsonDocument.Parse(Encoding.UTF8.GetString(raw));
        var expires = doc.RootElement.GetProperty("Expires").GetInt64();
        
        if (DateTimeOffset.UtcNow.ToUnixTimeSeconds() > expires)
        {
            _db.Delete("sessions", Encoding.UTF8.GetBytes(sessionId));
            return default;
        }
        
        return JsonSerializer.Deserialize<T>(
            doc.RootElement.GetProperty("Data").GetRawText());
    }
    
    public void Dispose()
    {
        _db?.Dispose();
    }
}

// Usage
using var sessions = new SessionManager();
sessions.CreateSession("sess_123", new { UserId = 42, Role = "admin" });
var data = sessions.GetSession<dynamic>("sess_123");
Console.WriteLine(data?.UserId);
```

### Cache Wrapper

```csharp
using DBX.Dotnet;
using System.Text.Json;

public class Cache : IDisposable
{
    private readonly Database _db;
    
    public Cache()
    {
        _db = Database.OpenInMemory();
    }
    
    public async Task<T> WrapAsync<T>(string key, Func<Task<T>> fn, int ttl = 300)
    {
        // Check cache
        var cached = _db.Get("cache", Encoding.UTF8.GetBytes(key));
        if (cached != null)
        {
            using var doc = JsonDocument.Parse(Encoding.UTF8.GetString(cached));
            var expires = doc.RootElement.GetProperty("Expires").GetInt64();
            
            if (DateTimeOffset.UtcNow.ToUnixTimeSeconds() < expires)
            {
                return JsonSerializer.Deserialize<T>(
                    doc.RootElement.GetProperty("Data").GetRawText())!;
            }
        }
        
        // Execute function
        var result = await fn();
        
        // Store in cache
        var payload = new
        {
            Data = result,
            Expires = DateTimeOffset.UtcNow.AddSeconds(ttl).ToUnixTimeSeconds()
        };
        
        var json = JsonSerializer.Serialize(payload);
        _db.Insert("cache", 
            Encoding.UTF8.GetBytes(key), 
            Encoding.UTF8.GetBytes(json));
        
        return result;
    }
    
    public void Invalidate(string key)
    {
        _db.Delete("cache", Encoding.UTF8.GetBytes(key));
    }
    
    public void Dispose()
    {
        _db?.Dispose();
    }
}

// Usage
using var cache = new Cache();

async Task<string> ExpensiveOperation()
{
    await Task.Delay(1000);
    return "expensive result";
}

var result = await cache.WrapAsync("my-key", ExpensiveOperation);
Console.WriteLine(result); // First call: takes 1s

var cached = await cache.WrapAsync("my-key", ExpensiveOperation);
Console.WriteLine(cached); // Second call: instant
```

## Requirements

- .NET Standard 2.0+ (.NET Framework 4.6.1+, .NET Core 2.0+, .NET 5+)
- **Windows x64 only** (Linux/macOS support planned)

## Troubleshooting

### DllNotFoundException

```bash
# Ensure native library is in output directory
dotnet build
# Check bin/Debug/net8.0/ for dbx_native.dll
```

### Performance Issues

```csharp
// Use batch operations
using var db = Database.Open("data.db");

// Bad: Flush after every insert
for (int i = 0; i < 10000; i++)
{
    db.Insert("items", $"k{i}"u8.ToArray(), $"v{i}"u8.ToArray());
    db.Flush(); // ❌ Slow
}

// Good: Flush once at the end
for (int i = 0; i < 10000; i++)
{
    db.Insert("items", $"k{i}"u8.ToArray(), $"v{i}"u8.ToArray());
}
db.Flush(); // ✅ Fast
```

## Platform Support

| Platform | Architecture | Status |
|----------|--------------|--------|
| Windows | x64 | ✅ Tested |
| Linux | x64 | ⚠️ Planned |
| macOS | x64 (Intel) | ⚠️ Planned |
| macOS | ARM64 (Apple Silicon) | ⚠️ Planned |

## License

Dual-licensed under MIT or Commercial license.
