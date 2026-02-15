---
layout: default
title: Advanced
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 5
---

# Advanced Features

## Transactions

```csharp
var tx = db.BeginTransaction();
try
{
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    tx.Commit();
}
catch
{
    tx.Rollback();
}
```

## Async Patterns

```csharp
public async Task<int> CreateUserAsync(string name)
{
    return await Task.Run(() =>
    {
        var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        _db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{name}', 'email')");
        return id;
    });
}
```

## Performance Tuning

```csharp
var tx = db.BeginTransaction();
for (int i = 0; i < 10000; i++)
{
    db.Insert("data", $"key:{i}"u8.ToArray(), $"value:{i}"u8.ToArray());
}
tx.Commit();
db.Flush();
```

## Feature Flags

```csharp
db.EnableFeature("parallel_query");
db.EnableFeature("query_plan_cache");
db.DisableFeature("parallel_query");

if (db.IsFeatureEnabled("parallel_query"))
{
    Console.WriteLine("Parallel query enabled");
}
```

## Query Plan Cache

```csharp
db.EnableFeature("query_plan_cache");

// Repeated queries skip parsing (7.3x faster)
for (int i = 0; i < 100; i++)
{
    var results = db.ExecuteSql("SELECT * FROM users WHERE age > 20");
}
```

## Schema Versioning

```csharp
db.ExecuteSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.ExecuteSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

var version = db.SchemaVersion("users");  // → 2
```

## UDF (User-Defined Functions)

```csharp
db.RegisterScalarUdf("double", (double x) => x * 2);
var results = db.ExecuteSql("SELECT double(price) FROM products");
```

## Triggers

```csharp
db.RegisterTrigger("users", "after_insert", (event) =>
{
    Console.WriteLine($"New user: {event.NewValues}");
});
```

## Scheduler

```csharp
db.ScheduleJob("cleanup", "0 0 * * *", () =>
{
    db.ExecuteSql("DELETE FROM sessions WHERE expired = 1");
});
```

## Next Steps

- [Examples](examples) - More examples
