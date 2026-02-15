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

## 기능 플래그

```csharp
// 런타임에 기능 활성화/비활성화
db.EnableFeature("parallel_query");
db.EnableFeature("query_plan_cache");
db.DisableFeature("parallel_query");

if (db.IsFeatureEnabled("parallel_query"))
{
    Console.WriteLine("병렬 쿼리 활성화됨");
}
```

## 쿼리 플랜 캐시

```csharp
db.EnableFeature("query_plan_cache");

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (int i = 0; i < 100; i++)
{
    var results = db.ExecuteSql("SELECT * FROM users WHERE age > 20");
}
```

## 스키마 버저닝

```csharp
db.ExecuteSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.ExecuteSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

var version = db.SchemaVersion("users");  // → 2
```

## UDF (사용자 정의 함수)

```csharp
// 스칼라 UDF 등록
db.RegisterScalarUdf("double", (double x) => x * 2);

// SQL에서 사용
var results = db.ExecuteSql("SELECT double(price) FROM products");
```

## 트리거

```csharp
db.RegisterTrigger("users", "after_insert", (event) =>
{
    Console.WriteLine($"새 사용자: {event.NewValues}");
});
```

## 스케줄러

```csharp
db.ScheduleJob("cleanup", "0 0 * * *", () =>
{
    db.ExecuteSql("DELETE FROM sessions WHERE expired = 1");
});
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
