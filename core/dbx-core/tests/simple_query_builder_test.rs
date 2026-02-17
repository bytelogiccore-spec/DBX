//! Simple Query Builder Test

use arrow::datatypes::{DataType, Field, Schema};
use dbx_core::Database;

#[test]
fn test_query_builder_simple() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // Create table using DDL API
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("name", DataType::Utf8, true),
    ]);
    db.create_table("test_users", schema)?;

    // Insert data using SQL (to avoid Binary type comparison issue)
    db.execute_sql("INSERT INTO test_users (id, name) VALUES (1, 'Alice')")?;
    db.execute_sql("INSERT INTO test_users (id, name) VALUES (2, 'Bob')")?;

    // Test Query Builder
    let results = db
        .query_builder()
        .select(&["id", "name"])
        .from("test_users")
        .execute()?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].num_rows(), 2);

    Ok(())
}
