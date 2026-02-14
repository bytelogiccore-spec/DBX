---
layout: default
title: SQL 가이드
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 4
---

# SQL 가이드

DBX는 표준 SQL을 지원합니다. C/C++에서 `dbx_execute_sql` 함수로 사용할 수 있습니다.

## 테이블 생성 (CREATE TABLE)

### C 예제

```c
#include "dbx.h"
#include <stdio.h>

int main() {
    DbxDatabase* db = dbx_open("mydb.db");
    
    // 기본 테이블
    dbx_execute_sql(db, 
        "CREATE TABLE users ("
        "  id INTEGER,"
        "  name TEXT,"
        "  email TEXT,"
        "  age INTEGER"
        ")"
    );
    
    // Primary Key 지정
    dbx_execute_sql(db,
        "CREATE TABLE products ("
        "  id INTEGER PRIMARY KEY,"
        "  name TEXT NOT NULL,"
        "  price REAL"
        ")"
    );
    
    dbx_close(db);
    return 0;
}
```

### C++ 예제

```cpp
#include "dbx.hpp"
#include <iostream>

int main() {
    auto db = dbx::Database::open("mydb.db");
    
    // 기본 테이블
    db.executeSql(R"(
        CREATE TABLE users (
            id INTEGER,
            name TEXT,
            email TEXT,
            age INTEGER
        )
    )");
    
    // Primary Key 지정
    db.executeSql(R"(
        CREATE TABLE products (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            price REAL
        )
    )");
    
    return 0;
}
```

## 데이터 삽입 (INSERT)

### C 예제

```c
// 기본 INSERT
dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// 컬럼 명시
dbx_execute_sql(db,
    "INSERT INTO users (id, name, email) "
    "VALUES (2, 'Bob', 'bob@example.com')"
);

// 다중 행 삽입
for (int i = 0; i < 3; i++) {
    char sql[256];
    snprintf(sql, sizeof(sql),
        "INSERT INTO users VALUES (%d, 'User%d', 'user%d@example.com', %d)",
        i, i, i, 20 + i);
    dbx_execute_sql(db, sql);
}
```

### C++ 예제

```cpp
#include <sstream>

// 기본 INSERT
db.executeSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// std::string 사용
std::string sql = "INSERT INTO users (id, name, email) "
                  "VALUES (2, 'Bob', 'bob@example.com')";
db.executeSql(sql);

// 다중 행 삽입
for (int i = 0; i < 3; i++) {
    std::ostringstream oss;
    oss << "INSERT INTO users VALUES (" << i << ", 'User" << i 
        << "', 'user" << i << "@example.com', " << (20 + i) << ")";
    db.executeSql(oss.str());
}
```

## 데이터 조회 (SELECT)

### C 예제

```c
// 전체 조회
char* result = dbx_execute_sql(db, "SELECT * FROM users");
if (result) {
    printf("Result: %s\n", result);
    dbx_free_string(result);
}

// WHERE 조건
result = dbx_execute_sql(db, "SELECT * FROM users WHERE age >= 18");
if (result) {
    printf("Adults: %s\n", result);
    dbx_free_string(result);
}

// ORDER BY
result = dbx_execute_sql(db, "SELECT * FROM users ORDER BY age DESC");
if (result) {
    printf("Sorted: %s\n", result);
    dbx_free_string(result);
}

// LIMIT
result = dbx_execute_sql(db, "SELECT * FROM users LIMIT 10");
if (result) {
    printf("Top 10: %s\n", result);
    dbx_free_string(result);
}
```

### C++ 예제

```cpp
// 전체 조회
auto result = db.executeSql("SELECT * FROM users");
std::cout << "Result: " << result << std::endl;

// WHERE 조건
result = db.executeSql("SELECT * FROM users WHERE age >= 18");
std::cout << "Adults: " << result << std::endl;

// ORDER BY
result = db.executeSql("SELECT * FROM users ORDER BY age DESC");

// LIMIT
result = db.executeSql("SELECT * FROM users LIMIT 10");
```

## 데이터 수정 (UPDATE)

### C 예제

