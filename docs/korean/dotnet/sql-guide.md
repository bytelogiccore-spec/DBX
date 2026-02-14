---
layout: default
title: SQL 가이드
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 4
---

# SQL 가이드

DBX는 표준 SQL을 지원합니다. .NET에서 강력한 타입 안전성과 함께 사용할 수 있습니다.

## 테이블 생성 (CREATE TABLE)

```csharp
using DBX.Dotnet;

using var db = Database.Open("mydb.db");

// 기본 테이블
db.ExecuteSql(@"
  CREATE TABLE users (
    id INTEGER,
    name TEXT,
    email TEXT,
    age INTEGER
  )
");

// Primary Key 지정
db.ExecuteSql(@"
  CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    price REAL
  )
");
```

## 데이터 삽입 (INSERT)

### 단일 행 삽입

```csharp
// 기본 INSERT
db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// 컬럼 명시
db.ExecuteSql(@"
  INSERT INTO users (id, name, email) 
  VALUES (2, 'Bob', 'bob@example.com')
");
```

### 다중 행 삽입

```csharp
// 배치 삽입
var users = new[]
{
    new { Id = 1, Name = "Alice", Email = "alice@example.com", Age = 25 },
    new { Id = 2, Name = "Bob", Email = "bob@example.com", Age = 30 },
    new { Id = 3, Name = "Carol", Email = "carol@example.com", Age = 28 }
};

foreach (var user in users)
{
    db.ExecuteSql(
        $"INSERT INTO users VALUES ({user.Id}, '{user.Name}', '{user.Email}', {user.Age})"
    );
}
```

## 데이터 조회 (SELECT)

### 기본 조회

```csharp
// 전체 조회
var result = db.ExecuteSql("SELECT * FROM users");
Console.WriteLine(result);

// 특정 컬럼
var names = db.ExecuteSql("SELECT name, email FROM users");

// WHERE 조건
var adults = db.ExecuteSql("SELECT * FROM users WHERE age >= 18");
```

### 정렬 및 제한

```csharp
// ORDER BY
var sorted = db.ExecuteSql("SELECT * FROM users ORDER BY age DESC");

// LIMIT
var top10 = db.ExecuteSql("SELECT * FROM users LIMIT 10");

// OFFSET
var page2 = db.ExecuteSql("SELECT * FROM users LIMIT 10 OFFSET 10");
```

### 집계 함수

```csharp
// COUNT
var count = db.ExecuteSql("SELECT COUNT(*) FROM users");

// AVG, SUM, MIN, MAX
var stats = db.ExecuteSql(@"
  SELECT 
    AVG(age) as avg_age,
    MIN(age) as min_age,
    MAX(age) as max_age
  FROM users
");

// GROUP BY
var ageGroups = db.ExecuteSql(@"
  SELECT age, COUNT(*) as count
  FROM users
  GROUP BY age
");
```

## 데이터 수정 (UPDATE)

```csharp
// 단일 컬럼 수정
db.ExecuteSql("UPDATE users SET age = 26 WHERE id = 1");

// 다중 컬럼 수정
db.ExecuteSql(@"
  UPDATE users 
  SET name = 'Alice Smith', email = 'alice.smith@example.com'
  WHERE id = 1
");

// 조건부 수정
db.ExecuteSql("UPDATE users SET age = age + 1 WHERE age < 30");
```

## 데이터 삭제 (DELETE)

```csharp
// 특정 행 삭제
db.ExecuteSql("DELETE FROM users WHERE id = 1");

// 조건부 삭제
db.ExecuteSql("DELETE FROM users WHERE age < 18");

// 전체 삭제 (주의!)
db.ExecuteSql("DELETE FROM users");
```

## 트랜잭션과 함께 사용

```csharp
var tx = db.BeginTransaction();

try
{
    db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");
    db.ExecuteSql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)");
    
    tx.Commit();
}
catch (Exception ex)
{
    tx.Rollback();
    Console.WriteLine($"Transaction failed: {ex.Message}");
}
```

## 강력한 타입 안전성

```csharp
public class User
{
    public int Id { get; set; }
    public string Name { get; set; }
    public string Email { get; set; }
    public int Age { get; set; }
}

public class UserRepository
{
    private readonly Database _db;

    public UserRepository(string dbPath)
    {
        _db = Database.Open(dbPath);
        InitSchema();
    }

    private void InitSchema()
    {
        _db.ExecuteSql(@"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                age INTEGER
            )
        ");
    }

    public int CreateUser(User user)
    {
        var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        _db.ExecuteSql(
            $"INSERT INTO users (id, name, email, age) " +
            $"VALUES ({id}, '{user.Name}', '{user.Email}', {user.Age})"
        );
        return id;
    }

    public string GetUser(int id)
    {
        return _db.ExecuteSql($"SELECT * FROM users WHERE id = {id}");
    }

    public void UpdateUser(int id, User user)
    {
        _db.ExecuteSql(
            $"UPDATE users SET " +
            $"name = '{user.Name}', " +
            $"email = '{user.Email}', " +
            $"age = {user.Age} " +
            $"WHERE id = {id}"
        );
    }

    public void DeleteUser(int id)
    {
        _db.ExecuteSql($"DELETE FROM users WHERE id = {id}");
    }

    public string ListUsers(int limit = 100)
    {
        return _db.ExecuteSql($"SELECT * FROM users LIMIT {limit}");
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}

// 사용 예제
using var repo = new UserRepository("users.db");

var userId = repo.CreateUser(new User
{
    Name = "Alice",
    Email = "alice@example.com",
    Age = 25
});

Console.WriteLine($"Created user: {userId}");

var user = repo.GetUser(userId);
Console.WriteLine($"User: {user}");

repo.UpdateUser(userId, new User
{
    Name = "Alice Smith",
    Email = "alice.smith@example.com",
    Age = 26
});

var users = repo.ListUsers();
Console.WriteLine($"All users: {users}");
```

