"""
FFI Performance Benchmark for DBX Python Bindings

Measures INSERT, GET, and SCAN performance to quantify FFI overhead
compared to Rust Core benchmarks.

Run with: pytest benchmarks/test_ffi_benchmark.py --benchmark-only
"""

import pytest

try:
    from dbx_py import Database
except ImportError:
    pytest.skip("dbx_py not installed", allow_module_level=True)


NUM_ENTRIES = 10_000


def generate_test_data():
    """Generate identical test data as Rust Core benchmark"""
    return [
        (
            f"key_{i:08d}".encode(),
            f"value_{i:08d}_data".encode()
        )
        for i in range(NUM_ENTRIES)
    ]


def test_insert_10k(benchmark):
    """Benchmark: INSERT 10,000 records"""
    data = generate_test_data()
    
    def insert():
        db = Database.open_in_memory()
        for key, value in data:
            db.insert("bench", key, value)
    
    result = benchmark(insert)
    print(f"\nINSERT 10k: {result.stats.mean * 1000:.2f}ms")


def test_get_10k(benchmark):
    """Benchmark: GET 10,000 records"""
    data = generate_test_data()
    
    # Setup: Insert data first
    db = Database.open_in_memory()
    for key, value in data:
        db.insert("bench", key, value)
    
    def get():
        for key, _ in data:
            db.get("bench", key)
    
    result = benchmark(get)
    print(f"\nGET 10k: {result.stats.mean * 1000:.2f}ms")


def test_scan_10k(benchmark):
    """Benchmark: SCAN 10,000 records"""
    data = generate_test_data()
    
    # Setup: Insert data first
    db = Database.open_in_memory()
    for key, value in data:
        db.insert("bench", key, value)
    
    def scan():
        db.scan("bench")
    
    result = benchmark(scan)
    print(f"\nSCAN 10k: {result.stats.mean * 1000:.2f}ms")


if __name__ == "__main__":
    pytest.main([__file__, "--benchmark-only", "-v"])
