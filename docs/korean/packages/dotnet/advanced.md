---
layout: default
title: 고급 기능
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능

## 트랜잭션

```csharp
using DBX.Dotnet;

using var db = Database.Open("mydb.db");

var tx = db.BeginTransaction();
try
{
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    db.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());
    tx.Commit();
}
catch (Exception ex)
{
    tx.Rollback();
    Console.WriteLine($"Transaction failed: {ex.Message}");
}
```

## 비동기 패턴

```csharp
public class AsyncRepository
{
    private readonly Database _db;

    public AsyncRepository(string dbPath)
    {
        _db = Database.Open(dbPath);
    }

    public async Task<int> CreateUserAsync(string name, string email)
    {
        return await Task.Run(() =>
        {
            var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            _db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{name}', '{email}')");
            return id;
        });
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}
```

## 성능 튜닝

### 배치 작업

```csharp
var tx = db.BeginTransaction();
for (int i = 0; i < 10000; i++)
{
    db.Insert("data", $"key:{i}"u8.ToArray(), $"value:{i}"u8.ToArray());
}
tx.Commit();
db.Flush();
```

### Span<byte> 사용 (.NET 6+)

```csharp
Span<byte> keyBuffer = stackalloc byte[20];
for (int i = 0; i < 10000; i++)
{
    var key = Encoding.UTF8.GetBytes($"key:{i}", keyBuffer);
    db.Insert("data", keyBuffer.ToArray(), "value"u8.ToArray());
}
```

## 멀티스레딩

```csharp
var tasks = new List<Task>();

for (int threadId = 0; threadId < 4; threadId++)
{
    int id = threadId;
    tasks.Add(Task.Run(() =>
    {
        using var db = Database.Open("mydb.db");
        for (int i = 0; i < 1000; i++)
        {
            var key = Encoding.UTF8.GetBytes($"thread:{id}:key:{i}");
            var value = Encoding.UTF8.GetBytes($"value:{i}");
            db.Insert("data", key, value);
        }
    }));
}

await Task.WhenAll(tasks);
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
