/**
 * DBX C++ Bindings - Performance Benchmark
 * 
 * Measures the performance of basic CRUD operations.
 */

#include <iostream>
#include <iomanip>
#include <chrono>
#include <string>
#include "dbx.hpp"

using namespace dbx;
using namespace std::chrono;

struct BenchmarkResult {
    double elapsed;
    double ops_per_sec;
};

BenchmarkResult benchmark_insert(Database& db, int n) {
    auto start = high_resolution_clock::now();
    
    for (int i = 0; i < n; i++) {
        std::string key = "key:" + std::to_string(i);
        std::string value = "value:" + std::to_string(i);
        db.insert("bench", key, value);
    }
    
    auto end = high_resolution_clock::now();
    duration<double> elapsed = end - start;
    
    return {
        elapsed.count(),
        n / elapsed.count()
    };
}

BenchmarkResult benchmark_get(Database& db, int n) {
    auto start = high_resolution_clock::now();
    
    for (int i = 0; i < n; i++) {
        std::string key = "key:" + std::to_string(i);
        auto value = db.getString("bench", key);
    }
    
    auto end = high_resolution_clock::now();
    duration<double> elapsed = end - start;
    
    return {
        elapsed.count(),
        n / elapsed.count()
    };
}

BenchmarkResult benchmark_delete(Database& db, int n) {
    auto start = high_resolution_clock::now();
    
    for (int i = 0; i < n; i++) {
        std::string key = "key:" + std::to_string(i);
        db.remove("bench", key);
    }
    
    auto end = high_resolution_clock::now();
    duration<double> elapsed = end - start;
    
    return {
        elapsed.count(),
        n / elapsed.count()
    };
}

int main() {
    try {
        std::cout << std::string(60, '=') << std::endl;
        std::cout << "DBX C++ Bindings - Performance Benchmark" << std::endl;
        std::cout << std::string(60, '=') << std::endl;
        
        auto db = Database::openInMemory();
        
        int n = 10000;
        std::cout << "\nRunning benchmarks with " << n << " operations...\n" << std::endl;
        
        // Benchmark INSERT
        std::cout << "Benchmarking INSERT..." << std::endl;
        auto result = benchmark_insert(db, n);
        std::cout << "  Time: " << std::fixed << std::setprecision(4) << result.elapsed << "s" << std::endl;
        std::cout << "  Throughput: " << std::fixed << std::setprecision(0) << result.ops_per_sec << " ops/sec" << std::endl;
        std::cout << "  Latency: " << std::fixed << std::setprecision(4) << (result.elapsed/n)*1000 << " ms/op" << std::endl;
        
        // Benchmark GET
        std::cout << "\nBenchmarking GET..." << std::endl;
        result = benchmark_get(db, n);
        std::cout << "  Time: " << std::fixed << std::setprecision(4) << result.elapsed << "s" << std::endl;
        std::cout << "  Throughput: " << std::fixed << std::setprecision(0) << result.ops_per_sec << " ops/sec" << std::endl;
        std::cout << "  Latency: " << std::fixed << std::setprecision(4) << (result.elapsed/n)*1000 << " ms/op" << std::endl;
        
        // Benchmark DELETE
        std::cout << "\nBenchmarking DELETE..." << std::endl;
        result = benchmark_delete(db, n);
        std::cout << "  Time: " << std::fixed << std::setprecision(4) << result.elapsed << "s" << std::endl;
        std::cout << "  Throughput: " << std::fixed << std::setprecision(0) << result.ops_per_sec << " ops/sec" << std::endl;
        std::cout << "  Latency: " << std::fixed << std::setprecision(4) << (result.elapsed/n)*1000 << " ms/op" << std::endl;
        
        std::cout << "\n" << std::string(60, '=') << std::endl;
        std::cout << "Benchmark completed!" << std::endl;
        std::cout << "=" << std::string(60, '=') << std::endl;
        
    } catch (const DatabaseError& e) {
        std::cerr << "Error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