## ASP.NET Core 통합

```csharp
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using DBX.Dotnet;
using System.Text.Json;

var builder = WebApplication.CreateBuilder(args);

// DBX를 싱글톤으로 등록
builder.Services.AddSingleton<Database>(sp =>
{
    var db = Database.Open("api.db");
    db.ExecuteSql(@"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL
        )
    ");
    return db;
});

var app = builder.Build();

// 사용자 생성
app.MapPost("/users", (Database db, User user) =>
{
    var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
    db.ExecuteSql(
        $"INSERT INTO users (id, name, email) VALUES ({id}, '{user.Name}', '{user.Email}')"
    );
    return Results.Ok(new { id, user.Name, user.Email });
});

// 사용자 조회
app.MapGet("/users/{id}", (Database db, int id) =>
{
    var result = db.ExecuteSql($"SELECT * FROM users WHERE id = {id}");
    return Results.Ok(result);
});

// 사용자 목록
app.MapGet("/users", (Database db, int limit = 100) =>
{
    var result = db.ExecuteSql($"SELECT * FROM users LIMIT {limit}");
    return Results.Ok(result);
});

// 사용자 수정
app.MapPut("/users/{id}", (Database db, int id, User user) =>
{
    db.ExecuteSql(
        $"UPDATE users SET name = '{user.Name}', email = '{user.Email}' WHERE id = {id}"
    );
    return Results.Ok(new { id, user.Name, user.Email });
});

// 사용자 삭제
app.MapDelete("/users/{id}", (Database db, int id) =>
{
    db.ExecuteSql($"DELETE FROM users WHERE id = {id}");
    return Results.Ok(new { success = true });
});

app.Run();

public record User(string Name, string Email);
```

## 비동기 패턴

```csharp
public class AsyncUserRepository
{
    private readonly Database _db;

    public AsyncUserRepository(string dbPath)
    {
        _db = Database.Open(dbPath);
        InitSchema();
    }

    private void InitSchema()
    {
        _db.ExecuteSql(@"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL
            )
        ");
    }

    public async Task<int> CreateUserAsync(User user)
    {
        return await Task.Run(() =>
        {
            var id = (int)DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
            _db.ExecuteSql(
                $"INSERT INTO users (id, name, email) " +
                $"VALUES ({id}, '{user.Name}', '{user.Email}')"
            );
            return id;
        });
    }

    public async Task<string> GetUserAsync(int id)
    {
        return await Task.Run(() =>
            _db.ExecuteSql($"SELECT * FROM users WHERE id = {id}")
        );
    }

    public async Task UpdateUserAsync(int id, User user)
    {
        await Task.Run(() =>
            _db.ExecuteSql(
                $"UPDATE users SET name = '{user.Name}', email = '{user.Email}' " +
                $"WHERE id = {id}"
            )
        );
    }

    public async Task DeleteUserAsync(int id)
    {
        await Task.Run(() =>
            _db.ExecuteSql($"DELETE FROM users WHERE id = {id}")
        );
    }

    public void Dispose()
    {
        _db?.Dispose();
    }
}

// 사용 예제
using var repo = new AsyncUserRepository("users.db");

var userId = await repo.CreateUserAsync(new User("Alice", "alice@example.com"));
Console.WriteLine($"Created user: {userId}");

var user = await repo.GetUserAsync(userId);
Console.WriteLine($"User: {user}");
```

## 성능 팁

### 1. 배치 작업

```csharp
// ❌ 느림
for (int i = 0; i < 1000; i++)
{
    db.ExecuteSql($"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)");
}

// ✅ 빠름 (트랜잭션 사용)
var tx = db.BeginTransaction();
for (int i = 0; i < 1000; i++)
{
    db.ExecuteSql($"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)");
}
tx.Commit();
```

### 2. SQL Injection 방지

```csharp
// 입력값 검증
public static string Sanitize(string input)
{
    return input.Replace("'", "''");
}

var name = Sanitize(userInput);
db.ExecuteSql($"INSERT INTO users (name) VALUES ('{name}')");
```

## 제한사항

- **JOIN**: 현재 미지원 (향후 지원 예정)
- **서브쿼리**: 제한적 지원
- **외래 키**: 현재 미지원

## 다음 단계

- [고급 기능](advanced) - 트랜잭션, 암호화
- [API 레퍼런스](api-reference) - 전체 API
- [실전 예제](examples) - 더 많은 예제
