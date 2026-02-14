---
layout: default
title: C/C++
parent: Packages
grand_parent: English
nav_order: 5
---

# C/C++ Bindings

Native C and C++ interfaces for DBX via the FFI layer.

## Installation

Download `dbx-native-windows-x64.zip` from the [GitHub Releases](https://github.com/bytelogiccore-spec/DBX/releases) page.

Contents:
- `dbx_ffi.dll` — Dynamic library
- `dbx_ffi.lib` — Import library
- `dbx.h` — C header
- `dbx.hpp` — C++ header

## C Example

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

## C++ Example

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

## C API Reference

| Function | Description |
|----------|-------------|
| `dbx_open(path)` | Open file database |
| `dbx_open_in_memory()` | Open in-memory database |
| `dbx_insert(db, table, key, key_len, val, val_len)` | Insert |
| `dbx_get(db, table, key, key_len, &val, &val_len)` | Get |
| `dbx_delete(db, table, key, key_len)` | Delete |
| `dbx_free_value(val, val_len)` | Free returned value |
| `dbx_close(db)` | Close database |

## C++ Features

- ✅ RAII (automatic resource management)
- ✅ Modern C++17
- ✅ `std::optional` for nullable returns
- ✅ Move semantics
- ✅ Exception-based error handling
