#include "../include/dbx.hpp"
#include <iostream>
#include <chrono>
#include <iomanip>
#include <string>

using namespace dbx;
using namespace std::chrono;

constexpr size_t NUM_ENTRIES = 10000;
constexpr size_t NUM_ITERATIONS = 100;

std::string formatKey(size_t i) {
    char buf[32];
    snprintf(buf, sizeof(buf), "key_%08zu", i);
    return std::string(buf);
}

std::string formatValue(size_t i) {
    char buf[64];
    snprintf(buf, sizeof(buf), "value_%08zu_data", i);
    return std::string(buf);
}

int main() {
    std::cout << "============================================================\n";
    std::cout << "DBX C++ Zero-Copy SCAN Test\n";
    std::cout << "============================================================\n";

    try {
        auto db = Database::openInMemory();

        // Insert test data
        std::cout << "Inserting " << NUM_ENTRIES << " entries...\n";
        for (size_t i = 0; i < NUM_ENTRIES; ++i) {
            db.insert("bench", formatKey(i), formatValue(i));
        }

        // Warmup
        for (int i = 0; i < 5; ++i) {
            auto _ = db.scan("bench");
            auto __ = db.scanZeroCopy("bench");
        }

        // Test Standard SCAN
        auto start = high_resolution_clock::now();
        for (size_t i = 0; i < NUM_ITERATIONS; ++i) {
            auto result = db.scan("bench");
        }
        auto end = high_resolution_clock::now();
        double standardTime = duration_cast<microseconds>(end - start).count() / 1000.0 / NUM_ITERATIONS;

        // Test Zero-Copy SCAN (no parse)
        start = high_resolution_clock::now();
        for (size_t i = 0; i < NUM_ITERATIONS; ++i) {
            auto result = db.scanZeroCopy("bench");
            auto count = result.count();
            auto [data, len] = result.getRawData();
        }
        end = high_resolution_clock::now();
        double zeroCopyTime = duration_cast<microseconds>(end - start).count() / 1000.0 / NUM_ITERATIONS;

        // Test Zero-Copy SCAN (with parse)
        start = high_resolution_clock::now();
        for (size_t i = 0; i < NUM_ITERATIONS; ++i) {
            auto result = db.scanZeroCopy("bench");
            auto pairs = result.toPairs();
        }
        end = high_resolution_clock::now();
        double zeroCopyParseTime = duration_cast<microseconds>(end - start).count() / 1000.0 / NUM_ITERATIONS;

        // Results
        std::cout << "\nResults:\n";
        std::cout << std::fixed << std::setprecision(2);
        std::cout << "Standard SCAN:              " << standardTime << "ms\n";
        std::cout << "Zero-Copy SCAN (no parse):  " << zeroCopyTime << "ms\n";
        std::cout << "Zero-Copy SCAN (w/ parse):  " << zeroCopyParseTime << "ms\n";
        std::cout << "\n";

        if (zeroCopyTime < standardTime) {
            double speedup = standardTime / zeroCopyTime;
            double improvement = ((standardTime - zeroCopyTime) / standardTime) * 100.0;
            std::cout << std::setprecision(2);
            std::cout << "Speedup (no parse):         " << speedup << "x\n";
            std::cout << "Improvement (no parse):     " << improvement << "%\n";
        }

        if (zeroCopyParseTime < standardTime) {
            double speedup = standardTime / zeroCopyParseTime;
            double improvement = ((standardTime - zeroCopyParseTime) / standardTime) * 100.0;
            std::cout << "Speedup (w/ parse):         " << speedup << "x\n";
            std::cout << "Improvement (w/ parse):     " << improvement << "%\n";
        }

        std::cout << "\n";
        std::cout << "============================================================\n";
        std::cout << "Test completed! ✅\n";
        std::cout << "============================================================\n";

    } catch (const DatabaseError& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }

    return 0;
}
