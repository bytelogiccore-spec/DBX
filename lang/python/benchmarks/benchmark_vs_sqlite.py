"""
DBX vs SQLite - Performance Comparison Benchmark

Compares DBX Python bindings with SQLite (sqlite3 module).
"""

import time
import sys
import os
import sqlite3

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from dbx_py import Database


def benchmark_dbx(n=10000):
    """Benchmark DBX operations with transactions"""
    db = Database.open_in_memory()
    
    try:
        # INSERT with transaction
        start = time.perf_counter()
        tx = db.begin_transaction()
        for i in range(n):
            key = f"key:{i}".encode()
            value = f"value:{i}".encode()
            tx.insert("bench", key, value)
        tx.commit()
        insert_time = time.perf_counter() - start
        
        # GET (no transaction needed for reads)
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
        
        return insert_time, get_time, delete_time
    finally:
        db.close()


def benchmark_sqlite(n=10000, in_memory=True):
    """Benchmark SQLite operations with explicit transactions"""
    db_path = ":memory:" if in_memory else "sqlite_bench.db"
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    try:
        cursor.execute("CREATE TABLE IF NOT EXISTS bench (key TEXT PRIMARY KEY, value TEXT)")
        
        # INSERT with explicit transaction
        start = time.perf_counter()
        cursor.execute("BEGIN TRANSACTION")
        for i in range(n):
            key = f"key:{i}"
            value = f"value:{i}"
            cursor.execute("INSERT INTO bench (key, value) VALUES (?, ?)", (key, value))
        cursor.execute("COMMIT")
        insert_time = time.perf_counter() - start
        
        # GET (no transaction needed for reads)
        start = time.perf_counter()
        for i in range(n):
            key = f"key:{i}"
            cursor.execute("SELECT value FROM bench WHERE key = ?", (key,))
            _ = cursor.fetchone()
        get_time = time.perf_counter() - start
        
        # DELETE with explicit transaction
        start = time.perf_counter()
        cursor.execute("BEGIN TRANSACTION")
        for i in range(n):
            key = f"key:{i}"
            cursor.execute("DELETE FROM bench WHERE key = ?", (key,))
        cursor.execute("COMMIT")
        delete_time = time.perf_counter() - start
        
        return insert_time, get_time, delete_time
    finally:
        conn.close()
        if not in_memory and os.path.exists("sqlite_bench.db"):
            os.remove("sqlite_bench.db")


def print_results(name, insert_time, get_time, delete_time, n):
    """Print benchmark results"""
    print(f"\n{name}:")
    print(f"  INSERT: {insert_time:.4f}s ({n/insert_time:,.0f} ops/sec)")
    print(f"  GET:    {get_time:.4f}s ({n/get_time:,.0f} ops/sec)")
    print(f"  DELETE: {delete_time:.4f}s ({n/delete_time:,.0f} ops/sec)")


def main():
    print("=" * 60)
    print("DBX vs SQLite - Performance Comparison")
    print("=" * 60)
    
    n = 10000
    print(f"\nRunning benchmarks with {n:,} operations...\n")
    
    # Benchmark DBX
    print("Benchmarking DBX...")
    dbx_insert, dbx_get, dbx_delete = benchmark_dbx(n)
    print_results("DBX (In-Memory)", dbx_insert, dbx_get, dbx_delete, n)
    
    # Benchmark SQLite (Memory)
    print("\nBenchmarking SQLite (In-Memory)...")
    sql_insert, sql_get, sql_delete = benchmark_sqlite(n, in_memory=True)
    print_results("SQLite (In-Memory)", sql_insert, sql_get, sql_delete, n)
    
    # Benchmark SQLite (Disk)
    print("\nBenchmarking SQLite (Disk)...")
    sql_disk_insert, sql_disk_get, sql_disk_delete = benchmark_sqlite(n, in_memory=False)
    print_results("SQLite (Disk)", sql_disk_insert, sql_disk_get, sql_disk_delete, n)
    
    # Comparison
    print("\n" + "=" * 60)
    print("Performance Comparison (DBX vs SQLite In-Memory):")
    print("=" * 60)
    print(f"INSERT: DBX is {sql_insert/dbx_insert:.2f}x faster")
    print(f"GET:    DBX is {sql_get/dbx_get:.2f}x faster")
    print(f"DELETE: DBX is {sql_delete/dbx_delete:.2f}x faster")
    
    print("\n" + "=" * 60)
    print("Benchmark completed!")
    print("=" * 60)


if __name__ == "__main__":
    main()
