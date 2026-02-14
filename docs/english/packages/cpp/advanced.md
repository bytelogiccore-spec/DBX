---
layout: default
title: Advanced
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 7
---

# Advanced Features

## Transactions (C)

```c
DbxTransaction* tx = dbx_begin_transaction(db);
dbx_insert(db, "users", (uint8_t*)"user:1", 6, (uint8_t*)"Alice", 5);
dbx_commit(tx);
```

## Transactions (C++)

```cpp
auto tx = db.beginTransaction();
db.insert("users", "user:1", "Alice");
tx.commit();
```

## Multithreading (C++)

```cpp
#include <thread>

void worker(const std::string& dbPath, int threadId) {
    auto db = dbx::Database::open(dbPath);
    for (int i = 0; i < 1000; i++) {
        db.insert("data", "key:" + std::to_string(i), "value");
    }
}

std::vector<std::thread> threads;
for (int i = 0; i < 4; i++) {
    threads.emplace_back(worker, "mydb.db", i);
}
for (auto& t : threads) {
    t.join();
}
```

## Performance Tuning

```c
DbxTransaction* tx = dbx_begin_transaction(db);
for (int i = 0; i < 10000; i++) {
    char key[32];
    snprintf(key, sizeof(key), "key:%d", i);
    dbx_insert(db, "data", (uint8_t*)key, strlen(key), (uint8_t*)"value", 5);
}
dbx_commit(tx);
dbx_flush(db);
```

## Next Steps

- [Examples](examples) - More examples
