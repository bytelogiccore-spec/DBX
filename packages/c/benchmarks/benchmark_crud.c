/**
 * DBX C Bindings - Performance Benchmark
 * 
 * Measures the performance of basic CRUD operations.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "dbx.h"

#ifdef _WIN32
#include <windows.h>
double get_time() {
    LARGE_INTEGER frequency, counter;
    QueryPerformanceFrequency(&frequency);
    QueryPerformanceCounter(&counter);
    return (double)counter.QuadPart / (double)frequency.QuadPart;
}
#else
double get_time() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec + ts.tv_nsec / 1e9;
}
#endif

typedef struct {
    double elapsed;
    double ops_per_sec;
} BenchmarkResult;

BenchmarkResult benchmark_insert(DbxHandle* db, int n) {
    char key[32], value[32];
    double start = get_time();
    
    for (int i = 0; i < n; i++) {
        snprintf(key, sizeof(key), "key:%d", i);
        snprintf(value, sizeof(value), "value:%d", i);
        dbx_insert(db, "bench", 
                  (const uint8_t*)key, strlen(key),
                  (const uint8_t*)value, strlen(value));
    }
    
    double end = get_time();
    BenchmarkResult result;
    result.elapsed = end - start;
    result.ops_per_sec = n / result.elapsed;
    return result;
}

BenchmarkResult benchmark_get(DbxHandle* db, int n) {
    char key[32];
    uint8_t* value = NULL;
    size_t value_len = 0;
    double start = get_time();
    
    for (int i = 0; i < n; i++) {
        snprintf(key, sizeof(key), "key:%d", i);
        int result = dbx_get(db, "bench",
                            (const uint8_t*)key, strlen(key),
                            &value, &value_len);
        if (result == DBX_OK && value != NULL) {
            dbx_free_value(value, value_len);
            value = NULL;
        }
    }
    
    double end = get_time();
    BenchmarkResult result;
    result.elapsed = end - start;
    result.ops_per_sec = n / result.elapsed;
    return result;
}

BenchmarkResult benchmark_delete(DbxHandle* db, int n) {
    char key[32];
    double start = get_time();
    
    for (int i = 0; i < n; i++) {
        snprintf(key, sizeof(key), "key:%d", i);
        dbx_delete(db, "bench", (const uint8_t*)key, strlen(key));
    }
    
    double end = get_time();
    BenchmarkResult result;
    result.elapsed = end - start;
    result.ops_per_sec = n / result.elapsed;
    return result;
}

int main() {
    printf("============================================================\n");
    printf("DBX C Bindings - Performance Benchmark\n");
    printf("============================================================\n");
    
    DbxHandle* db = dbx_open_in_memory();
    if (db == NULL) {
        fprintf(stderr, "Failed to open database\n");
        return 1;
    }
    
    int n = 10000;
    printf("\nRunning benchmarks with %d operations...\n\n", n);
    
    // Benchmark INSERT
    printf("Benchmarking INSERT...\n");
    BenchmarkResult result = benchmark_insert(db, n);
    printf("  Time: %.4fs\n", result.elapsed);
    printf("  Throughput: %.0f ops/sec\n", result.ops_per_sec);
    printf("  Latency: %.4f ms/op\n", (result.elapsed/n)*1000);
    
    // Benchmark GET
    printf("\nBenchmarking GET...\n");
    result = benchmark_get(db, n);
    printf("  Time: %.4fs\n", result.elapsed);
    printf("  Throughput: %.0f ops/sec\n", result.ops_per_sec);
    printf("  Latency: %.4f ms/op\n", (result.elapsed/n)*1000);
    
    // Benchmark DELETE
    printf("\nBenchmarking DELETE...\n");
    result = benchmark_delete(db, n);
    printf("  Time: %.4fs\n", result.elapsed);
    printf("  Throughput: %.0f ops/sec\n", result.ops_per_sec);
    printf("  Latency: %.4f ms/op\n", (result.elapsed/n)*1000);
    
    printf("\n============================================================\n");
    printf("Benchmark completed!\n");
    printf("============================================================\n");
    
    dbx_close(db);
    return 0;
}
