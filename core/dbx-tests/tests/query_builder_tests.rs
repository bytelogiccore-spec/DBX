//! Query Builder Tests
//!
//! Tests for the Query Builder API

use arrow::array::{Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use dbx_core::Database;

#[test]
fn test_basic_select() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("age", DataType::Int32, true),
    ]);
    db.create_table("users_basic_select", schema)?;

    // Insert test data
    db.execute_sql("INSERT INTO users_basic_select (id, name, age) VALUES (1, 'Alice', 25)")?;
    db.execute_sql("INSERT INTO users_basic_select (id, name, age) VALUES (2, 'Bob', 30)")?;
    db.execute_sql("INSERT INTO users_basic_select (id, name, age) VALUES (3, 'Charlie', 35)")?;

    // Test basic SELECT
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("users_basic_select")
        .execute()?;

    let total_rows: usize = results.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 3);
    assert_eq!(results[0].num_columns(), 2);

    Ok(())
}

#[test]
fn test_where_clause() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("age", DataType::Int32, true),
    ]);
    db.create_table("users_where", schema)?;

    // Insert test data
    db.execute_sql("INSERT INTO users_where (id, name, age) VALUES (1, 'Alice', 25)")?;
    db.execute_sql("INSERT INTO users_where (id, name, age) VALUES (2, 'Bob', 30)")?;
    db.execute_sql("INSERT INTO users_where (id, name, age) VALUES (3, 'Charlie', 35)")?;

    // Test WHERE clause
    let results = db
        .query_builder()
        .select(&["id", "name", "age"])
        .from("users_where")
        .where_("age", ">", "25")
        .execute()?;

    let total_rows: usize = results.iter().map(|b| b.num_rows()).sum();
    assert_eq!(total_rows, 2); // Bob and Charlie

    Ok(())
}

#[test]
fn test_and_or_conditions() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("age", DataType::Int32, true),
        Field::new("status", DataType::Utf8, true),
    ]);
    db.create_table("users_and_or", schema)?;

    // Insert test data
    db.execute_sql(
        "INSERT INTO users_and_or (id, name, age, status) VALUES (1, 'Alice', 25, 'active')",
    )?;
    db.execute_sql(
        "INSERT INTO users_and_or (id, name, age, status) VALUES (2, 'Bob', 30, 'inactive')",
    )?;
    db.execute_sql(
        "INSERT INTO users_and_or (id, name, age, status) VALUES (3, 'Charlie', 35, 'active')",
    )?;

    // Test AND condition
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("users_and_or")
        .where_("age", ">", "25")
        .and("status", "=", "'active'")
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 1); // Only Charlie

    Ok(())
}

#[test]
fn test_order_by() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("age", DataType::Int32, true),
    ]);
    db.create_table("users_order", schema)?;

    // Insert test data
    db.execute_sql("INSERT INTO users_order (id, name, age) VALUES (1, 'Charlie', 35)")?;
    db.execute_sql("INSERT INTO users_order (id, name, age) VALUES (2, 'Alice', 25)")?;
    db.execute_sql("INSERT INTO users_order (id, name, age) VALUES (3, 'Bob', 30)")?;

    // Test ORDER BY
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("users_order")
        .order_by("name", "ASC")
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 3);

    // Verify order: Alice, Bob, Charlie
    let name_array = results[0]
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    assert_eq!(name_array.value(0), "Alice");
    assert_eq!(name_array.value(1), "Bob");
    assert_eq!(name_array.value(2), "Charlie");

    Ok(())
}

#[test]
fn test_limit_offset() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
    ]);
    db.create_table("users_limit", schema)?;

    // Insert test data
    for i in 1..=10 {
        db.execute_sql(&format!(
            "INSERT INTO users_limit (id, name) VALUES ({}, 'User{}')",
            i, i
        ))?;
    }

    // Test LIMIT
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("users_limit")
        .order_by("id", "ASC")
        .limit(5)
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 5);

    // Test LIMIT with OFFSET
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("users_limit")
        .order_by("id", "ASC")
        .limit(3)
        .offset(5)
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 3);

    // Verify IDs are 6, 7, 8
    let id_array = results[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    assert_eq!(id_array.value(0), 6);
    assert_eq!(id_array.value(1), 7);
    assert_eq!(id_array.value(2), 8);

    Ok(())
}

#[test]
fn test_count_aggregate() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("status", DataType::Utf8, true),
    ]);
    db.create_table("users_count", schema)?;

    // Insert test data
    db.execute_sql("INSERT INTO users_count (id, name, status) VALUES (1, 'Alice', 'active')")?;
    db.execute_sql("INSERT INTO users_count (id, name, status) VALUES (2, 'Bob', 'active')")?;
    db.execute_sql("INSERT INTO users_count (id, name, status) VALUES (3, 'Charlie', 'inactive')")?;

    // Test COUNT
    let results = db
        .query_builder()
        .count("*")
        .from("users_count")
        .where_("status", "=", "'active'")
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 1);

    // Verify count is 2
    let count_array = results[0]
        .column(0)
        .as_any()
        .downcast_ref::<Int64Array>()
        .unwrap();
    assert_eq!(count_array.value(0), 2);

    Ok(())
}

#[test]
fn test_complex_query() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
        Field::new("age", DataType::Int32, true),
        Field::new("status", DataType::Utf8, true),
    ]);
    db.create_table("users_complex", schema)?;

    // Insert test data
    db.execute_sql(
        "INSERT INTO users_complex (id, name, age, status) VALUES (1, 'Alice', 25, 'active')",
    )?;
    db.execute_sql(
        "INSERT INTO users_complex (id, name, age, status) VALUES (2, 'Bob', 30, 'active')",
    )?;
    db.execute_sql(
        "INSERT INTO users_complex (id, name, age, status) VALUES (3, 'Charlie', 35, 'inactive')",
    )?;
    db.execute_sql(
        "INSERT INTO users_complex (id, name, age, status) VALUES (4, 'David', 28, 'active')",
    )?;
    db.execute_sql(
        "INSERT INTO users_complex (id, name, age, status) VALUES (5, 'Eve', 32, 'active')",
    )?;

    // Complex query: active users over 27, ordered by age, limit 2
    let results = db
        .query_builder()
        .select(&["id", "name", "age"])
        .from("users_complex")
        .where_("status", "=", "'active'")
        .and("age", ">", "27")
        .order_by("age", "ASC")
        .limit(2)
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 2);

    // Verify: David (28) and Bob (30)
    let name_array = results[0]
        .column(1)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap();
    assert_eq!(name_array.value(0), "David");
    assert_eq!(name_array.value(1), "Bob");

    Ok(())
}
