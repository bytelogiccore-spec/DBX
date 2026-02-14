---
layout: default
title: Examples
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 8
---

# Real-World Examples

## Log Collector (C)

```c
#include "dbx.h"
#include <stdio.h>
#include <time.h>

typedef struct {
    DbxDatabase* db;
} LogCollector;

LogCollector* log_collector_create(const char* db_path) {
    LogCollector* collector = malloc(sizeof(LogCollector));
    collector->db = dbx_open(db_path);
    
    dbx_execute_sql(collector->db,
        "CREATE TABLE IF NOT EXISTS logs ("
        "  timestamp INTEGER,"
        "  level TEXT,"
        "  message TEXT"
        ")"
    );
    
    return collector;
}

void log_collector_add(LogCollector* collector, const char* level, const char* message) {
    char sql[1024];
    snprintf(sql, sizeof(sql),
        "INSERT INTO logs VALUES (%ld, '%s', '%s')",
        time(NULL), level, message);
    dbx_execute_sql(collector->db, sql);
}

void log_collector_destroy(LogCollector* collector) {
    dbx_close(collector->db);
    free(collector);
}
```

## Game Save System (C++)

```cpp
#include "dbx.hpp"
#include <nlohmann/json.hpp>

class GameSaveSystem {
private:
    dbx::Database db;

public:
    GameSaveSystem(const std::string& dbPath) : db(dbx::Database::open(dbPath)) {
        db.executeSql(R"(
            CREATE TABLE IF NOT EXISTS saves (
                slot INTEGER PRIMARY KEY,
                player_name TEXT,
                level INTEGER,
                score INTEGER,
                data TEXT
            )
        )");
    }

    void saveGame(int slot, const std::string& playerName, int level, int score, const json& gameData) {
        std::ostringstream sql;
        sql << "INSERT OR REPLACE INTO saves VALUES ("
            << slot << ", '" << playerName << "', " << level << ", " 
            << score << ", '" << gameData.dump() << "')";
        db.executeSql(sql.str());
    }

    std::optional<json> loadGame(int slot) {
        auto result = db.executeSql("SELECT data FROM saves WHERE slot = " + std::to_string(slot));
        if (result.empty()) return std::nullopt;
        return json::parse(result);
    }
};
```

## Embedded System (C)

```c
typedef struct {
    int sensor_id;
    float temperature;
    float humidity;
    long timestamp;
} SensorData;

void store_sensor_data(DbxDatabase* db, const SensorData* data) {
    char sql[256];
    snprintf(sql, sizeof(sql),
        "INSERT INTO sensor_data VALUES (%d, %.2f, %.2f, %ld)",
        data->sensor_id, data->temperature, data->humidity, data->timestamp);
    dbx_execute_sql(db, sql);
}
```

## Next Steps

- [C API](c-api) - C function reference
- [C++ API](cpp-api) - C++ class reference
