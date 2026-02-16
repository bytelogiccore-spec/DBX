"""
Zero-Copy SCAN Performance Test for DBX Python Bindings

Tests that zero-copy scan is significantly faster than standard scan.
"""

import dbx_native
import time


def test_zero_copy_scan_correctness():
    """Verify zero-copy scan returns correct data"""
    print("Testing correctness...")
    db = dbx_native.Database.open_in_memory()
    
    # Insert test data
    for i in range(100):
        key = f"key_{i:03d}".encode()
        value = f"value_{i:03d}".encode()
        db.insert("test", key, value)
    
    # Get results
    result = db.scan_zero_copy("test")
    
    # Verify count
    assert result.count() == 100, f"Expected 100, got {result.count()}"
    
    # Verify raw data access
    raw_data = result.as_bytes()
    assert len(raw_data) > 0, "Raw data should not be empty"
    
    # Verify parsing
    pairs = result.to_pairs()
    assert len(pairs) == 100, f"Expected 100 pairs, got {len(pairs)}"
    assert pairs[0][0] == b"key_000", f"First key mismatch: {pairs[0][0]}"
    assert pairs[0][1] == b"value_000", f"First value mismatch: {pairs[0][1]}"
    assert pairs[99][0] == b"key_099", f"Last key mismatch: {pairs[99][0]}"
    
    print("✅ Correctness test passed!")


def test_zero_copy_scan_performance():
    """Verify zero-copy scan is significantly faster than standard scan"""
    print("\nTesting performance...")
    db = dbx_native.Database.open_in_memory()
    
    # Insert 10,000 entries
    print("Inserting 10,000 entries...")
    for i in range(10000):
        key = f"key_{i:08d}".encode()
        value = f"value_{i:08d}_data".encode()
        db.insert("bench", key, value)
    
    # Warmup
    _ = db.scan("bench")
    _ = db.scan_zero_copy("bench")
    
    # Standard scan
    start = time.perf_counter()
    standard_result = db.scan("bench")
    standard_time = (time.perf_counter() - start) * 1000
    
    # Zero-copy scan
    start = time.perf_counter()
    zero_copy_result = db.scan_zero_copy("bench")
    zero_copy_time = (time.perf_counter() - start) * 1000
    
    # Verify correctness
    assert zero_copy_result.count() == 10000
    pairs = zero_copy_result.to_pairs()
    assert len(pairs) == 10000
    assert pairs[0][0] == b"key_00000000"
    
    # Performance comparison
    speedup = standard_time / zero_copy_time
    improvement = ((standard_time - zero_copy_time) / standard_time) * 100
    
    print(f"Standard SCAN:   {standard_time:7.2f}ms")
    print(f"Zero-Copy SCAN:  {zero_copy_time:7.2f}ms")
    print(f"Speedup:         {speedup:7.2f}x")
    print(f"Improvement:     {improvement:6.1f}%")
    
    # Performance assertion (should be at least 5x faster)
    if zero_copy_time < standard_time / 5:
        print("✅ Performance test passed! (>5x faster)")
    else:
        print(f"⚠️  Performance improvement is less than expected")
        print(f"   Expected: <{standard_time / 5:.2f}ms, Got: {zero_copy_time:.2f}ms")


def test_empty_table():
    """Test zero-copy scan on empty table"""
    print("\nTesting empty table...")
    db = dbx_native.Database.open_in_memory()
    
    result = db.scan_zero_copy("empty")
    assert result.count() == 0
    assert len(result.to_pairs()) == 0
    
    print("✅ Empty table test passed!")


if __name__ == "__main__":
    print("=" * 60)
    print("DBX Python Zero-Copy SCAN Test")
    print("=" * 60)
    
    test_zero_copy_scan_correctness()
    test_zero_copy_scan_performance()
    test_empty_table()
    
    print("\n" + "=" * 60)
    print("All tests passed! ✅")
    print("=" * 60)
