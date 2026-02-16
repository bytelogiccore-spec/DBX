---
layout: default
title: 고급 기능
parent: .NET (DBX.Dotnet)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능
{: .no_toc }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

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

---

## SQL Trigger

SQL 표준 문법으로 데이터 변경 시 자동 실행되는 로직을 정의합니다.

### CREATE TRIGGER

```csharp
// 감사 로그 Trigger
db.ExecuteSql(@"
    CREATE TRIGGER audit_trigger
    AFTER INSERT ON users
    FOR EACH ROW
    BEGIN
        INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', datetime('now'));
    END;
");

// 데이터 검증 Trigger (WHEN 조건 사용)
db.ExecuteSql(@"
    CREATE TRIGGER validate_price
    BEFORE UPDATE ON products
    FOR EACH ROW
    WHEN (NEW.price < 0)
    BEGIN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '가격은 0 이상이어야 합니다';
    END;
");
```

### DROP TRIGGER

```csharp
db.ExecuteSql("DROP TRIGGER audit_trigger;");
```

### 실전 예제

#### 변경 이력 추적

```csharp
// 변경 이력 테이블 생성
db.ExecuteSql(@"
    CREATE TABLE change_log (
        id INT,
        old_price DECIMAL,
        new_price DECIMAL,
        changed_at TIMESTAMP
    )
");

// UPDATE 시 변경 이력 기록
db.ExecuteSql(@"
    CREATE TRIGGER track_price_changes
    AFTER UPDATE ON products
    FOR EACH ROW
    BEGIN
        INSERT INTO change_log VALUES (
            OLD.id, OLD.price, NEW.price, datetime('now')
        );
    END;
");
```

#### 조건부 알림

```csharp
// 고액 주문 알림
db.ExecuteSql(@"
    CREATE TRIGGER high_value_alert
    AFTER INSERT ON orders
    FOR EACH ROW
    WHEN (NEW.total > 10000)
    BEGIN
        INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER', datetime('now'));
    END;
");
```

---

## Stored Procedure

재사용 가능한 SQL 프로시저를 정의하고 호출합니다.

### CREATE PROCEDURE

```csharp
// 잔액 업데이트 프로시저
db.ExecuteSql(@"
    CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
    BEGIN
        UPDATE accounts SET balance = balance + amount WHERE id = user_id;
        INSERT INTO transactions VALUES (user_id, amount, datetime('now'));
    END;
");

// 데이터 정리 프로시저
db.ExecuteSql(@"
    CREATE PROCEDURE cleanup_old_data (days_old INT)
    BEGIN
        DELETE FROM logs WHERE created_at < datetime('now', '-' || days_old || ' days');
        DELETE FROM temp_files WHERE created_at < datetime('now', '-' || days_old || ' days');
        UPDATE stats SET last_cleanup = datetime('now');
    END;
");
```

### CALL PROCEDURE

```csharp
// 프로시저 호출
db.ExecuteSql("CALL update_balance(123, 100.50);");
db.ExecuteSql("CALL cleanup_old_data(30);");
```

### DROP PROCEDURE

```csharp
db.ExecuteSql("DROP PROCEDURE update_balance;");
```

---

## Scheduler

Cron 표현식 기반으로 주기적인 작업을 자동 실행합니다.

### CREATE SCHEDULE

```csharp
// 매 5분마다 통계 갱신
db.ExecuteSql(@"
    CREATE SCHEDULE refresh_stats
    CRON '*/5 * * * *'
    BEGIN
        UPDATE product_stats SET 
            total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id);
    END;
");

// 매일 자정에 정리 작업
db.ExecuteSql(@"
    CREATE SCHEDULE daily_cleanup
    CRON '0 0 * * *'
    BEGIN
        CALL cleanup_old_data(30);
    END;
");
```

### Cron 표현식

| 표현식 | 설명 |
|--------|------|
| `*/5 * * * *` | 5분마다 |
| `0 * * * *` | 매시 정각 |
| `0 0 * * *` | 매일 자정 |
| `0 0 * * 0` | 매주 일요일 자정 |
| `0 0 1 * *` | 매월 1일 자정 |

형식: `분 시 일 월 요일`

### DROP SCHEDULE

```csharp
db.ExecuteSql("DROP SCHEDULE refresh_stats;");
```

---

## UDF (사용자 정의 함수)

### CREATE FUNCTION (SQL)

```csharp
// SQL 표준 문법으로 함수 정의
db.ExecuteSql(@"
    CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
    BEGIN
        RETURN a + b;
    END;
");
```

> **참고**: 현재는 메타데이터 파싱만 지원하며, 실제 함수 로직은 C# 코드로 등록해야 합니다.

### C# 함수 등록

```csharp
// 스칼라 UDF 등록
db.RegisterScalarUdf("double", (double x) => x * 2);

// SQL에서 사용
var results = db.ExecuteSql("SELECT double(price) FROM products");
```

### 집계 UDF

```csharp
// 중앙값 계산
db.RegisterAggregateUdf<List<double>, double>(
    "median",
    () => new List<double>(),                    // 초기 상태
    (state, value) => { state.Add(value); return state; },  // 축적
    (state) =>                                   // 최종 계산
    {
        var sorted = state.OrderBy(x => x).ToList();
        return sorted[sorted.Count / 2];
    }
);

// SQL에서 사용
var results = db.ExecuteSql("SELECT median(score) FROM students");
```

---

## Event Hook (Rust 클로저 기반)

C#에서는 SQL Trigger를 사용하는 것을 권장하지만, 향후 C# 델리게이트 지원 예정입니다.

```csharp
// 향후 지원 예정
// db.RegisterEventHook("users", "after_insert", (event) =>
// {
//     Console.WriteLine($"새 사용자: {event.NewValues}");
// });
```

---

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

---

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

---

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

---

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

---

## 쿼리 플랜 캐시

```csharp
db.EnableFeature("query_plan_cache");

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (int i = 0; i < 100; i++)
{
    var results = db.ExecuteSql("SELECT * FROM users WHERE age > 20");
}
```

---

## 스키마 버저닝

```csharp
db.ExecuteSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.ExecuteSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

var version = db.SchemaVersion("users");  // → 2
```

---

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
