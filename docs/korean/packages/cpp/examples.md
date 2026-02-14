---
layout: default
title: 실전 예제
parent: C/C++ (dbx-ffi)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 8
---

# 실전 예제

## HTTP 서버 (C++ with httplib)

{% raw %}
```cpp
#include "dbx.hpp"
#include "httplib.h"
#include <nlohmann/json.hpp>

using json = nlohmann::json;

int main() {
    auto db = dbx::Database::open("api.db");
    db.executeSql("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT, email TEXT)");
    
    httplib::Server svr;
    
    svr.Post("/users", [&](const httplib::Request& req, httplib::Response& res) {
        auto body = json::parse(req.body);
        int id = std::time(nullptr);
        
        std::ostringstream sql;
        sql << "INSERT INTO users VALUES (" << id << ", '" 
            << body["name"].get<std::string>() << "', '" 
            << body["email"].get<std::string>() << "')";
        
        db.executeSql(sql.str());
        
        json response = {{"id", id}, {"name", body["name"]}, {"email", body["email"]}};
        res.set_content(response.dump(), "application/json");
    });
    
    svr.Get(R"(/users/(\d+))", [&](const httplib::Request& req, httplib::Response& res) {
        int id = std::stoi(req.matches[1]);
        auto result = db.executeSql("SELECT * FROM users WHERE id = " + std::to_string(id));
        res.set_content(result, "application/json");
    });
    
    svr.listen("0.0.0.0", 8080);
    return 0;
}
```
{% endraw %}

## 로그 수집기 (C)

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
    time_t now = time(NULL);
    
    snprintf(sql, sizeof(sql),
        "INSERT INTO logs VALUES (%ld, '%s', '%s')",
        now, level, message);
    
    dbx_execute_sql(collector->db, sql);
}

char* log_collector_get_errors(LogCollector* collector) {
    return dbx_execute_sql(collector->db, "SELECT * FROM logs WHERE level = 'ERROR'");
}

void log_collector_destroy(LogCollector* collector) {
    dbx_close(collector->db);
    free(collector);
}

int main() {
    LogCollector* collector = log_collector_create("logs.db");
    
    log_collector_add(collector, "INFO", "Application started");
    log_collector_add(collector, "ERROR", "Connection failed");
    
    char* errors = log_collector_get_errors(collector);
    printf("Errors: %s\n", errors);
    dbx_free_string(errors);
    
    log_collector_destroy(collector);
    return 0;
}
```

## 임베디드 시스템 (C)

```c
#include "dbx.h"
#include <stdio.h>

// 센서 데이터 저장
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

int main() {
    DbxDatabase* db = dbx_open("sensors.db");
    
    dbx_execute_sql(db,
        "CREATE TABLE IF NOT EXISTS sensor_data ("
        "  sensor_id INTEGER,"
        "  temperature REAL,"
        "  humidity REAL,"
        "  timestamp INTEGER"
        ")"
    );
    
    // 센서 데이터 수집
    for (int i = 0; i < 100; i++) {
        SensorData data = {
            .sensor_id = 1,
            .temperature = 20.0 + (i % 10),
            .humidity = 50.0 + (i % 20),
            .timestamp = time(NULL)
        };
        
        store_sensor_data(db, &data);
    }
    
    // 통계 조회
    char* stats = dbx_execute_sql(db,
        "SELECT AVG(temperature), AVG(humidity) FROM sensor_data");
    printf("Stats: %s\n", stats);
    dbx_free_string(stats);
    
    dbx_close(db);
    return 0;
}
```

## 게임 저장 시스템 (C++)

{% raw %}
```cpp
#include "dbx.hpp"
#include <nlohmann/json.hpp>

using json = nlohmann::json;

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
                data TEXT,
                timestamp INTEGER
            )
        )");
    }

    void saveGame(int slot, const std::string& playerName, int level, int score, const json& gameData) {
        auto now = std::time(nullptr);
        
        std::ostringstream sql;
        sql << "INSERT OR REPLACE INTO saves VALUES ("
            << slot << ", '" << playerName << "', " << level << ", " 
            << score << ", '" << gameData.dump() << "', " << now << ")";
        
        db.executeSql(sql.str());
    }

    std::optional<json> loadGame(int slot) {
        auto result = db.executeSql("SELECT data FROM saves WHERE slot = " + std::to_string(slot));
        if (result.empty()) return std::nullopt;
        
        return json::parse(result);
    }

    void deleteGame(int slot) {
        db.executeSql("DELETE FROM saves WHERE slot = " + std::to_string(slot));
    }
};

int main() {
    GameSaveSystem saves("game.db");
    
    json gameData = {
        {"position", {{"x", 100}, {"y", 200}}},
        {"inventory", {"sword", "shield", "potion"}},
        {"quests", {{"main", "completed"}, {"side", "in_progress"}}}
    };
    
    saves.saveGame(1, "Player1", 10, 5000, gameData);
    
    auto loaded = saves.loadGame(1);
    if (loaded) {
        std::cout << "Loaded game: " << loaded->dump(2) << std::endl;
    }
    
    return 0;
}
```
{% endraw %}

## 다음 단계

- [C API](c-api) - C 함수 레퍼런스
- [C++ API](cpp-api) - C++ 클래스 레퍼런스
- [고급 기능](advanced) - 트랜잭션, 멀티스레딩
