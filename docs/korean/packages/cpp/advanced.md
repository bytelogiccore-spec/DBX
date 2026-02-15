---
layout: default
title: 고급 기능
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 7
---

# 고급 기능

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

## 기능 플래그 (C)

```c
dbx_enable_feature(db, "parallel_query");
dbx_enable_feature(db, "query_plan_cache");
dbx_disable_feature(db, "parallel_query");

if (dbx_is_feature_enabled(db, "parallel_query")) {
    printf("병렬 쿼리 활성화됨\n");
}
```

## 쿼리 플랜 캐시

```cpp
db.enableFeature("query_plan_cache");

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (int i = 0; i < 100; i++) {
    auto results = db.executeSql("SELECT * FROM users WHERE age > 20");
}
```

## 스키마 버저닝

```cpp
db.executeSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.executeSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

auto version = db.schemaVersion("users");  // → 2
```

## UDF (C++)

```cpp
// 스칼라 UDF 등록
db.registerScalarUdf("double", [](double x) { return x * 2; });

// SQL에서 사용
auto results = db.executeSql("SELECT double(price) FROM products");
```

## 트리거 (C++)

```cpp
db.registerTrigger("users", "after_insert", [](const auto& event) {
    std::cout << "새 사용자 추가됨" << std::endl;
});
```

## 스케줄러 (C++)

```cpp
db.scheduleJob("cleanup", "0 0 * * *", [&db]() {
    db.executeSql("DELETE FROM sessions WHERE expired = 1");
});
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
