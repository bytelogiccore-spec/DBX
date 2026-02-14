---
layout: default
title: 빠른 시작
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 2
---

# 빠른 시작

5분 안에 DBX를 시작해보세요!

## C 예제

```c
#include "dbx.h"
#include <stdio.h>
#include <string.h>

int main() {
    // 인메모리 데이터베이스
    DbxDatabase* db = dbx_open_in_memory();
    
    // KV 작업
    const char* key = "user:1";
    const char* value = "Alice";
    dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
    
    uint8_t* result = NULL;
    size_t result_len = 0;
    if (dbx_get(db, "users", (uint8_t*)key, strlen(key), &result, &result_len) == 0) {
        printf("Value: %.*s\n", (int)result_len, result);
        dbx_free_bytes(result);
    }
    
    // SQL 작업
    dbx_execute_sql(db, "CREATE TABLE users (id INTEGER, name TEXT)");
    dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice')");
    
    char* sql_result = dbx_execute_sql(db, "SELECT * FROM users");
    printf("SQL Result: %s\n", sql_result);
    dbx_free_string(sql_result);
    
    dbx_close(db);
    return 0;
}
```

## C++ 예제

```cpp
#include "dbx.hpp"
#include <iostream>

int main() {
    // RAII 래퍼
    auto db = dbx::Database::openInMemory();
    
    // KV 작업
    db.insert("users", "user:1", "Alice");
    
    auto value = db.get("users", "user:1");
    if (value) {
        std::cout << "Value: " << *value << std::endl;
    }
    
    // SQL 작업
    db.executeSql("CREATE TABLE users (id INTEGER, name TEXT)");
    db.executeSql("INSERT INTO users VALUES (1, 'Alice')");
    
    auto result = db.executeSql("SELECT * FROM users");
    std::cout << "SQL Result: " << result << std::endl;
    
    return 0;
}
```

## 컴파일

### GCC/MinGW

```bash
gcc -I./include -L./lib main.c -ldbx_ffi -o myapp.exe
```

### Visual Studio

프로젝트 속성에서 헤더 및 라이브러리 경로 설정 후 빌드

## 다음 단계

- [설치](installation) - 상세 설치 가이드
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
