---
layout: default
title: .NET (DBX.Dotnet)
parent: Packages
grand_parent: 한국어
nav_order: 2
---

# .NET — DBX.Dotnet

[![NuGet](https://img.shields.io/nuget/v/DBX.Dotnet.svg)](https://www.nuget.org/packages/DBX.Dotnet)

관용적인 C# API를 제공하는 DBX 임베디드 데이터베이스용 고성능 .NET 바인딩입니다.

## 설치

```bash
dotnet add package DBX.Dotnet
```

또는 패키지 관리자를 통해:

```powershell
Install-Package DBX.Dotnet
```

## 빠른 시작

```csharp
using DBX.Dotnet;

// 인메모리 데이터베이스 열기
using var db = Database.OpenInMemory();

// 삽입
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
db.Insert("users", "user:2"u8.ToArray(), "Bob"u8.ToArray());

// 조회
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
{
    Console.WriteLine(Encoding.UTF8.GetString(value)); // Alice
}

// 삭제
db.Delete("users", "user:2"u8.ToArray());

// 개수 확인
var count = db.Count("users");
Console.WriteLine($"전체 사용자: {count}");
```

## 고급 사용법

### Using 문 사용 (권장)

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
} // 자동으로 해제되고 플러시됨
```

### JSON 데이터 다루기

```csharp
using DBX.Dotnet;
using System.Text.Json;

public record User(int Id, string Name, string Email);

using var db = Database.OpenInMemory();

// JSON 데이터 저장
var user = new User(1, "Alice", "alice@example.com");
var json = JsonSerializer.Serialize(user);
db.Insert("users", "user:1"u8.ToArray(), Encoding.UTF8.GetBytes(json));

// JSON 데이터 조회
var data = db.Get("users", "user:1"u8.ToArray());
if (data != null)
{
    var retrievedUser = JsonSerializer.Deserialize<User>(Encoding.UTF8.GetString(data));
    Console.WriteLine(retrievedUser?.Name); // Alice
}
```

### 배치 작업

```csharp
using DBX.Dotnet;

using var db = Database.Open("data.db");

// 배치 삽입
for (int i = 0; i < 1000; i++)
{
    var key = Encoding.UTF8.GetBytes($"item:{i}");
    var value = Encoding.UTF8.GetBytes($"value_{i}");
    db.Insert("items", key, value);
}

// 디스크에 플러시
db.Flush();
```

### 에러 처리

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
    Console.WriteLine($"데이터베이스 오류: {ex.Message}");
}
```

### 비동기 패턴

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

### ASP.NET Core 통합

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

## API 레퍼런스

### Database 클래스

#### 정적 메서드

| 메서드 | 매개변수 | 반환 | 설명 |
|--------|----------|------|------|
| `Database.Open(path)` | `string path` | `Database` | 파일 기반 데이터베이스 열기 |
| `Database.OpenInMemory()` | - | `Database` | 인메모리 데이터베이스 열기 |

#### 인스턴스 메서드

| 메서드 | 매개변수 | 반환 | 설명 |
|--------|----------|------|------|
| `Insert` | `string table, byte[] key, byte[] value` | `void` | 키-값 쌍 삽입 |
| `Get` | `string table, byte[] key` | `byte[]?` | 키로 값 조회 |
| `Delete` | `string table, byte[] key` | `void` | 키 삭제 |
| `Count` | `string table` | `int` | 테이블 행 개수 |
| `Flush` | - | `void` | 디스크에 플러시 |
| `Dispose` | - | `void` | 데이터베이스 닫기 (IDisposable) |

## 타입 정의

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

## 성능 팁

1. **`using` 문 사용**: 올바른 해제 보장
2. **배치 작업**: `Flush()` 호출 전에 삽입 그룹화
3. **UTF-8 리터럴**: 효율적인 인코딩을 위해 `"text"u8.ToArray()` 사용
4. **싱글톤 패턴**: DI 컨테이너에서 데이터베이스 인스턴스 재사용

## 예제

### 간단한 Key-Value 저장소

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

// 사용법
using var store = new KVStore("store.db");
store.Set("name", "Alice");
Console.WriteLine(store.Get("name")); // Alice
```

### 세션 매니저

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

// 사용법
using var sessions = new SessionManager();
sessions.CreateSession("sess_123", new { UserId = 42, Role = "admin" });
var data = sessions.GetSession<dynamic>("sess_123");
Console.WriteLine(data?.UserId);
```

### 캐시 래퍼

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
        // 캐시 확인
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
        
        // 함수 실행
        var result = await fn();
        
        // 캐시에 저장
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

// 사용법
using var cache = new Cache();

async Task<string> ExpensiveOperation()
{
    await Task.Delay(1000);
    return "비용이 많이 드는 결과";
}

var result = await cache.WrapAsync("my-key", ExpensiveOperation);
Console.WriteLine(result); // 첫 번째 호출: 1초 소요

var cached = await cache.WrapAsync("my-key", ExpensiveOperation);
Console.WriteLine(cached); // 두 번째 호출: 즉시 반환
```

## 요구사항

- .NET Standard 2.0+ (.NET Framework 4.6.1+, .NET Core 2.0+, .NET 5+)
- **Windows x64 전용** (Linux/macOS 지원 예정)

## 문제 해결

### DllNotFoundException

```bash
# 네이티브 라이브러리가 출력 디렉터리에 있는지 확인
dotnet build
# bin/Debug/net8.0/에서 dbx_native.dll 확인
```

### 성능 문제

```csharp
// 배치 작업 사용
using var db = Database.Open("data.db");

// 나쁨: 매번 삽입 후 플러시
for (int i = 0; i < 10000; i++)
{
    db.Insert("items", $"k{i}"u8.ToArray(), $"v{i}"u8.ToArray());
    db.Flush(); // ❌ 느림
}

// 좋음: 마지막에 한 번만 플러시
for (int i = 0; i < 10000; i++)
{
    db.Insert("items", $"k{i}"u8.ToArray(), $"v{i}"u8.ToArray());
}
db.Flush(); // ✅ 빠름
```

## 플랫폼 지원

| 플랫폼 | 아키텍처 | 상태 |
|--------|----------|------|
| Windows | x64 | ✅ 테스트 완료 |
| Linux | x64 | ⚠️ 계획됨 |
| macOS | x64 (Intel) | ⚠️ 계획됨 |
| macOS | ARM64 (Apple Silicon) | ⚠️ 계획됨 |

## 라이선스

MIT 또는 Commercial 라이선스로 이중 라이선스됩니다.
