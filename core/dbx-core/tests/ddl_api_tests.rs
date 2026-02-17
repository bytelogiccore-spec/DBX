//! DDL API Tests - Index Management
//!
//! Tests for SQL-based index management functions

use dbx_core::Database;
use arrow::datatypes::{DataType, Field, Schema};

#[test]
fn test_create_and_drop_sql_index() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("email", DataType::Utf8, true),
    ]);
    db.create_table("users", schema)?;

    // Create SQL index
    db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    assert!(db.sql_index_exists("idx_email"));

    // List indexes
    let indexes = db.list_sql_indexes("users");
    assert!(indexes.contains(&"idx_email".to_string()));

    // Drop SQL index
    db.drop_sql_index("users", "idx_email")?;
    assert!(!db.sql_index_exists("idx_email"));

    Ok(())
}

#[test]
fn test_multiple_sql_indexes() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("email", DataType::Utf8, true),
        Field::new("name", DataType::Utf8, true),
    ]);
    db.create_table("users", schema)?;

    // Create multiple SQL indexes
    db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    db.create_sql_index("users", "idx_name", vec!["name".to_string()])?;

    // List indexes
    let indexes = db.list_sql_indexes("users");
    assert_eq!(indexes.len(), 2);
    assert!(indexes.contains(&"idx_email".to_string()));
    assert!(indexes.contains(&"idx_name".to_string()));

    // Drop one index
    db.drop_sql_index("users", "idx_email")?;
    let indexes = db.list_sql_indexes("users");
    assert_eq!(indexes.len(), 1);
    assert!(indexes.contains(&"idx_name".to_string()));

    Ok(())
}

#[test]
fn test_sql_index_on_nonexistent_table() {
    let db = Database::open_in_memory().unwrap();

    // Try to create index on non-existent table
    let result = db.create_sql_index("nonexistent", "idx", vec!["col".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_ddl_api_integration() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // 1. Create table (DDL API)
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("email", DataType::Utf8, true),
        Field::new("name", DataType::Utf8, true),
    ]);
    db.create_table("users", schema)?;
    assert!(db.table_exists("users"));
    
    // Debug: Check if schema is stored
    let stored_schema = db.get_table_schema("users")?;
    println!("✅ Table 'users' created with schema: {:?}", stored_schema);

    // 2. Insert data using SQL
    println!("Attempting INSERT...");
    db.execute_sql("INSERT INTO users (id, email, name) VALUES (1, 'test@example.com', 'Alice')")?;
    println!("✅ INSERT successful");

    // 3. Query via SQL API (with WHERE clause - tests Binary type comparison fix!)
    println!("Attempting SELECT...");
    
    // Debug: Print all registered tables
    let tables = db.list_tables();
    println!("Registered tables: {:?}", tables);
    
    let results = db.execute_sql("SELECT * FROM users WHERE email = 'test@example.com'")?;  // 소문자로 변경
    assert!(!results.is_empty());
    println!("✅ SELECT successful, found {} rows", results.len());

    // 4. Drop table (DDL API)
    db.drop_table("users")?;
    assert!(!db.table_exists("users"));

    Ok(())
}

#[test]
fn test_sql_index_vs_hash_index() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("email", DataType::Utf8, true),
    ]);
    db.create_table("users", schema)?;

    // Create SQL index (named index)
    db.create_sql_index("users", "idx_email", vec!["email".to_string()])?;
    assert!(db.sql_index_exists("idx_email"));

    // Create Hash index (table, column)
    db.create_index("users", "id")?;
    assert!(db.has_index("users", "id"));

    // Both should coexist
    assert!(db.sql_index_exists("idx_email"));
    assert!(db.has_index("users", "id"));

    // Drop both
    db.drop_sql_index("users", "idx_email")?;
    db.drop_index("users", "id")?;

    assert!(!db.sql_index_exists("idx_email"));
    assert!(!db.has_index("users", "id"));

    Ok(())
}