```c
// 단일 컬럼 수정
dbx_execute_sql(db, "UPDATE users SET age = 26 WHERE id = 1");

// 다중 컬럼 수정
dbx_execute_sql(db,
    "UPDATE users "
    "SET name = 'Alice Smith', email = 'alice.smith@example.com' "
    "WHERE id = 1"
);

// 조건부 수정
dbx_execute_sql(db, "UPDATE users SET age = age + 1 WHERE age < 30");
```

### C++ 예제

```cpp
// 단일 컬럼 수정
db.executeSql("UPDATE users SET age = 26 WHERE id = 1");

// 다중 컬럼 수정
db.executeSql(R"(
    UPDATE users 
    SET name = 'Alice Smith', email = 'alice.smith@example.com'
    WHERE id = 1
)");

// 조건부 수정
db.executeSql("UPDATE users SET age = age + 1 WHERE age < 30");
```

## 데이터 삭제 (DELETE)

### C 예제

```c
// 특정 행 삭제
dbx_execute_sql(db, "DELETE FROM users WHERE id = 1");

// 조건부 삭제
dbx_execute_sql(db, "DELETE FROM users WHERE age < 18");

// 전체 삭제 (주의!)
dbx_execute_sql(db, "DELETE FROM users");
```

### C++ 예제

```cpp
// 특정 행 삭제
db.executeSql("DELETE FROM users WHERE id = 1");

// 조건부 삭제
db.executeSql("DELETE FROM users WHERE age < 18");
```

## 트랜잭션과 함께 사용

### C 예제

```c
DbxTransaction* tx = dbx_begin_transaction(db);
if (!tx) {
    fprintf(stderr, "Failed to begin transaction\n");
    return 1;
}

// SQL 실행
if (dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)") == NULL) {
    dbx_rollback(tx);
    fprintf(stderr, "Insert failed\n");
    return 1;
}

if (dbx_execute_sql(db, "INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)") == NULL) {
    dbx_rollback(tx);
    fprintf(stderr, "Insert failed\n");
    return 1;
}

// 커밋
if (dbx_commit(tx) != 0) {
    fprintf(stderr, "Commit failed\n");
    return 1;
}
```

### C++ 예제

```cpp
auto tx = db.beginTransaction();

try {
    db.executeSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");
    db.executeSql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)");
    
    tx.commit();
} catch (const std::exception& e) {
    tx.rollback();
    std::cerr << "Transaction failed: " << e.what() << std::endl;
}
```

## 실전 예제

### 사용자 관리 시스템 (C)

```c
#include "dbx.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

typedef struct {
    DbxDatabase* db;
} UserManager;

UserManager* user_manager_create(const char* db_path) {
    UserManager* mgr = malloc(sizeof(UserManager));
    mgr->db = dbx_open(db_path);
    
    // 스키마 초기화
    dbx_execute_sql(mgr->db,
        "CREATE TABLE IF NOT EXISTS users ("
        "  id INTEGER PRIMARY KEY,"
        "  username TEXT NOT NULL,"
        "  email TEXT NOT NULL,"
        "  created_at INTEGER"
        ")"
    );
    
    return mgr;
}

int user_manager_create_user(UserManager* mgr, const char* username, const char* email) {
    char sql[512];
    int user_id = (int)time(NULL);
    
    snprintf(sql, sizeof(sql),
        "INSERT INTO users (id, username, email, created_at) "
        "VALUES (%d, '%s', '%s', %ld)",
        user_id, username, email, time(NULL));
    
    char* result = dbx_execute_sql(mgr->db, sql);
    if (result) {
        dbx_free_string(result);
        return user_id;
    }
    return -1;
}

char* user_manager_get_user(UserManager* mgr, int user_id) {
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM users WHERE id = %d", user_id);
    return dbx_execute_sql(mgr->db, sql);
}

void user_manager_destroy(UserManager* mgr) {
    dbx_close(mgr->db);
    free(mgr);
}

// 사용 예제
int main() {
    UserManager* mgr = user_manager_create("users.db");
    
    int user_id = user_manager_create_user(mgr, "alice", "alice@example.com");
    printf("Created user: %d\n", user_id);
    
    char* user = user_manager_get_user(mgr, user_id);
    if (user) {
        printf("User: %s\n", user);
        dbx_free_string(user);
    }
    
    user_manager_destroy(mgr);
    return 0;
}
```

