//! DDL Test Example
//!
//! Tests DROP TABLE and CREATE TABLE functionality

use dbx_core::{Database, DbxResult};

fn main() -> DbxResult<()> {
    println!("=== DBX DDL Test ===\n");
    
    let db = Database::open_in_memory()?;
    println!("✓ Database opened (in-memory)\n");
    
    // ═══════════════════════════════════════════
    // Test 1: CREATE TABLE
    // ═══════════════════════════════════════════
    println!("Test 1: CREATE TABLE");
    
    match db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT)") {
        Ok(result) => {
            println!("  ✓ CREATE TABLE succeeded");
            println!("  Result:\n{}", result);
        }
        Err(e) => {
            println!("  ✗ CREATE TABLE failed: {}", e);
        }
    }
    
    // Insert some data
    db.execute_sql("INSERT INTO users (id, name, age) VALUES (1, 'Alice', 25)")?;
    db.execute_sql("INSERT INTO users (id, name, age) VALUES (2, 'Bob', 30)")?;
    println!("  ✓ Inserted 2 records\n");
    
    // ═══════════════════════════════════════════
    // Test 2: SELECT from created table
    // ═══════════════════════════════════════════
    println!("Test 2: SELECT from created table");
    
    match db.execute_sql("SELECT * FROM users") {
        Ok(result) => {
            println!("  ✓ SELECT succeeded");
            println!("  Result:\n{}", result);
        }
        Err(e) => {
            println!("  ✗ SELECT failed: {}", e);
        }
    }
    
    println!("\n--- Test Complete ---\n");
    
    // ═══════════════════════════════════════════
    // Test 3: DROP TABLE
    // ═══════════════════════════════════════════
    println!("Test 3: DROP TABLE");
    
    match db.execute_sql("DROP TABLE users") {
        Ok(result) => {
            println!("  ✓ DROP TABLE succeeded");
            println!("  Result:\n{}", result);
        }
        Err(e) => {
            println!("  ✗ DROP TABLE failed: {}", e);
        }
    }
    
    // Try to SELECT from dropped table (should fail)
    match db.execute_sql("SELECT * FROM users") {
        Ok(_) => {
            println!("  ✗ SELECT should have failed after DROP TABLE");
        }
        Err(e) => {
            println!("  ✓ SELECT correctly failed: {}", e);
        }
    }
    
    println!("\n=== All tests complete ===");
    
    Ok(())
}
