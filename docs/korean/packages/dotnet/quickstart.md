---
layout: default
title: 빠른 시작
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 2
---

# 빠른 시작

5분 안에 DBX를 시작해보세요!

## 설치

```bash
dotnet add package DBX.Dotnet
```

## 첫 번째 프로그램

```csharp
using DBX.Dotnet;
using System.Text;

// 인메모리 데이터베이스
using var db = Database.OpenInMemory();

// KV 작업
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
var value = db.Get("users", "user:1"u8.ToArray());
Console.WriteLine(Encoding.UTF8.GetString(value));  // Alice

// SQL 작업
db.ExecuteSql("CREATE TABLE users (id INTEGER, name TEXT)");
db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice')");
var result = db.ExecuteSql("SELECT * FROM users");
Console.WriteLine(result);
```

## Using 문 사용

```csharp
using (var db = Database.Open("mydb.db"))
{
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
    // 자동으로 Flush() 및 Dispose()
}
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [API 레퍼런스](api-reference) - 전체 API
