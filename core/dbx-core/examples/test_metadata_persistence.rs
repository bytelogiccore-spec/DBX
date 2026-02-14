//! Integration test for metadata persistence
//!
//! Tests that table schemas are persisted across database restarts.

use dbx_core::Database;
use tempfile::TempDir;

fn main() -> dbx_core::DbxResult<()> {
    println!("\n═══════════════════════════════════════════");
    println!("  Metadata Persistence Integration Test");
    println!("═══════════════════════════════════════════\n");

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().to_path_buf();

    // Phase 1: Create database and tables
    println!("Phase 1: Creating database with schemas...");
    {
        let db = Database::open(&db_path)?;

        // Create tables
        db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT)")?;
        db.execute_sql("CREATE TABLE products (id INT, name TEXT, price REAL)")?;
        println!("  ✓ Created 2 tables");

        // Flush to ensure metadata is persisted
        db.flush()?;
        println!("  ✓ Flushed to storage");
    }

    println!("\nPhase 2: Reopening database and verifying schema persistence...");
    {
        let db = Database::open(&db_path)?;
        println!("  ✓ Database reopened successfully");

        // Try to create the same table again - should fail if schema was persisted
        match db.execute_sql("CREATE TABLE users (id INT, name TEXT)") {
            Err(e) => {
                println!(
                    "  ✓ users table schema persisted (got expected error: {})",
                    e
                );
            }
            Ok(_) => {
                panic!("users table should already exist!");
            }
        }

        // Try to create the same table with IF NOT EXISTS - should succeed silently
        db.execute_sql("CREATE TABLE IF NOT EXISTS users (id INT, name TEXT)")?;
        println!("  ✓ CREATE TABLE IF NOT EXISTS works correctly");

        // Try to create a new table - should work
        db.execute_sql("CREATE TABLE orders (id INT, total REAL)")?;
        println!("  ✓ Can create new tables");

        db.flush()?;
    }

    println!("\nPhase 3: Reopening and testing DROP TABLE persistence...");
    {
        let db = Database::open(&db_path)?;

        // Verify all 3 tables exist
        match db.execute_sql("CREATE TABLE users (id INT)") {
            Err(_) => println!("  ✓ users table still exists"),
            Ok(_) => panic!("users should exist"),
        }

        match db.execute_sql("CREATE TABLE products (id INT)") {
            Err(_) => println!("  ✓ products table still exists"),
            Ok(_) => panic!("products should exist"),
        }

        match db.execute_sql("CREATE TABLE orders (id INT)") {
            Err(_) => println!("  ✓ orders table still exists"),
            Ok(_) => panic!("orders should exist"),
        }

        // Test DROP TABLE
        db.execute_sql("DROP TABLE products")?;
        println!("  ✓ DROP TABLE executed");
        db.flush()?;
    }

    println!("\nPhase 4: Reopening after DROP TABLE...");
    {
        let db = Database::open(&db_path)?;

        // Verify products is gone
        db.execute_sql("CREATE TABLE products (id INT, name TEXT)")?;
        println!("  ✓ products table was correctly dropped (can recreate it)");

        // Verify users and orders still exist
        match db.execute_sql("CREATE TABLE users (id INT)") {
            Err(_) => println!("  ✓ users table still exists"),
            Ok(_) => panic!("users should still exist"),
        }

        match db.execute_sql("CREATE TABLE orders (id INT)") {
            Err(_) => println!("  ✓ orders table still exists"),
            Ok(_) => panic!("orders should still exist"),
        }
    }

    println!("\n═══════════════════════════════════════════");
    println!("  ✅ All metadata persistence tests passed!");
    println!("═══════════════════════════════════════════\n");

    Ok(())
}
