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

## Next Steps

- [Examples](examples) - More examples
