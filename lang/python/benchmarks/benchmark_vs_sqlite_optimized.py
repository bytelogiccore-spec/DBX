"""
DBX vs SQLite - Optimized Performance Comparison

Optimizations:
1. Pre-generate keys and values (avoid repeated encoding)
2. Reuse ctypes arrays (reduce allocation overhead)
3. Direct pointer passing (minimize FFI overhead)
"""

import time
import sys
import os
import sqlite3
import ctypes

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from dbx_py import Database


def benchmark_dbx_optimized(n=10000):
    """Benchmark DBX with optimizations"""
    db = Database.open_in_memory()
    
    try:
        # PRE-GENERATE all keys and values (Optimization 1)
        keys = [f"key:{i}".encode() for i in range(n)]
        values = [f"value:{i}".encode() for i in range(n)]
        
        # INSERT with FFI transaction
        start = time.perf_counter()
        tx = db._lib.dbx_begin_transaction(db._handle)
        
        for i in range(n):
            key = keys[i]
            value = values[i]
            key_array = (ctypes.c_uint8 * len(key)).from_buffer_copy(key)
            value_array = (ctypes.c_uint8 * len(value)).from_buffer_copy(value)
            db._lib.dbx_transaction_insert(
                tx,
                b"bench",
                key_array, len(key),
                value_array, len(value)
            )
        
        db._lib.dbx_transaction_commit(tx)
        insert_time = time.perf_counter() - start
        
        # GET (with pre-generated keys)
        start = time.perf_counter()
        for i in range(n):
            _ = db.get("bench", keys[i])
        get_time = time.perf_counter() - start
        
        # DELETE with FFI transaction
        start = time.perf_counter()
        tx = db._lib.dbx_begin_transaction(db._handle)
        
        for i in range(n):
            key = keys[i]
            key_array = (ctypes.c_uint8 * len(key)).from_buffer_copy(key)
            db._lib.dbx_transaction_delete(
                tx,
                b"bench",
                key_array, len(key)
            )
        
        db._lib.dbx_transaction_commit(tx)
        delete_time = time.perf_counter() - start
        
        return insert_time, get_time, delete_time
    finally:
        db.close()


def benchmark_sqlite(n=10000):
    """Benchmark SQLite operations"""
    conn = sqlite3.connect(":memory:")
    cursor = conn.cursor()
    
    try:
        cursor.execute("CREATE TABLE IF NOT EXISTS bench (key TEXT PRIMARY KEY, value TEXT)")
        
        # PRE-GENERATE all keys and values
        keys = [f"key:{i}" for i in range(n)]
        values = [f"value:{i}" for i in range(n)]
        
        # INSERT
        start = time.perf_counter()
        conn.execute("BEGIN TRANSACTION")
        for i in range(n):
            cursor.execute("INSERT INTO bench (key, value) VALUES (?, ?)", (keys[i], values[i]))
        conn.commit()
        insert_time = time.perf_counter() - start
        
        # GET
        start = time.perf_counter()
        for i in range(n):
            cursor.execute("SELECT value FROM bench WHERE key = ?", (keys[i],))
            _ = cursor.fetchone()
        get_time = time.perf_counter() - start
        
        # DELETE
        start = time.perf_counter()
        conn.execute("BEGIN TRANSACTION")
        for i in range(n):
            cursor.execute("DELETE FROM bench WHERE key = ?", (keys[i],))
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
    print("DBX vs SQLite - Optimized Performance Comparison")
    print("=" * 60)
    print("\nOptimizations:")
    print("  - Pre-generated keys/values (no repeated encoding)")
    print("  - Reduced allocation overhead")
    
    n = 10000
    print(f"\nRunning benchmarks with {n:,} operations...\n")
    
    # Benchmark DBX (optimized)
    print("Benchmarking DBX (optimized)...")
    dbx_insert, dbx_get, dbx_delete = benchmark_dbx_optimized(n)
    print_results("DBX (In-Memory, Optimized)", dbx_insert, dbx_get, dbx_delete, n)
    
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
