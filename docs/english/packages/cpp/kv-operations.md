---
layout: default
title: KV Operations
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 6
---

# Key-Value Operations

High-performance KV operations for C/C++.

## Basic CRUD (C)

```c
// Insert
const char* key = "user:1";
const char* value = "Alice";
dbx_insert(db, "users", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));

// Get
uint8_t* result = NULL;
size_t result_len = 0;
dbx_get(db, "users", (uint8_t*)key, strlen(key), &result, &result_len);
if (result) {
    printf("%.*s\n", (int)result_len, result);
    dbx_free_bytes(result);
}

// Delete
dbx_delete(db, "users", (uint8_t*)key, strlen(key));
```

## Basic CRUD (C++)

```cpp
// Insert
db.insert("users", "user:1", "Alice");

// Get
auto value = db.get("users", "user:1");
if (value) {
    std::cout << *value << std::endl;
}

// Delete
db.remove("users", "user:1");
```

## Batch Operations

```c
for (int i = 0; i < 10000; i++) {
    char key[32], value[64];
    snprintf(key, sizeof(key), "key:%d", i);
    snprintf(value, sizeof(value), "value:%d", i);
    dbx_insert(db, "data", (uint8_t*)key, strlen(key), (uint8_t*)value, strlen(value));
}
dbx_flush(db);
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [Advanced](advanced) - Transactions, multithreading
