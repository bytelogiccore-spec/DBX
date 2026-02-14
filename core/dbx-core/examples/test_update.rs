// Simple UPDATE test example
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    println!("=== UPDATE Test Example ===\n");

    let db = Database::open_in_memory()?;

    // Create table and insert data
    println!("1. Creating table and inserting data...");
    db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT)")?;
    db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 25)")?;
    db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 30)")?;
    println!("   ✓ Data inserted\n");

    // Try UPDATE without WHERE (should work)
    println!("2. Testing UPDATE without WHERE clause...");
    match db.execute_sql("UPDATE users SET age = 35") {
        Ok(batches) => {
            println!("   ✓ UPDATE succeeded");
            for batch in batches {
                println!("   Result: {:?}", batch);
            }
        }
        Err(e) => {
            println!("   ✗ UPDATE failed: {}", e);
        }
    }
    println!();

    // Try UPDATE with WHERE (should return not implemented)
    println!("3. Testing UPDATE with WHERE clause...");
    match db.execute_sql("UPDATE users SET age = 40 WHERE id = 1") {
        Ok(batches) => {
            println!("   ✓ UPDATE succeeded");
            for batch in batches {
                println!("   Result: {:?}", batch);
            }
        }
        Err(e) => {
            println!("   ✗ UPDATE failed (expected): {}", e);
        }
    }

    Ok(())
}
