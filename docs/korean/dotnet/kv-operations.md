---
layout: default
title: KV 작업
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 3
---

# Key-Value 작업

DBX는 SQL 외에도 고성능 Key-Value 스토어로 사용할 수 있습니다.

## 기본 CRUD

### 삽입 (Insert)

```csharp
using DBX.Dotnet;
using System.Text;

using var db = Database.OpenInMemory();

// 기본 삽입
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());

// JSON 데이터
using System.Text.Json;

var user = new { Id = 1, Name = "Alice", Email = "alice@example.com" };
var json = JsonSerializer.Serialize(user);
db.Insert("users", "user:1"u8.ToArray(), Encoding.UTF8.GetBytes(json));

// 바이너리 데이터
db.Insert("files", "file:1"u8.ToArray(), new byte[] { 0x89, 0x50, 0x4E, 0x47 });
```

### 조회 (Get)

```csharp
// 단일 조회
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
{
    Console.WriteLine(Encoding.UTF8.GetString(value));  // Alice
}

// JSON 파싱
var userBytes = db.Get("users", "user:1"u8.ToArray());
if (userBytes != null)
{
    var userJson = Encoding.UTF8.GetString(userBytes);
    var user = JsonSerializer.Deserialize<dynamic>(userJson);
    Console.WriteLine(user.Name);  // Alice
}
```

### 삭제 (Delete)

```csharp
db.Delete("users", "user:1"u8.ToArray());

// 존재 확인 후 삭제
if (db.Get("users", "user:1"u8.ToArray()) != null)
{
    db.Delete("users", "user:1"u8.ToArray());
    Console.WriteLine("Deleted");
}
```

### 개수 확인 (Count)

```csharp
var count = db.Count("users");
Console.WriteLine($"Total users: {count}");
```

## 배치 작업

```csharp
// 대량 삽입
for (int i = 0; i < 10000; i++)
{
    var key = Encoding.UTF8.GetBytes($"user:{i}");
    var value = Encoding.UTF8.GetBytes($"User {i}");
    db.Insert("users", key, value);
}

// 플러시
db.Flush();
```

## 실전 예제

### 세션 저장소

```csharp
using System.Text.Json;

public class SessionData
{
    public int UserId { get; set; }
    public string Username { get; set; } = string.Empty;
    public string Role { get; set; } = string.Empty;
}

public class SessionStore : IDisposable
{
    private readonly Database _db;

    public SessionStore(string dbPath)
    {
        _db = Database.Open(dbPath);
    }

    public void CreateSession(string sessionId, SessionData data, int ttlSeconds = 3600)
    {
        var session = new
        {
            Data = data,
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
            ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(ttlSeconds).ToUnixTimeMilliseconds()
        };

        var json = JsonSerializer.Serialize(session);
        _db.Insert("sessions", 
            Encoding.UTF8.GetBytes(sessionId), 
            Encoding.UTF8.GetBytes(json));
    }

    public SessionData? GetSession(string sessionId)
    {
        var bytes = _db.Get("sessions", Encoding.UTF8.GetBytes(sessionId));
        if (bytes == null) return null;

        var json = Encoding.UTF8.GetString(bytes);
        var session = JsonSerializer.Deserialize<dynamic>(json);

        // 만료 확인
        var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        if (now > (long)session.ExpiresAt)
        {
            _db.Delete("sessions", Encoding.UTF8.GetBytes(sessionId));
            return null;
        }

        return JsonSerializer.Deserialize<SessionData>(session.Data.ToString());
    }

    public void DeleteSession(string sessionId)
    {
        _db.Delete("sessions", Encoding.UTF8.GetBytes(sessionId));
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}

// 사용 예제
using var store = new SessionStore("sessions.db");

store.CreateSession("sess_abc123", new SessionData
{
    UserId = 42,
    Username = "alice",
    Role = "admin"
}, 3600);

var session = store.GetSession("sess_abc123");
if (session != null)
{
    Console.WriteLine($"User: {session.Username}");
}

store.DeleteSession("sess_abc123");
```

### 제네릭 캐시 시스템

```csharp
public class Cache<T> : IDisposable where T : class
{
    private readonly Database _db;
    private readonly int _defaultTtlSeconds;

    public Cache(string dbPath, int defaultTtlSeconds = 300)
    {
        _db = Database.Open(dbPath);
        _defaultTtlSeconds = defaultTtlSeconds;
    }

    public void Set(string key, T value, int? ttlSeconds = null)
    {
        var ttl = ttlSeconds ?? _defaultTtlSeconds;
        var cacheData = new
        {
            Value = value,
            ExpiresAt = DateTimeOffset.UtcNow.AddSeconds(ttl).ToUnixTimeMilliseconds()
        };

        var json = JsonSerializer.Serialize(cacheData);
        _db.Insert("cache", 
            Encoding.UTF8.GetBytes(key), 
            Encoding.UTF8.GetBytes(json));
    }

    public T? Get(string key)
    {
        var bytes = _db.Get("cache", Encoding.UTF8.GetBytes(key));
        if (bytes == null) return null;

        var json = Encoding.UTF8.GetString(bytes);
        var cacheData = JsonSerializer.Deserialize<dynamic>(json);

        // 만료 확인
        var now = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        if (now > (long)cacheData.ExpiresAt)
        {
            _db.Delete("cache", Encoding.UTF8.GetBytes(key));
            return null;
        }

        return JsonSerializer.Deserialize<T>(cacheData.Value.ToString());
    }

    public void Delete(string key)
    {
        _db.Delete("cache", Encoding.UTF8.GetBytes(key));
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}

// 사용 예제
public class User
{
    public string Name { get; set; } = string.Empty;
    public string Email { get; set; } = string.Empty;
}

using var cache = new Cache<User>("cache.db", 300);

cache.Set("user:1", new User 
{ 
    Name = "Alice", 
    Email = "alice@example.com" 
});

var user = cache.Get("user:1");
if (user != null)
{
    Console.WriteLine($"Cached user: {user.Name}");
}
else
{
    Console.WriteLine("Cache miss");
}
```

## 성능 최적화

### 1. 배치 작업 + 플러시

```csharp
// ❌ 느림
for (int i = 0; i < 10000; i++)
{
    db.Insert("data", $"key:{i}"u8.ToArray(), $"value:{i}"u8.ToArray());
    db.Flush();  // 매번 플러시
}

// ✅ 빠름
for (int i = 0; i < 10000; i++)
{
    db.Insert("data", $"key:{i}"u8.ToArray(), $"value:{i}"u8.ToArray());
}
db.Flush();  // 한 번만 플러시
```

### 2. Span<byte> 사용 (.NET 6+)

```csharp
// ✅ 빠름 (메모리 할당 최소화)
Span<byte> keyBuffer = stackalloc byte[20];
for (int i = 0; i < 10000; i++)
{
    var key = Encoding.UTF8.GetBytes($"key:{i}", keyBuffer);
    db.Insert("data", keyBuffer.ToArray(), "value"u8.ToArray());
}
```

### 3. Using 문 사용

```csharp
// ✅ 자동 Dispose
using (var db = Database.Open("data.db"))
{
    for (int i = 0; i < 10000; i++)
    {
        db.Insert("data", $"key:{i}"u8.ToArray(), $"value:{i}"u8.ToArray());
    }
}  // 자동으로 Flush() 및 Dispose() 호출
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화
- [API 레퍼런스](api-reference) - 전체 API
