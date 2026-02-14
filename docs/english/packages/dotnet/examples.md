---
layout: default
title: Examples
parent: .NET (DBX.Dotnet)
grand_parent: Packages
great_grand_parent: English
nav_order: 7
---

# Real-World Examples

## ASP.NET Core Minimal API

```csharp
using DBX.Dotnet;

var builder = WebApplication.CreateBuilder(args);
builder.Services.AddSingleton<Database>(sp => Database.Open("api.db"));

var app = builder.Build();

app.MapPost("/users", (Database db, User user) =>
{
    var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    db.ExecuteSql($"INSERT INTO users VALUES ({id}, '{user.Name}', '{user.Email}')");
    return Results.Ok(new { id, user.Name, user.Email });
});

app.Run();

record User(string Name, string Email);
```

## SignalR Real-time Chat

```csharp
public class ChatHub : Hub
{
    private readonly Database _db;

    public ChatHub(Database db) => _db = db;

    public async Task SendMessage(string user, string message)
    {
        var id = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        _db.ExecuteSql($"INSERT INTO messages VALUES ({id}, '{user}', '{message}', {id})");
        await Clients.All.SendAsync("ReceiveMessage", user, message);
    }
}
```

## Background Service

```csharp
public class DataProcessingService : BackgroundService
{
    private readonly Database _db;

    public DataProcessingService(Database db) => _db = db;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        while (!stoppingToken.IsCancellationRequested)
        {
            var result = _db.ExecuteSql("SELECT * FROM pending_jobs LIMIT 10");
            await Task.Delay(TimeSpan.FromSeconds(10), stoppingToken);
        }
    }
}
```

## Next Steps

- [API Reference](api-reference) - Complete API
