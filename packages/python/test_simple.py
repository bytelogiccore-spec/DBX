import sys
import os

# Add parent directory to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

try:
    from dbx_py import Database
    print("✓ Import successful")
    
    db = Database.open_in_memory()
    print("✓ Database opened")
    
    db.insert("test", b"key1", b"value1")
    print("✓ Insert successful")
    
    value = db.get("test", b"key1")
    print(f"✓ Get successful: {value}")
    
    db.close()
    print("✓ All tests passed!")
    
except Exception as e:
    print(f"✗ Error: {e}")
    import traceback
    traceback.print_exc()
