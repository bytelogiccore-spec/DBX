---
layout: default
title: KV 작업
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# Key-Value 작업

DBX는 SQL 외에도 고성능 Key-Value 스토어로 사용할 수 있습니다.

## 기본 CRUD (C)

### 삽입

```c
#include "dbx.h"
#include <string.h>

DbxDatabase* db = dbx_open_in_memory();

// 기본 삽입
const char* key = "user:1";
const char* value = "Alice";
dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));

// JSON 데이터
const char* json = "{\"id\":1,\"name\":\"Alice\"}";
dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)json, strlen(json));
```

### 조회

```c
uint8_t* result = NULL;
size_t result_len = 0;

if (dbx_get(db, "users", (uint8_t*)key, strlen(key), &result, &result_len) == 0) {
    printf("Value: %.*s\n", (int)result_len, result);
    dbx_free_bytes(result);
}
```

### 삭제

```c
dbx_delete(db, "users", (uint8_t*)key, strlen(key));
```

### 개수 확인

```c
size_t count = dbx_count(db, "users");
printf("Total users: %zu\n", count);
```

## 기본 CRUD (C++)

### 삽입

```cpp
#include "dbx.hpp"

auto db = dbx::Database::openInMemory();

// 기본 삽입
db.insert("users", "user:1", "Alice");

// JSON 데이터
db.insert("users", "user:1", R"({"id":1,"name":"Alice"})");
```

### 조회

```cpp
auto value = db.get("users", "user:1");
if (value) {
    std::cout << "Value: " << *value << std::endl;
}
```

### 삭제

```cpp
db.remove("users", "user:1");
```

### 개수 확인

```cpp
size_t count = db.count("users");
std::cout << "Total users: " << count << std::endl;
```

## 배치 작업 (C)

```c
// 대량 삽입
for (int i = 0; i < 10000; i++) {
    char key[32];
    char value[64];
    snprintf(key, sizeof(key), "user:%d", i);
    snprintf(value, sizeof(value), "User %d", i);
    
    dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
}

// 플러시
dbx_flush(db);
```

## 배치 작업 (C++)

```cpp
// 대량 삽입
for (int i = 0; i < 10000; i++) {
    std::string key = "user:" + std::to_string(i);
    std::string value = "User " + std::to_string(i);
    db.insert("users", key, value);
}

// 플러시
db.flush();
```

## 실전 예제

### 세션 저장소 (C++)

```cpp
#include "dbx.hpp"
#include <chrono>
#include <nlohmann/json.hpp>

using json = nlohmann::json;

class SessionStore {
private:
    dbx::Database db;

public:
    SessionStore(const std::string& dbPath) : db(dbx::Database::open(dbPath)) {}

    void createSession(const std::string& sessionId, const json& data, int ttlSeconds = 3600) {
        auto now = std::chrono::system_clock::now().time_since_epoch().count();
        
        json session = {
            {"data", data},
            {"createdAt", now},
            {"expiresAt", now + (ttlSeconds * 1000000000LL)}
        };
        
        db.insert("sessions", sessionId, session.dump());
    }

    std::optional<json> getSession(const std::string& sessionId) {
        auto value = db.get("sessions", sessionId);
        if (!value) return std::nullopt;

        auto session = json::parse(*value);
        auto now = std::chrono::system_clock::now().time_since_epoch().count();

        // 만료 확인
        if (now > session["expiresAt"].get<long long>()) {
            db.remove("sessions", sessionId);
            return std::nullopt;
        }

        return session["data"];
    }

    void deleteSession(const std::string& sessionId) {
        db.remove("sessions", sessionId);
    }
};

// 사용 예제
int main() {
    SessionStore store("sessions.db");

    json userData = {
        {"userId", 42},
        {"username", "alice"},
        {"role", "admin"}
    };

    store.createSession("sess_abc123", userData, 3600);

    auto session = store.getSession("sess_abc123");
    if (session) {
        std::cout << "User: " << (*session)["username"] << std::endl;
    }

    store.deleteSession("sess_abc123");
    return 0;
}
```

### 캐시 시스템 (C++)

```cpp
template<typename T>
class Cache {
private:
    dbx::Database db;
    int defaultTtl;

public:
    Cache(const std::string& dbPath, int defaultTtlSeconds = 300)
        : db(dbx::Database::open(dbPath)), defaultTtl(defaultTtlSeconds) {}

    void set(const std::string& key, const T& value, int ttlSeconds = -1) {
        int ttl = (ttlSeconds > 0) ? ttlSeconds : defaultTtl;
        auto now = std::chrono::system_clock::now().time_since_epoch().count();

        json cacheData = {
            {"value", value},
            {"expiresAt", now + (ttl * 1000000000LL)}
        };

        db.insert("cache", key, cacheData.dump());
    }

    std::optional<T> get(const std::string& key) {
        auto data = db.get("cache", key);
        if (!data) return std::nullopt;

        auto cacheData = json::parse(*data);
        auto now = std::chrono::system_clock::now().time_since_epoch().count();

        // 만료 확인
        if (now > cacheData["expiresAt"].get<long long>()) {
            db.remove("cache", key);
            return std::nullopt;
        }

        return cacheData["value"].get<T>();
    }

    void remove(const std::string& key) {
        db.remove("cache", key);
    }
};

// 사용 예제
struct User {
    std::string name;
    std::string email;
};

void to_json(json& j, const User& u) {
    j = json{{"name", u.name}, {"email", u.email}};
}

void from_json(const json& j, User& u) {
    j.at("name").get_to(u.name);
    j.at("email").get_to(u.email);
}

int main() {
    Cache<User> cache("cache.db", 300);

    User user{"Alice", "alice@example.com"};
    cache.set("user:1", user);

    auto cached = cache.get("user:1");
    if (cached) {
        std::cout << "Cached user: " << cached->name << std::endl;
    }

    return 0;
}
```

## 성능 최적화

### 1. 배치 작업 + 트랜잭션 (C)

```c
// ❌ 느림
for (int i = 0; i < 10000; i++) {
    char key[32], value[64];
    snprintf(key, sizeof(key), "key:%d", i);
    snprintf(value, sizeof(value), "value:%d", i);
    dbx_insert(db, "data", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
    dbx_flush(db);
}

// ✅ 빠름
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

### 2. 스마트 포인터 사용 (C++)

```cpp
// ✅ RAII 패턴
{
    auto db = dbx::Database::open("data.db");
    for (int i = 0; i < 10000; i++) {
        db.insert("data", "key:" + std::to_string(i), "value:" + std::to_string(i));
    }
}  // 자동으로 flush() 및 close()
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
