---
layout: default
title: C/C++ (dbx-ffi)
nav_order: 5
parent: 패키지
grand_parent: 한국어
has_children: true
---

# C/C++ — dbx-ffi

고성능 임베디드 데이터베이스 DBX의 공식 C/C++ FFI (Foreign Function Interface) 바인딩입니다.

## 주요 기능

- 🚀 **네이티브 성능**: Rust 코어 직접 호출
- 💾 **5-Tier 스토리지**: WOS → L0 → L1 → L2 → Cold Storage
- 🔒 **MVCC 트랜잭션**: 스냅샷 격리 지원
- 📊 **SQL 지원**: DDL + DML 완벽 지원
- 🔐 **암호화**: AES-GCM-SIV, ChaCha20-Poly1305
- 🔧 **C89 호환**: 모든 C/C++ 컴파일러 지원

## 빠른 시작

### C 예제

```c
#include "dbx.h"
#include <stdio.h>

int main() {
    // 데이터베이스 열기
    DbxDatabase* db = dbx_open_in_memory();
    
    // KV 작업
    const char* key = "user:1";
    const char* value = "Alice";
    dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
    
    // 조회
    uint8_t* result = NULL;
    size_t result_len = 0;
    dbx_get(db, "users", (uint8_t*)key, strlen(key), &result, &result_len);
    
    if (result) {
        printf("Value: %.*s\n", (int)result_len, result);
        dbx_free_bytes(result);
    }
    
    // SQL 작업
    dbx_execute_sql(db, "CREATE TABLE users (id INTEGER, name TEXT)");
    dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice')");
    
    char* sql_result = dbx_execute_sql(db, "SELECT * FROM users");
    printf("SQL Result: %s\n", sql_result);
    dbx_free_string(sql_result);
    
    // 정리
    dbx_close(db);
    return 0;
}
```

### C++ 예제

```cpp
#include "dbx.hpp"
#include <iostream>
#include <string>

int main() {
    // RAII 래퍼 사용
    dbx::Database db = dbx::Database::openInMemory();
    
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

## 문서 구조

- [설치](installation) - 헤더 및 라이브러리 설정
- [빠른 시작](quickstart) - 5분 안에 시작하기
- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
- [KV 작업](kv-operations) - Key-Value 작업 가이드
- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화, 멀티스레딩
- [빌드 가이드](build-guide) - CMake, Makefile, Visual Studio
- [실전 예제](examples) - 실무 활용 예제

## 버전 정보

- **현재 버전**: {{ site.dbx_version }}
- **C 표준**: C89 이상
- **C++ 표준**: C++11 이상 (C++ 래퍼)
- **플랫폼**: Windows x64 (Linux/macOS 계획됨)

## 라이선스

MIT License
