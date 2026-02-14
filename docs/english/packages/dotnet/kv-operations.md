---
layout: default
title: KV Operations
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 4
---

# Key-Value Operations

High-performance KV operations for .NET.

## Basic CRUD

```csharp
using DBX.Dotnet;
using System.Text;

using var db = Database.OpenInMemory();

// Insert
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());

// Get
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
{
    Console.WriteLine(Encoding.UTF8.GetString(value));  // Alice
}

// Delete
db.Delete("users", "user:1"u8.ToArray());

// Count
var count = db.Count("users");
Console.WriteLine($"Total users: {count}");
```

## JSON Storage

```csharp
using System.Text.Json;

var user = new { Id = 1, Name = "Alice", Email = "alice@example.com" };
var json = JsonSerializer.Serialize(user);
db.Insert("users", "user:1"u8.ToArray(), Encoding.UTF8.GetBytes(json));

var data = db.Get("users", "user:1"u8.ToArray());
if (data != null)
{
    var retrieved = JsonSerializer.Deserialize<dynamic>(Encoding.UTF8.GetString(data));
    Console.WriteLine(retrieved.Name);
}
```

## Batch Operations

```csharp
for (int i = 0; i < 10000; i++)
{
    var key = Encoding.UTF8.GetBytes($"user:{i}");
    var value = Encoding.UTF8.GetBytes($"User {i}");
    db.Insert("users", key, value);
}
db.Flush();
```

## Practical Examples

### Session Store

```csharp
public class SessionStore : IDisposable
{
    private readonly Database _db;

    public SessionStore(string dbPath)
    {
        _db = Database.Open(dbPath);
    }

    public void CreateSession(string sessionId, object data, int ttlSeconds = 3600)
    {
        var session = new
        {
            Data = data,
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(ttlSeconds).ToUnixTimeMilliseconds()
        };
        
        var json = JsonSerializer.Serialize(session);
        _db.Insert("sessions", Encoding.UTF8.GetBytes(sessionId), Encoding.UTF8.GetBytes(json));
    }

    public T? GetSession<T>(string sessionId)
    {
        var data = _db.Get("sessions", Encoding.UTF8.GetBytes(sessionId));
        if (data == null) return default;

        var session = JsonSerializer.Deserialize<SessionData<T>>(Encoding.UTF8.GetString(data));
        
        if (DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() > session.ExpiresAt)
        {
            _db.Delete("sessions", Encoding.UTF8.GetBytes(sessionId));
            return default;
        }

        return session.Data;
    }

    public void Dispose() => _db?.Dispose();
}

record SessionData<T>(T Data, long CreatedAt, long ExpiresAt);
```

## Performance Optimization

```csharp
// ✅ Use transactions
var tx = db.BeginTransaction();
for (int i = 0; i < 10000; i++)
{
    db.Insert("data", Encoding.UTF8.GetBytes($"key:{i}"), Encoding.UTF8.GetBytes($"value:{i}"));
}
tx.Commit();
db.Flush();
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [Advanced](advanced) - Transactions, async patterns
- [API Reference](api-reference) - Complete API
