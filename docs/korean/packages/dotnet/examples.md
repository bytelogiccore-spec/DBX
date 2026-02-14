---
layout: default
title: 실전 예제
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 7
---

# 실전 예제

## ASP.NET Core Minimal API

```csharp
using DBX.Dotnet;

var builder = WebApplication.CreateBuilder(args);

builder.Services.AddSingleton<Database>(sp =>
{
    var db = Database.Open("api.db");
    db.ExecuteSql("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT, email TEXT)");
    return db;
});

var app = builder.Build();

app.MapPost("/users", (Database db, User user) =>
{
    var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{user.Name}', '{user.Email}')");
    return Results.Ok(new { id, user.Name, user.Email });
});

app.MapGet("/users/{id}", (Database db, int id) =>
{
    var result = db.ExecuteSql($"SELECT * FROM users WHERE id = {id}");
    return Results.Ok(result);
});

app.Run();

public record User(string Name, string Email);
```

## SignalR 실시간 채팅

```csharp
using Microsoft.AspNetCore.SignalR;
using DBX.Dotnet;

public class ChatHub : Hub
{
    private readonly Database _db;

    public ChatHub(Database db)
    {
        _db = db;
    }

    public async Task SendMessage(string user, string message)
    {
        var id = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        
        _db.ExecuteSql(
            $"INSERT INTO messages VALUES ({id}, '{user}', '{message}', {id})"
        );

        await Clients.All.SendAsync("ReceiveMessage", user, message);
    }
}

// Startup.cs
builder.Services.AddSingleton<Database>(sp =>
{
    var db = Database.Open("chat.db");
    db.ExecuteSql("CREATE TABLE IF NOT EXISTS messages (id INTEGER, user TEXT, message TEXT, timestamp INTEGER)");
    return db;
});

builder.Services.AddSignalR();
app.MapHub<ChatHub>("/chatHub");
```

## 백그라운드 서비스

```csharp
public class DataProcessingService : BackgroundService
{
    private readonly Database _db;
    private readonly ILogger<DataProcessingService> _logger;

    public DataProcessingService(Database db, ILogger<DataProcessingService> logger)
    {
        _db = db;
        _logger = logger;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            try
            {
                // 데이터 처리
                var result = _db.ExecuteSql("SELECT * FROM pending_jobs LIMIT 10");
                _logger.LogInformation($"Processed: {result}");

                await Task.Delay(TimeSpan.FromSeconds(10), stoppingToken);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error processing data");
            }
        }
    }
}

// Program.cs
builder.Services.AddHostedService<DataProcessingService>();
```

## 메모리 캐시 통합

```csharp
using Microsoft.Extensions.Caching.Memory;

public class CachedRepository
{
    private readonly Database _db;
    private readonly IMemoryCache _cache;

    public CachedRepository(Database db, IMemoryCache cache)
    {
        _db = db;
        _cache = cache;
    }

    public string GetUser(int id)
    {
        return _cache.GetOrCreate($"user:{id}", entry =>
        {
            entry.AbsoluteExpirationRelativeToNow = TimeSpan.FromMinutes(5);
            return _db.ExecuteSql($"SELECT * FROM users WHERE id = {id}");
        });
    }
}
```

## 다음 단계

- [API 레퍼런스](api-reference) - 전체 API
- [고급 기능](advanced) - 트랜잭션, 성능 튜닝
