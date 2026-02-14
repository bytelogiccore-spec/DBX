---
layout: default
title: C++ API
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 4
---

# C++ API 레퍼런스

C++ 래퍼는 RAII 패턴과 예외 처리를 제공합니다.

## Database 클래스

### 생성자

#### `Database::open(const std::string& path)`

파일 기반 데이터베이스를 엽니다.

**매개변수:**
- `path`: 데이터베이스 파일 경로

**반환:** `Database` 객체

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
auto db = dbx::Database::open("mydb.db");
```

#### `Database::openInMemory()`

인메모리 데이터베이스를 엽니다.

**반환:** `Database` 객체

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
auto db = dbx::Database::openInMemory();
```

### Key-Value 메서드

#### `insert(const std::string& table, const std::string& key, const std::string& value)`

키-값 쌍을 삽입합니다.

**매개변수:**
- `table`: 테이블 이름
- `key`: 키
- `value`: 값

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
db.insert("users", "user:1", "Alice");
```

#### `get(const std::string& table, const std::string& key) -> std::optional<std::string>`

키로 값을 조회합니다.

**매개변수:**
- `table`: 테이블 이름
- `key`: 키

**반환:** `std::optional<std::string>` (값 또는 nullopt)

**예제:**
```cpp
auto value = db.get("users", "user:1");
if (value) {
    std::cout << *value << std::endl;
}
```

#### `remove(const std::string& table, const std::string& key)`

키를 삭제합니다.

**매개변수:**
- `table`: 테이블 이름
- `key`: 키

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
db.remove("users", "user:1");
```

#### `count(const std::string& table) -> size_t`

테이블의 행 개수를 반환합니다.

**매개변수:**
- `table`: 테이블 이름

**반환:** 행 개수

**예제:**
```cpp
size_t count = db.count("users");
std::cout << "Total: " << count << std::endl;
```

### SQL 메서드

#### `executeSql(const std::string& sql) -> std::string`

SQL 문을 실행합니다.

**매개변수:**
- `sql`: SQL 문

**반환:** 결과 (JSON 문자열)

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
db.executeSql("CREATE TABLE users (id INTEGER, name TEXT)");
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");
auto result = db.executeSql("SELECT * FROM users");
std::cout << result << std::endl;
```

### 트랜잭션 메서드

#### `beginTransaction() -> Transaction`

트랜잭션을 시작합니다.

**반환:** `Transaction` 객체

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
auto tx = db.beginTransaction();
try {
    db.insert("users", "user:1", "Alice");
    tx.commit();
} catch (...) {
    tx.rollback();
}
```

### 유틸리티 메서드

#### `flush()`

버퍼를 디스크에 플러시합니다.

**예제:**
```cpp
db.flush();
```

## Transaction 클래스

### 메서드

#### `commit()`

트랜잭션을 커밋합니다.

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
tx.commit();
```

#### `rollback()`

트랜잭션을 롤백합니다.

**예외:** `std::runtime_error` (실패 시)

**예제:**
```cpp
tx.rollback();
```

## RAII 패턴

C++ 래퍼는 자동 리소스 관리를 제공합니다:

```cpp
{
    auto db = dbx::Database::open("mydb.db");
    db.insert("users", "user:1", "Alice");
    // 자동으로 flush() 및 close() 호출
}
```

## 예외 처리

모든 에러는 `std::runtime_error` 예외로 전달됩니다:

```cpp
try {
    auto db = dbx::Database::open("mydb.db");
    db.insert("users", "user:1", "Alice");
} catch (const std::runtime_error& e) {
    std::cerr << "Error: " << e.what() << std::endl;
}
```

## 완전한 예제

```cpp
#include "dbx.hpp"
#include <iostream>
#include <exception>

int main() {
    try {
        // 데이터베이스 열기
        auto db = dbx::Database::open("example.db");
        
        // 트랜잭션 시작
        auto tx = db.beginTransaction();
        
        // KV 삽입
        db.insert("users", "user:1", "Alice");
        db.insert("users", "user:2", "Bob");
        
        // 커밋
        tx.commit();
        
        // 조회
        auto value = db.get("users", "user:1");
        if (value) {
            std::cout << "Value: " << *value << std::endl;
        }
        
        // SQL 실행
        db.executeSql("CREATE TABLE products (id INTEGER, name TEXT)");
        db.executeSql("INSERT INTO products VALUES (1, 'Laptop')");
        
        auto result = db.executeSql("SELECT * FROM products");
        std::cout << "SQL Result: " << result << std::endl;
        
        // 자동으로 flush() 및 close() 호출
        return 0;
        
    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
}
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [실전 예제](examples) - 더 많은 예제
