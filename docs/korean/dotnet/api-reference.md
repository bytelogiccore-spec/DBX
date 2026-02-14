---
layout: default
title: API 레퍼런스
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 6
---

# API 레퍼런스

## Database 클래스

### 생성자

#### `Database.Open(string path): Database`

파일 기반 데이터베이스를 엽니다.

**매개변수:**
- `path` (string): 데이터베이스 파일 경로

**반환:** `Database` 인스턴스

**예제:**
```csharp
var db = Database.Open("mydb.db");
```

#### `Database.OpenInMemory(): Database`

인메모리 데이터베이스를 엽니다.

**반환:** `Database` 인스턴스

**예제:**
```csharp
var db = Database.OpenInMemory();
```

### Key-Value 메서드

#### `Insert(string table, byte[] key, byte[] value): void`

키-값 쌍을 삽입합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (byte[]): 키
- `value` (byte[]): 값

**예제:**
```csharp
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
```

#### `Get(string table, byte[] key): byte[]?`

키로 값을 조회합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (byte[]): 키

**반환:** 값 (byte[]) 또는 null

**예제:**
```csharp
var value = db.Get("users", "user:1"u8.ToArray());
if (value != null)
{
    Console.WriteLine(Encoding.UTF8.GetString(value));
}
```

#### `Delete(string table, byte[] key): void`

키를 삭제합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (byte[]): 키

**예제:**
```csharp
db.Delete("users", "user:1"u8.ToArray());
```

#### `Count(string table): int`

테이블의 행 개수를 반환합니다.

**매개변수:**
- `table` (string): 테이블 이름

**반환:** 행 개수 (int)

**예제:**
```csharp
var count = db.Count("users");
Console.WriteLine($"Total: {count}");
```

### SQL 메서드

#### `ExecuteSql(string sql): string`

SQL 문을 실행합니다.

**매개변수:**
- `sql` (string): SQL 문

**반환:** 결과 (문자열, JSON 형식)

**예제:**
```csharp
// DDL
db.ExecuteSql("CREATE TABLE users (id INTEGER, name TEXT)");

// DML
db.ExecuteSql("INSERT INTO users VALUES (1, 'Alice')");

// 조회
var result = db.ExecuteSql("SELECT * FROM users");
Console.WriteLine(result);
```

### 트랜잭션 메서드

#### `BeginTransaction(): Transaction`

트랜잭션을 시작합니다.

**반환:** `Transaction` 객체

**예제:**
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

### 유틸리티 메서드

#### `Flush(): void`

버퍼를 디스크에 플러시합니다.

**예제:**
```csharp
db.Flush();
```

#### `Dispose(): void`

데이터베이스를 닫습니다. (IDisposable 구현)

**예제:**
```csharp
db.Dispose();
// 또는
using var db = Database.Open("mydb.db");
```

## Transaction 클래스

### 메서드

#### `Commit(): void`

트랜잭션을 커밋합니다.

**예제:**
```csharp
var tx = db.BeginTransaction();
db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
tx.Commit();
```

#### `Rollback(): void`

트랜잭션을 롤백합니다.

**예제:**
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

## 인터페이스

### IDisposable

`Database` 클래스는 `IDisposable`을 구현합니다.

```csharp
public class Database : IDisposable
{
    public void Dispose();
}
```

**사용 예제:**
```csharp
using (var db = Database.Open("mydb.db"))
{
    // 작업 수행
}  // 자동으로 Dispose() 호출
```

## 예외

### DbxException

DBX 관련 모든 예외의 기본 클래스.

**예제:**
```csharp
using DBX.Dotnet;

try
{
    using var db = Database.Open("mydb.db");
    db.Insert("users", "user:1"u8.ToArray(), "Alice"u8.ToArray());
}
catch (DbxException ex)
{
    Console.WriteLine($"Error: {ex.Message}");
}
```

## 확장 메서드 (선택 사항)

```csharp
public static class DatabaseExtensions
{
    public static void InsertJson<T>(this Database db, string table, string key, T value)
    {
        var json = JsonSerializer.Serialize(value);
        db.Insert(table, 
            Encoding.UTF8.GetBytes(key), 
            Encoding.UTF8.GetBytes(json));
    }

    public static T? GetJson<T>(this Database db, string table, string key)
    {
        var bytes = db.Get(table, Encoding.UTF8.GetBytes(key));
        if (bytes == null) return default;

        var json = Encoding.UTF8.GetString(bytes);
        return JsonSerializer.Deserialize<T>(json);
    }
}

// 사용 예제
var user = new { Name = "Alice", Email = "alice@example.com" };
db.InsertJson("users", "user:1", user);

var retrieved = db.GetJson<dynamic>("users", "user:1");
Console.WriteLine(retrieved.Name);
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [실전 예제](examples) - 실무 활용 예제
