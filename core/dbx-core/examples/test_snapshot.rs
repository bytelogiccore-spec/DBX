//! Simple integration test for in-memory database save/load functionality

use dbx_core::Database;
use tempfile::NamedTempFile;

fn main() -> dbx_core::DbxResult<()> {
    println!("🧪 Testing In-Memory DB Save/Load Functionality (KV API)\n");

    test_kv_save_load()?;

    println!("\n✅ All tests passed!");
    Ok(())
}

fn test_kv_save_load() -> dbx_core::DbxResult<()> {
    println!("📝 Test: KV API save and load");

    let temp_file = NamedTempFile::new().unwrap();

    // Create in-memory DB and add data using KV API
    {
        let db = Database::open_in_memory()?;
        
        // Insert some data
        db.insert("users", b"user:1", b"Alice")?;
        db.insert("users", b"user:2", b"Bob")?;
        db.insert("products", b"prod:1", b"Laptop")?;
        
        println!("  ✓ Inserted 3 key-value pairs");

        // Flush to ensure data is in WOS (not just Delta)
        db.flush()?;
        println!("  ✓ Flushed data to WOS");

        // Save to file
        db.save_to_file(temp_file.path())?;
        println!("  ✓ Saved database to file");
    }

    // Load from file and verify
    {
        let db = Database::load_from_file(temp_file.path())?;
        println!("  ✓ Loaded database from file");

        // Verify data
        let value1 = db.get("users", b"user:1")?;
        let value2 = db.get("users", b"user:2")?;
        let value3 = db.get("products", b"prod:1")?;
        
        assert_eq!(value1, Some(b"Alice".to_vec()), "Expected Alice");
        assert_eq!(value2, Some(b"Bob".to_vec()), "Expected Bob");
        assert_eq!(value3, Some(b"Laptop".to_vec()), "Expected Laptop");
        
        println!("  ✓ All data verified correctly");
    }

    println!("  ✅ Test passed\n");
    Ok(())
}
