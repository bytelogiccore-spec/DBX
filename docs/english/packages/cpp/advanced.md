---
layout: default
title: 고급 기능
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 7
---

# 고급 기능
{: .no_toc }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 트랜잭션 (C)

```c
DbxTransaction* tx = dbx_begin_transaction(db);
if (!tx) {
    fprintf(stderr, "Failed to begin transaction\n");
    return 1;
}

if (dbx_insert(db, "users", (uint8_t*)"user:1", 6, (uint8_t*)"Alice", 5) != 0) {
    dbx_rollback(tx);
    return 1;
}

if (dbx_commit(tx) != 0) {
    fprintf(stderr, "Commit failed\n");
    return 1;
}
```

---

## 트랜잭션 (C++)

```cpp
auto tx = db.beginTransaction();
try {
    db.insert("users", "user:1", "Alice");
    db.insert("users", "user:2", "Bob");
    tx.commit();
} catch (const std::exception& e) {
    tx.rollback();
    std::cerr << "Transaction failed: " << e.what() << std::endl;
}
```

---

## SQL Trigger

SQL 표준 문법으로 데이터 변경 시 자동 실행되는 로직을 정의합니다.

### CREATE TRIGGER (C++)

```cpp
// 감사 로그 Trigger
db.executeSql(R"(
    CREATE TRIGGER audit_trigger
    AFTER INSERT ON users
    FOR EACH ROW
    BEGIN
        INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', datetime('now'));
    END;
)");

// 데이터 검증 Trigger (WHEN 조건 사용)
db.executeSql(R"(
    CREATE TRIGGER validate_price
    BEFORE UPDATE ON products
    FOR EACH ROW
    WHEN (NEW.price < 0)
    BEGIN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '가격은 0 이상이어야 합니다';
    END;
)");
```

### CREATE TRIGGER (C)

```c
const char* sql = 
    "CREATE TRIGGER audit_trigger "
    "AFTER INSERT ON users "
    "FOR EACH ROW "
    "BEGIN "
    "    INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', datetime('now')); "
    "END;";

if (dbx_execute_sql(db, sql) != 0) {
    fprintf(stderr, "Failed to create trigger\n");
}
```

### DROP TRIGGER

```cpp
// C++
db.executeSql("DROP TRIGGER audit_trigger;");

// C
dbx_execute_sql(db, "DROP TRIGGER audit_trigger;");
```

### 실전 예제

#### 변경 이력 추적 (C++)

```cpp
// 변경 이력 테이블 생성
db.executeSql(R"(
    CREATE TABLE change_log (
        id INT,
        old_price DECIMAL,
        new_price DECIMAL,
        changed_at TIMESTAMP
    )
)");

// UPDATE 시 변경 이력 기록
db.executeSql(R"(
    CREATE TRIGGER track_price_changes
    AFTER UPDATE ON products
    FOR EACH ROW
    BEGIN
        INSERT INTO change_log VALUES (
            OLD.id, OLD.price, NEW.price, datetime('now')
        );
    END;
)");
```

#### 조건부 알림 (C++)

```cpp
// 고액 주문 알림
db.executeSql(R"(
    CREATE TRIGGER high_value_alert
    AFTER INSERT ON orders
    FOR EACH ROW
    WHEN (NEW.total > 10000)
    BEGIN
        INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER', datetime('now'));
    END;
)");
```

---

## Stored Procedure

재사용 가능한 SQL 프로시저를 정의하고 호출합니다.

### CREATE PROCEDURE (C++)

```cpp
// 잔액 업데이트 프로시저
db.executeSql(R"(
    CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
    BEGIN
        UPDATE accounts SET balance = balance + amount WHERE id = user_id;
        INSERT INTO transactions VALUES (user_id, amount, datetime('now'));
    END;
)");

// 데이터 정리 프로시저
db.executeSql(R"(
    CREATE PROCEDURE cleanup_old_data (days_old INT)
    BEGIN
        DELETE FROM logs WHERE created_at < datetime('now', '-' || days_old || ' days');
        DELETE FROM temp_files WHERE created_at < datetime('now', '-' || days_old || ' days');
        UPDATE stats SET last_cleanup = datetime('now');
    END;
)");
```

### CREATE PROCEDURE (C)

```c
const char* sql = 
    "CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL) "
    "BEGIN "
    "    UPDATE accounts SET balance = balance + amount WHERE id = user_id; "
    "    INSERT INTO transactions VALUES (user_id, amount, datetime('now')); "
    "END;";

dbx_execute_sql(db, sql);
```

### CALL PROCEDURE

```cpp
// C++
db.executeSql("CALL update_balance(123, 100.50);");
db.executeSql("CALL cleanup_old_data(30);");

// C
dbx_execute_sql(db, "CALL update_balance(123, 100.50);");
dbx_execute_sql(db, "CALL cleanup_old_data(30);");
```

### DROP PROCEDURE

```cpp
// C++
db.executeSql("DROP PROCEDURE update_balance;");

// C
dbx_execute_sql(db, "DROP PROCEDURE update_balance;");
```

---

## Scheduler

Cron 표현식 기반으로 주기적인 작업을 자동 실행합니다.

### CREATE SCHEDULE (C++)

