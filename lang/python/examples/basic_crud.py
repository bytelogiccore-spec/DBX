"""
DBX Python Bindings - Basic CRUD Example

This example demonstrates basic database operations using DBX Python bindings.
"""

from dbx_py import Database


def main():
    # Open an in-memory database
    print("Opening in-memory database...")
    db = Database.open_in_memory()
    
    # Or open a file-based database:
    # db = Database("my_database.db")
    
    try:
        # Insert some data
        print("\nInserting data...")
        db.insert("users", b"user:1", b"Alice")
        db.insert("users", b"user:2", b"Bob")
        db.insert("users", b"user:3", b"Charlie")
        
        # Get data
        print("\nRetrieving data...")
        value = db.get("users", b"user:1")
        if value:
            print(f"user:1 = {value.decode('utf-8')}")
        
        value = db.get("users", b"user:2")
        if value:
            print(f"user:2 = {value.decode('utf-8')}")
        
        # Count rows
        count = db.count("users")
        print(f"\nTotal users: {count}")
        
        # Delete a row
        print("\nDeleting user:2...")
        db.delete("users", b"user:2")
        
        # Verify deletion
        value = db.get("users", b"user:2")
        if value is None:
            print("user:2 successfully deleted")
        
        # Count again
        count = db.count("users")
        print(f"Total users after deletion: {count}")
        
        # Flush to disk (if using file-based database)
        print("\nFlushing to disk...")
        db.flush()
        
        print("\n✓ All operations completed successfully!")
        
    finally:
        # Close the database
        print("\nClosing database...")
        db.close()


if __name__ == "__main__":
    main()
