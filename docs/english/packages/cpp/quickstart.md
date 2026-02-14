---
layout: default
title: Quick Start
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 2
---

# Quick Start

Get started with DBX in 5 minutes!

## C Example

```c
#include "dbx.h"
#include <stdio.h>
#include <string.h>

int main() {
    // Open in-memory database
    DbxDatabase* db = dbx_open_in_memory();
    
    // KV operations
    const char* key = "user:1";
    const char* value = "Alice";
    dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
    
    uint8_t* result = NULL;
    size_t result_len = 0;
    if (dbx_get(db, "users", (uint8_t*)key, strlen(key), &result, &result_len) == 0) {
        printf("Value: %.*s\n", (int)result_len, result);
        dbx_free_bytes(result);
    }
    
    // SQL operations
    dbx_execute_sql(db, "CREATE TABLE users (id INTEGER, name TEXT)");
    dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice')");
    
    char* sql_result = dbx_execute_sql(db, "SELECT * FROM users");
    printf("SQL Result: %s\n", sql_result);
    dbx_free_string(sql_result);
    
    dbx_close(db);
    return 0;
}
```

## C++ Example

```cpp
#include "dbx.hpp"
#include <iostream>

int main() {
    // RAII wrapper
    auto db = dbx::Database::openInMemory();
    
    // KV operations
    db.insert("users", "user:1", "Alice");
    
    auto value = db.get("users", "user:1");
    if (value) {
        std::cout << "Value: " << *value << std::endl;
    }
    
    // SQL operations
    db.executeSql("CREATE TABLE users (id INTEGER, name TEXT)");
    db.executeSql("INSERT INTO users VALUES (1, 'Alice')");
    
    auto result = db.executeSql("SELECT * FROM users");
    std::cout << "SQL Result: " << result << std::endl;
    
    return 0;
}
```

## Compile

### GCC/MinGW

```bash
gcc -I./include -L./lib main.c -ldbx_ffi -o myapp.exe
```

### Visual Studio

Set up project properties and build.

## Next Steps

- [Installation](installation) - Detailed setup guide
- [C API](c-api) - C function reference
- [C++ API](cpp-api) - C++ class reference