```cpp
// 매 5분마다 통계 갱신
db.executeSql(R"(
    CREATE SCHEDULE refresh_stats
    CRON '*/5 * * * *'
    BEGIN
        UPDATE product_stats SET 
            total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id);
    END;
)");

// 매일 자정에 정리 작업
db.executeSql(R"(
    CREATE SCHEDULE daily_cleanup
    CRON '0 0 * * *'
    BEGIN
        CALL cleanup_old_data(30);
    END;
)");
```

### CREATE SCHEDULE (C)

```c
const char* sql = 
    "CREATE SCHEDULE daily_cleanup "
    "CRON '0 0 * * *' "
    "BEGIN "
    "    CALL cleanup_old_data(30); "
    "END;";

dbx_execute_sql(db, sql);
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

```cpp
// C++
db.executeSql("DROP SCHEDULE refresh_stats;");

// C
dbx_execute_sql(db, "DROP SCHEDULE refresh_stats;");
```

---

## UDF (사용자 정의 함수)

### CREATE FUNCTION (SQL)

```cpp
// C++: SQL 표준 문법으로 함수 정의
db.executeSql(R"(
    CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
    BEGIN
        RETURN a + b;
    END;
)");

// C
const char* sql = 
    "CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT "
    "BEGIN "
    "    RETURN a + b; "
    "END;";
dbx_execute_sql(db, sql);
```

> **참고**: 현재는 메타데이터 파싱만 지원하며, 실제 함수 로직은 C/C++ 코드로 등록해야 합니다.

### C++ 함수 등록

```cpp
// 스칼라 UDF 등록
db.registerScalarUdf("double", [](double x) { return x * 2; });

// SQL에서 사용
auto results = db.executeSql("SELECT double(price) FROM products");
```

### C 함수 등록

```c
// 향후 지원 예정
// C API에서는 SQL CREATE FUNCTION 사용 권장
```

---

## Event Hook (Rust 클로저 기반)

C/C++에서는 SQL Trigger를 사용하는 것을 권장합니다.

```cpp
// 향후 지원 예정
// C++ 람다 콜백 지원 계획
```

---

## 멀티스레딩 (C)

```c
#include <pthread.h>

typedef struct {
    char* db_path;
    int thread_id;
} ThreadData;

void* worker(void* arg) {
    ThreadData* data = (ThreadData*)arg;
    DbxDatabase* db = dbx_open(data->db_path);
    
    for (int i = 0; i < 1000; i++) {
        char key[64], value[64];
        snprintf(key, sizeof(key), "thread:%d:key:%d", data->thread_id, i);
        snprintf(value, sizeof(value), "value:%d", i);
        dbx_insert(db, "data", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
    }
    
    dbx_close(db);
    return NULL;
}

int main() {
    pthread_t threads[4];
    ThreadData data[4];
    
    for (int i = 0; i < 4; i++) {
        data[i].db_path = "mydb.db";
        data[i].thread_id = i;
        pthread_create(&threads[i], NULL, worker, &data[i]);
    }
    
    for (int i = 0; i < 4; i++) {
        pthread_join(threads[i], NULL);
    }
    
    return 0;
}
```

---

## 멀티스레딩 (C++)

```cpp
#include <thread>
#include <vector>

void worker(const std::string& dbPath, int threadId) {
    auto db = dbx::Database::open(dbPath);
    
    for (int i = 0; i < 1000; i++) {
        std::string key = "thread:" + std::to_string(threadId) + ":key:" + std::to_string(i);
        std::string value = "value:" + std::to_string(i);
        db.insert("data", key, value);
    }
}

int main() {
    std::vector<std::thread> threads;
    
    for (int i = 0; i < 4; i++) {
        threads.emplace_back(worker, "mydb.db", i);
    }
    
    for (auto& t : threads) {
        t.join();
    }
    
    return 0;
}
```

---

## 성능 튜닝

### 배치 작업 (C)

```c
DbxTransaction* tx = dbx_begin_transaction(db);
for (int i = 0; i < 10000; i++) {
    char key[32], value[64];
    snprintf(key, sizeof(key), "key:%d", i);
    snprintf(value, sizeof(value), "value:%d", i);
    dbx_insert(db, "data", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
}
dbx_commit(tx);
dbx_flush(db);
```

### 배치 작업 (C++)

```cpp
auto tx = db.beginTransaction();
for (int i = 0; i < 10000; i++) {
    db.insert("data", "key:" + std::to_string(i), "value:" + std::to_string(i));
}
tx.commit();
db.flush();
```

---

## 기능 플래그 (C++)

```cpp
// 런타임에 기능 활성화/비활성화
db.enableFeature("parallel_query");
db.enableFeature("query_plan_cache");
db.disableFeature("parallel_query");

if (db.isFeatureEnabled("parallel_query")) {
    std::cout << "병렬 쿼리 활성화됨" << std::endl;
}
```

---

## 기능 플래그 (C)

```c
dbx_enable_feature(db, "parallel_query");
dbx_enable_feature(db, "query_plan_cache");
dbx_disable_feature(db, "parallel_query");

if (dbx_is_feature_enabled(db, "parallel_query")) {
    printf("병렬 쿼리 활성화됨\n");
}
```

---

## 쿼리 플랜 캐시

```cpp
db.enableFeature("query_plan_cache");

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (int i = 0; i < 100; i++) {
    auto results = db.executeSql("SELECT * FROM users WHERE age > 20");
}
```

---

## 스키마 버저닝

```cpp
db.executeSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.executeSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

auto version = db.schemaVersion("users");  // → 2
```

---

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
