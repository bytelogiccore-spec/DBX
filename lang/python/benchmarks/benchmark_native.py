"""
DBX Native (PyO3) vs SQLite - Performance Comparison

Tests the PyO3 native bindings performance.
"""

import time
import sys
import os
import sqlite3

# Test native import
try:
    import dbx_native
    print(f"✓ dbx_native imported successfully")
    NATIVE_AVAILABLE = True
except ImportError as e:
    print(f"✗ dbx_native import failed: {e}")
    NATIVE_AVAILABLE = False
    sys.exit(1)


def benchmark_dbx_native(n=10000):
    """Benchmark DBX native PyO3 bindings"""
    db = dbx_native.Database.open_in_memory()
    
    # INSERT with transaction
    start = time.perf_counter()
    tx = db.begin_transaction()
    for i in range(n):
        key = f"key:{i}".encode()
        value = f"value:{i}".encode()
        tx.insert("bench", key, value)
    tx.commit()
    insert_time = time.perf_counter() - start
    
    # GET
    start = time.perf_counter()
    for i in range(n):
        key = f"key:{i}".encode()
        _ = db.get("bench", key)
    get_time = time.perf_counter() - start
    
    # DELETE with transaction
    start = time.perf_counter()
    tx = db.begin_transaction()
    for i in range(n):
        key = f"key:{i}".encode()
        tx.delete("bench", key)
    tx.commit()
    delete_time = time.perf_counter() - start
    
    db.close()
    return insert_time, get_time, delete_time


def benchmark_sqlite(n=10000):
    """Benchmark SQLite operations"""
    conn = sqlite3.connect(":memory:")
    cursor = conn.cursor()
    
    try:
        cursor.execute("CREATE TABLE IF NOT EXISTS bench (key TEXT PRIMARY KEY, value TEXT)")
        
        # INSERT
        start = time.perf_counter()
        conn.execute("BEGIN TRANSACTION")
        for i in range(n):
            key = f"key:{i}"
            value = f"value:{i}"
            cursor.execute("INSERT INTO bench (key, value) VALUES (?, ?)", (key, value))
        conn.commit()
        insert_time = time.perf_counter() - start
        
        # GET
        start = time.perf_counter()
        for i in range(n):
            key = f"key:{i}"
            cursor.execute("SELECT value FROM bench WHERE key = ?", (key,))
            _ = cursor.fetchone()
        get_time = time.perf_counter() - start
        
        # DELETE
        start = time.perf_counter()
        conn.execute("BEGIN TRANSACTION")
        for i in range(n):
            key = f"key:{i}"
            cursor.execute("DELETE FROM bench WHERE key = ?", (key,))
        conn.commit()
        delete_time = time.perf_counter() - start
        
        return insert_time, get_time, delete_time
    finally:
        conn.close()


def print_results(name, insert_time, get_time, delete_time, n):
    """Print benchmark results"""
    print(f"\n{name}:")
    print(f"  INSERT: {insert_time:.4f}s ({n/insert_time:,.0f} ops/sec)")
    print(f"  GET:    {get_time:.4f}s ({n/get_time:,.0f} ops/sec)")
    print(f"  DELETE: {delete_time:.4f}s ({n/delete_time:,.0f} ops/sec)")


def main():
    print("=" * 60)
    print("DBX Native (PyO3) vs SQLite - Performance Comparison")
    print("=" * 60)
    
    n = 10000
    print(f"\nRunning benchmarks with {n:,} operations...\n")
    
    # Benchmark DBX Native
    print("Benchmarking DBX Native (PyO3)...")
    dbx_insert, dbx_get, dbx_delete = benchmark_dbx_native(n)
    print_results("DBX Native (PyO3)", dbx_insert, dbx_get, dbx_delete, n)
    
    # Benchmark SQLite
    print("\nBenchmarking SQLite (In-Memory)...")
    sql_insert, sql_get, sql_delete = benchmark_sqlite(n)
    print_results("SQLite (In-Memory)", sql_insert, sql_get, sql_delete, n)
    
    # Comparison
    print("\n" + "=" * 60)
    print("Performance Comparison:")
    print("=" * 60)
    
    if dbx_insert < sql_insert:
        print(f"INSERT: DBX is {sql_insert/dbx_insert:.2f}x faster")
    else:
        print(f"INSERT: SQLite is {dbx_insert/sql_insert:.2f}x faster")
    
    if dbx_get < sql_get:
        print(f"GET:    DBX is {sql_get/dbx_get:.2f}x faster")
    else:
        print(f"GET:    SQLite is {dbx_get/sql_get:.2f}x faster")
    
    if dbx_delete < sql_delete:
        print(f"DELETE: DBX is {sql_delete/dbx_delete:.2f}x faster")
    else:
        print(f"DELETE: SQLite is {dbx_delete/sql_delete:.2f}x faster")
    
    print("\n" + "=" * 60)
    print("Benchmark completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
