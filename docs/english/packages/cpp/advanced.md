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

## Feature Flags (C++)

```cpp
db.enableFeature("parallel_query");
db.enableFeature("query_plan_cache");
db.disableFeature("parallel_query");

if (db.isFeatureEnabled("parallel_query")) {
    std::cout << "Parallel query enabled" << std::endl;
}
```

## Feature Flags (C)

```c
dbx_enable_feature(db, "parallel_query");
dbx_enable_feature(db, "query_plan_cache");
dbx_disable_feature(db, "parallel_query");

if (dbx_is_feature_enabled(db, "parallel_query")) {
    printf("Parallel query enabled\n");
}
```

## Query Plan Cache

```cpp
db.enableFeature("query_plan_cache");

// Repeated queries skip parsing (7.3x faster)
for (int i = 0; i < 100; i++) {
    auto results = db.executeSql("SELECT * FROM users WHERE age > 20");
}
```

## Schema Versioning

```cpp
db.executeSql("CREATE TABLE users (id INT, name TEXT)");       // v1
db.executeSql("ALTER TABLE users ADD COLUMN email TEXT");       // v2

auto version = db.schemaVersion("users");  // → 2
```

## UDF (C++)

```cpp
db.registerScalarUdf("double", [](double x) { return x * 2; });
auto results = db.executeSql("SELECT double(price) FROM products");
```

## Triggers (C++)

```cpp
db.registerTrigger("users", "after_insert", [](const auto& event) {
    std::cout << "New user added" << std::endl;
});
```

## Scheduler (C++)

```cpp
db.scheduleJob("cleanup", "0 0 * * *", [&db]() {
    db.executeSql("DELETE FROM sessions WHERE expired = 1");
});
```

## Next Steps

- [Examples](examples) - More examples
