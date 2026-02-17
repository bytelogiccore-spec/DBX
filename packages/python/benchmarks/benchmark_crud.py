"""
DBX Python Bindings - Performance Benchmark

Measures the performance of basic CRUD operations.
"""

import time
import sys
import os

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from dbx_py import Database


def benchmark_insert(db, n=10000):
    """Benchmark insert operations"""
    start = time.perf_counter()
    
    for i in range(n):
        key = f"key:{i}".encode()
        value = f"value:{i}".encode()
        db.insert("bench", key, value)
    
    end = time.perf_counter()
    elapsed = end - start
    ops_per_sec = n / elapsed
    
    return elapsed, ops_per_sec


def benchmark_get(db, n=10000):
    """Benchmark get operations"""
    start = time.perf_counter()
    
    for i in range(n):
        key = f"key:{i}".encode()
        _ = db.get("bench", key)
    
    end = time.perf_counter()
    elapsed = end - start
    ops_per_sec = n / elapsed
    
    return elapsed, ops_per_sec


def benchmark_delete(db, n=10000):
    """Benchmark delete operations"""
    start = time.perf_counter()
    
    for i in range(n):
        key = f"key:{i}".encode()
        db.delete("bench", key)
    
    end = time.perf_counter()
    elapsed = end - start
    ops_per_sec = n / elapsed
    
    return elapsed, ops_per_sec


def main():
    print("=" * 60)
    print("DBX Python Bindings - Performance Benchmark")
    print("=" * 60)
    
    # Open in-memory database
    db = Database.open_in_memory()
    
    try:
        n = 10000
        print(f"\nRunning benchmarks with {n:,} operations...\n")
        
        # Benchmark INSERT
        print("Benchmarking INSERT...")
        elapsed, ops_per_sec = benchmark_insert(db, n)
        print(f"  Time: {elapsed:.4f}s")
        print(f"  Throughput: {ops_per_sec:,.0f} ops/sec")
        print(f"  Latency: {(elapsed/n)*1000:.4f} ms/op")
        
        # Benchmark GET
        print("\nBenchmarking GET...")
        elapsed, ops_per_sec = benchmark_get(db, n)
        print(f"  Time: {elapsed:.4f}s")
        print(f"  Throughput: {ops_per_sec:,.0f} ops/sec")
        print(f"  Latency: {(elapsed/n)*1000:.4f} ms/op")
        
        # Benchmark DELETE
        print("\nBenchmarking DELETE...")
        elapsed, ops_per_sec = benchmark_delete(db, n)
        print(f"  Time: {elapsed:.4f}s")
        print(f"  Throughput: {ops_per_sec:,.0f} ops/sec")
        print(f"  Latency: {(elapsed/n)*1000:.4f} ms/op")
        
        print("\n" + "=" * 60)
        print("Benchmark completed!")
        print("=" * 60)
        
    finally:
        db.close()


if __name__ == "__main__":
    main()
