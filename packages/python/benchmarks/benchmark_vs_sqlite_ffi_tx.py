"""
DBX vs SQLite - Performance Comparison Benchmark (with FFI Transaction)

Compares DBX Python bindings with SQLite using native FFI transactions.
"""

import time
import sys
import os
import sqlite3
import ctypes

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from dbx_py import Database


def benchmark_dbx_with_ffi_transaction(n=10000):
    """Benchmark DBX operations with FFI transaction"""
    db = Database.open_in_memory()
    
    try:
        # INSERT with FFI transaction
        start = time.perf_counter()
        tx = db._lib.dbx_begin_transaction(db._handle)
        for i in range(n):
            key = f"key:{i}".encode()
            value = f"value:{i}".encode()
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
        
        # GET (no transaction needed)
        start = time.perf_counter()
        for i in range(n):
            key = f"key:{i}".encode()
            _ = db.get("bench", key)
        get_time = time.perf_counter() - start
        
        # DELETE with FFI transaction
        start = time.perf_counter()
        tx = db._lib.dbx_begin_transaction(db._handle)
        for i in range(n):
            key = f"key:{i}".encode()
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


def benchmark_sqlite(n=10000, in_memory=True):
    """Benchmark SQLite operations"""
    db_path = ":memory:" if in_memory else "sqlite_bench.db"
    conn = sqlite3.connect(db_path)
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
        if not in_memory and os.path.exists("sqlite_bench.db"):
            os.remove("sqlite_bench.db")


def print_results(name, insert_time, get_time, delete_time, n):
    """Print benchmark results"""
    print(f"\n{name}:")
    print(f"  INSERT: {insert_time:.4f}s ({n/insert_time:,.0f} ops/sec, {(insert_time/n)*1000:.4f} ms/op)")
    print(f"  GET:    {get_time:.4f}s ({n/get_time:,.0f} ops/sec, {(get_time/n)*1000:.4f} ms/op)")
    print(f"  DELETE: {delete_time:.4f}s ({n/delete_time:,.0f} ops/sec, {(delete_time/n)*1000:.4f} ms/op)")


def main():
    print("=" * 60)
    print("DBX vs SQLite - Performance Comparison (FFI Transaction)")
    print("=" * 60)
    
    n = 10000
    print(f"\nRunning benchmarks with {n:,} operations...\n")
    
    # Benchmark DBX with FFI transaction
    print("Benchmarking DBX (with FFI transaction)...")
    dbx_insert, dbx_get, dbx_delete = benchmark_dbx_with_ffi_transaction(n)
    print_results("DBX (In-Memory, FFI Transaction)", dbx_insert, dbx_get, dbx_delete, n)
    
    # Benchmark SQLite (Memory)
    print("\nBenchmarking SQLite (In-Memory)...")
    sql_insert, sql_get, sql_delete = benchmark_sqlite(n, in_memory=True)
    print_results("SQLite (In-Memory)", sql_insert, sql_get, sql_delete, n)
    
    # Comparison
    print("\n" + "=" * 60)
    print("Performance Comparison (DBX vs SQLite In-Memory):")
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
