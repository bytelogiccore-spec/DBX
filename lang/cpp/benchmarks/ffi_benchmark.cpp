/**
 * FFI Performance Benchmark for DBX C++ Bindings
 * 
 * Measures INSERT, GET, and SCAN performance to quantify FFI overhead
 * compared to Rust Core benchmarks.
 * 
 * Build: cmake --build build --target dbx_ffi_benchmark
 * Run: ./build/benchmarks/dbx_ffi_benchmark
 */

#include <benchmark/benchmark.h>
#include <string>
#include <vector>
#include "dbx.h"

constexpr int NUM_ENTRIES = 10'000;

struct TestData {
    std::vector<std::pair<std::string, std::string>> data;
    
    TestData() {
        data.reserve(NUM_ENTRIES);
        for (int i = 0; i < NUM_ENTRIES; i++) {
            char key[32], value[64];
            snprintf(key, sizeof(key), "key_%08d", i);
            snprintf(value, sizeof(value), "value_%08d_data", i);
            data.emplace_back(key, value);
        }
    }
};

static void BM_DBX_Insert(benchmark::State& state) {
    TestData test_data;
    
    for (auto _ : state) {
        auto db = dbx::Database::open_in_memory();
        for (const auto& [key, value] : test_data.data) {
            db.insert("bench", key, value);
        }
    }
}
BENCHMARK(BM_DBX_Insert);

static void BM_DBX_Get(benchmark::State& state) {
    TestData test_data;
    
    // Setup: Insert data first
    auto db = dbx::Database::open_in_memory();
    for (const auto& [key, value] : test_data.data) {
        db.insert("bench", key, value);
    }
    
    for (auto _ : state) {
        for (const auto& [key, _] : test_data.data) {
            db.get("bench", key);
        }
    }
}
BENCHMARK(BM_DBX_Get);

static void BM_DBX_Scan(benchmark::State& state) {
    TestData test_data;
    
    // Setup: Insert data first
    auto db = dbx::Database::open_in_memory();
    for (const auto& [key, value] : test_data.data) {
        db.insert("bench", key, value);
    }
    
    for (auto _ : state) {
        db.scan("bench");
    }
}
BENCHMARK(BM_DBX_Scan);

BENCHMARK_MAIN();
