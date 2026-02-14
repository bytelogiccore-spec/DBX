---
layout: default
title: C/C++
parent: 패키지
grand_parent: 한국어
nav_order: 5
---

# C/C++ 바인딩

FFI 레이어를 통한 네이티브 C/C++ 인터페이스입니다.

## 설치

[GitHub Releases](https://github.com/bytelogiccore-spec/DBX/releases)에서 `dbx-native-windows-x64.zip` 다운로드.

포함 내용:
- `dbx_ffi.dll` — 동적 라이브러리
- `dbx_ffi.lib` — 임포트 라이브러리
- `dbx.h` — C 헤더
- `dbx.hpp` — C++ 헤더

## C 예제

```c
#include <stdio.h>
#include "dbx.h"

int main() {
    DbxHandle* db = dbx_open("my_database.db");

    dbx_insert(db, "users",
               (uint8_t*)"user:1", 6,
               (uint8_t*)"Alice", 5);

    uint8_t* value = NULL;
    size_t value_len = 0;
    dbx_get(db, "users", (uint8_t*)"user:1", 6, &value, &value_len);

    printf("%.*s\n", (int)value_len, value);
    dbx_free_value(value, value_len);

    dbx_close(db);
    return 0;
}
```

## C++ 예제

```cpp
#include <iostream>
#include "dbx.hpp"

using namespace dbx;

int main() {
    try {
        auto db = Database::openInMemory();

        db.insert("users", "user:1", "Alice");

        if (auto value = db.getString("users", "user:1")) {
            std::cout << *value << std::endl;
        }
    } catch (const DatabaseError& e) {
        std::cerr << "Error: " << e.what() << std::endl;
    }

    return 0;
}
```

## C API 레퍼런스

| 함수 | 설명 |
|------|------|
| `dbx_open(path)` | 파일 DB 열기 |
| `dbx_open_in_memory()` | 인메모리 DB 열기 |
| `dbx_insert(db, table, key, key_len, val, val_len)` | 삽입 |
| `dbx_get(db, table, key, key_len, &val, &val_len)` | 조회 |
| `dbx_delete(db, table, key, key_len)` | 삭제 |
| `dbx_free_value(val, val_len)` | 반환값 메모리 해제 |
| `dbx_close(db)` | 데이터베이스 닫기 |

## C++ 특징

- ✅ RAII (자동 리소스 관리)
- ✅ Modern C++17
- ✅ `std::optional` nullable 반환
- ✅ 이동 시맨틱
- ✅ 예외 기반 에러 처리
