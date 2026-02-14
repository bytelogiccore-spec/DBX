/**
 * DBX vs SQLite Performance Benchmark (C++)
 * 
 * Compares DBX with SQLite using transactions for bulk operations.
 */

#include <iostream>
#include <chrono>
#include <vector>
#include <string>
#include <cstring>
#include <sqlite3.h>
#include "../include/dbx.h"

using namespace std;
using namespace chrono;

const int N = 10000;

// Helper function to format numbers with commas
string formatNumber(int num) {
    string s = to_string(num);
    int n = s.length() - 3;
    while (n > 0) {
        s.insert(n, ",");
        n -= 3;
    }
    return s;
}

// Benchmark DBX
void benchmarkDBX() {
    cout << "Benchmarking DBX (with FFI transaction)...\n\n";
    
    // Open in-memory database
    DbxHandle* db = dbx_open_in_memory();
    if (!db) {
        cerr << "Failed to open DBX database\n";
        return;
    }
    
    // INSERT with transaction
    auto startInsert = high_resolution_clock::now();
    
    DbxTransaction* tx = dbx_begin_transaction(db);
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        string value = "value:" + to_string(i);
        dbx_transaction_insert(tx, "bench", 
            (const uint8_t*)key.c_str(), key.size(),
            (const uint8_t*)value.c_str(), value.size());
    }
    dbx_transaction_commit(tx);
    
    auto endInsert = high_resolution_clock::now();
    double insertTime = duration<double>(endInsert - startInsert).count();
    
    // GET
    auto startGet = high_resolution_clock::now();
    
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        uint8_t* value = nullptr;
        size_t valueLen = 0;
        dbx_get(db, "bench", (const uint8_t*)key.c_str(), key.size(), &value, &valueLen);
        if (value) {
            dbx_free_value(value, valueLen);
        }
    }
    
    auto endGet = high_resolution_clock::now();
    double getTime = duration<double>(endGet - startGet).count();
    
    // DELETE with transaction
    auto startDelete = high_resolution_clock::now();
    
    DbxTransaction* tx2 = dbx_begin_transaction(db);
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        dbx_transaction_delete(tx2, "bench", (const uint8_t*)key.c_str(), key.size());
    }
    dbx_transaction_commit(tx2);
    
    auto endDelete = high_resolution_clock::now();
    double deleteTime = duration<double>(endDelete - startDelete).count();
    
    dbx_close(db);
    
    // Print results
    cout << "DBX (In-Memory, FFI Transaction):\n";
    cout << "  INSERT: " << insertTime << "s (" << formatNumber((int)(N / insertTime)) << " ops/sec)\n";
    cout << "  GET:    " << getTime << "s (" << formatNumber((int)(N / getTime)) << " ops/sec)\n";
    cout << "  DELETE: " << deleteTime << "s (" << formatNumber((int)(N / deleteTime)) << " ops/sec)\n\n";
}

// Benchmark SQLite
void benchmarkSQLite() {
    cout << "Benchmarking SQLite (In-Memory)...\n\n";
    
    sqlite3* db;
    if (sqlite3_open(":memory:", &db) != SQLITE_OK) {
        cerr << "Failed to open SQLite database\n";
        return;
    }
    
    // Create table
    sqlite3_exec(db, "CREATE TABLE bench (key TEXT PRIMARY KEY, value TEXT)", nullptr, nullptr, nullptr);
    
    // INSERT with transaction
    auto startInsert = high_resolution_clock::now();
    
    sqlite3_exec(db, "BEGIN TRANSACTION", nullptr, nullptr, nullptr);
    
    sqlite3_stmt* insertStmt;
    sqlite3_prepare_v2(db, "INSERT INTO bench (key, value) VALUES (?, ?)", -1, &insertStmt, nullptr);
    
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        string value = "value:" + to_string(i);
        sqlite3_bind_text(insertStmt, 1, key.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_bind_text(insertStmt, 2, value.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(insertStmt);
        sqlite3_reset(insertStmt);
    }
    
    sqlite3_finalize(insertStmt);
    sqlite3_exec(db, "COMMIT", nullptr, nullptr, nullptr);
    
    auto endInsert = high_resolution_clock::now();
    double insertTime = duration<double>(endInsert - startInsert).count();
    
    // GET
    auto startGet = high_resolution_clock::now();
    
    sqlite3_stmt* getStmt;
    sqlite3_prepare_v2(db, "SELECT value FROM bench WHERE key = ?", -1, &getStmt, nullptr);
    
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        sqlite3_bind_text(getStmt, 1, key.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(getStmt);
        sqlite3_reset(getStmt);
    }
    
    sqlite3_finalize(getStmt);
    
    auto endGet = high_resolution_clock::now();
    double getTime = duration<double>(endGet - startGet).count();
    
    // DELETE with transaction
    auto startDelete = high_resolution_clock::now();
    
    sqlite3_exec(db, "BEGIN TRANSACTION", nullptr, nullptr, nullptr);
    
    sqlite3_stmt* deleteStmt;
    sqlite3_prepare_v2(db, "DELETE FROM bench WHERE key = ?", -1, &deleteStmt, nullptr);
    
    for (int i = 0; i < N; i++) {
        string key = "key:" + to_string(i);
        sqlite3_bind_text(deleteStmt, 1, key.c_str(), -1, SQLITE_TRANSIENT);
        sqlite3_step(deleteStmt);
        sqlite3_reset(deleteStmt);
    }
    
    sqlite3_finalize(deleteStmt);
    sqlite3_exec(db, "COMMIT", nullptr, nullptr, nullptr);
    
    auto endDelete = high_resolution_clock::now();
    double deleteTime = duration<double>(endDelete - startDelete).count();
    
    sqlite3_close(db);
    
    // Print results
    cout << "SQLite (In-Memory):\n";
    cout << "  INSERT: " << insertTime << "s (" << formatNumber((int)(N / insertTime)) << " ops/sec)\n";
    cout << "  GET:    " << getTime << "s (" << formatNumber((int)(N / getTime)) << " ops/sec)\n";
    cout << "  DELETE: " << deleteTime << "s (" << formatNumber((int)(N / deleteTime)) << " ops/sec)\n\n";
}

int main() {
    cout << "============================================================\n";
    cout << "DBX vs SQLite - Performance Comparison (C++)\n";
    cout << "============================================================\n\n";
    cout << "Running benchmarks with " << formatNumber(N) << " operations...\n\n";
    
    benchmarkDBX();
    benchmarkSQLite();
    
    cout << "============================================================\n";
    cout << "Benchmark completed!\n";
    cout << "============================================================\n";
    
    return 0;
}