### 사용자 관리 시스템 (C++)

```cpp
#include "dbx.hpp"
#include <iostream>
#include <sstream>
#include <chrono>

class UserManager {
private:
    dbx::Database db;

public:
    UserManager(const std::string& dbPath) : db(dbx::Database::open(dbPath)) {
        // 스키마 초기화
        db.executeSql(R"(
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                created_at INTEGER
            )
        )");
    }

    int createUser(const std::string& username, const std::string& email) {
        auto now = std::chrono::system_clock::now().time_since_epoch().count();
        int userId = static_cast<int>(now);
        
        std::ostringstream sql;
        sql << "INSERT INTO users (id, username, email, created_at) "
            << "VALUES (" << userId << ", '" << username << "', '" 
            << email << "', " << now << ")";
        
        db.executeSql(sql.str());
        return userId;
    }

    std::string getUser(int userId) {
        std::ostringstream sql;
        sql << "SELECT * FROM users WHERE id = " << userId;
        return db.executeSql(sql.str());
    }

    void updateEmail(int userId, const std::string& newEmail) {
        std::ostringstream sql;
        sql << "UPDATE users SET email = '" << newEmail 
            << "' WHERE id = " << userId;
        db.executeSql(sql.str());
    }

    void deleteUser(int userId) {
        std::ostringstream sql;
        sql << "DELETE FROM users WHERE id = " << userId;
        db.executeSql(sql.str());
    }

    std::string listUsers(int limit = 100) {
        std::ostringstream sql;
        sql << "SELECT * FROM users LIMIT " << limit;
        return db.executeSql(sql.str());
    }
};

// 사용 예제
int main() {
    UserManager mgr("users.db");
    
    int userId = mgr.createUser("alice", "alice@example.com");
    std::cout << "Created user: " << userId << std::endl;
    
    auto user = mgr.getUser(userId);
    std::cout << "User: " << user << std::endl;
    
    mgr.updateEmail(userId, "alice.new@example.com");
    
    auto users = mgr.listUsers();
    std::cout << "All users: " << users << std::endl;
    
    return 0;
}
```

## 성능 팁

### 1. 배치 작업 (C)

```c
// ❌ 느림
for (int i = 0; i < 1000; i++) {
    char sql[256];
    snprintf(sql, sizeof(sql), "INSERT INTO users VALUES (%d, 'User%d', 'user%d@example.com', 25)", i, i, i);
    dbx_execute_sql(db, sql);
}

// ✅ 빠름 (트랜잭션 사용)
DbxTransaction* tx = dbx_begin_transaction(db);
for (int i = 0; i < 1000; i++) {
    char sql[256];
    snprintf(sql, sizeof(sql), "INSERT INTO users VALUES (%d, 'User%d', 'user%d@example.com', 25)", i, i, i);
    dbx_execute_sql(db, sql);
}
dbx_commit(tx);
```

### 2. SQL Injection 방지

```c
// 입력값 검증 함수
void sanitize_string(char* output, const char* input, size_t max_len) {
    size_t j = 0;
    for (size_t i = 0; input[i] && j < max_len - 1; i++) {
        if (input[i] == '\'') {
            output[j++] = '\'';  // 작은따옴표 이스케이프
        }
        output[j++] = input[i];
    }
    output[j] = '\0';
}

// 사용
char safe_name[256];
sanitize_string(safe_name, user_input, sizeof(safe_name));

char sql[512];
snprintf(sql, sizeof(sql), "INSERT INTO users (name) VALUES ('%s')", safe_name);
dbx_execute_sql(db, sql);
```

## 제한사항

- **JOIN**: 현재 미지원 (향후 지원 예정)
- **서브쿼리**: 제한적 지원
- **외래 키**: 현재 미지원

## 다음 단계

- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
- [실전 예제](examples) - 더 많은 예제
