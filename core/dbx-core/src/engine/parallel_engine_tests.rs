//! Integration tests for parallel execution engine and batch SQL parser
//!
//! These tests verify that the new parallel features work correctly with
//! existing database functionality and don't break any existing features.

use crate::engine::{ParallelExecutionEngine, ParallelizationPolicy};
use crate::sql::ParallelSqlParser;
use crate::Database;
use std::sync::Arc;

#[test]
fn test_parallel_engine_basic() {
    let engine = ParallelExecutionEngine::new_auto().unwrap();
    assert!(engine.thread_count() > 0);
}

#[test]
fn test_parallel_engine_fixed_threads() {
    let engine = ParallelExecutionEngine::new_fixed(4).unwrap();
    assert_eq!(engine.thread_count(), 4);
    assert_eq!(engine.policy(), ParallelizationPolicy::Fixed(4));
}

#[test]
fn test_parallel_engine_auto_tune() {
    let engine = ParallelExecutionEngine::new_auto().unwrap();

    // Small workload should use 1 thread
    assert_eq!(engine.auto_tune(500), 1);

    // Large workload should use multiple threads
    assert!(engine.auto_tune(100_000) > 1);
}

#[test]
fn test_parallel_engine_should_parallelize() {
    let engine = ParallelExecutionEngine::new_auto().unwrap();

    assert!(!engine.should_parallelize(500)); // Too small
    assert!(engine.should_parallelize(100_000)); // Large enough
}

#[test]
fn test_parallel_sql_parser_single() {
    let parser = ParallelSqlParser::new();
    let result = parser.parse("SELECT * FROM users WHERE id = 1");
    assert!(result.is_ok());
    let statements = result.unwrap();
    assert_eq!(statements.len(), 1);
}

#[test]
fn test_parallel_sql_parser_batch() {
    let parser = ParallelSqlParser::new();
    let sqls = vec![
        "SELECT * FROM users",
        "SELECT * FROM orders",
        "SELECT * FROM products",
    ];
    let results = parser.parse_batch(&sqls).unwrap();
    assert_eq!(results.len(), 3);
    for result in results {
        assert_eq!(result.len(), 1);
    }
}

#[test]
fn test_parallel_sql_parser_batch_with_errors() {
    let parser = ParallelSqlParser::new();
    let sqls = vec!["SELECT * FROM users", "INVALID SQL", "SELECT * FROM orders"];
    let result = parser.parse_batch(&sqls);
    assert!(result.is_err());
}

#[test]
fn test_parallel_sql_parser_partial() {
    let parser = ParallelSqlParser::new();
    let sqls = vec!["SELECT * FROM users", "INVALID SQL", "SELECT * FROM orders"];
    let (successes, errors) = parser.parse_batch_partial(&sqls);
    assert_eq!(successes.len(), 2);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 1); // Error at index 1
}

#[test]
fn test_parallel_sql_parser_with_custom_pool() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let parser = ParallelSqlParser::with_thread_pool(Arc::new(pool));
    let sqls = vec!["SELECT * FROM users", "SELECT * FROM orders"];
    let results = parser.parse_batch(&sqls).unwrap();
    assert_eq!(results.len(), 2);
}

// ════════════════════════════════════════════
// Regression Tests: Database Integration
// ════════════════════════════════════════════

#[test]
fn test_database_has_parallel_engine() {
    let db = Database::open_in_memory().unwrap();
    // Verify that the database has a parallel engine
    assert!(db.parallel_engine.thread_count() > 0);
}

#[test]
fn test_database_basic_operations_still_work() {
    let db = Database::open_in_memory().unwrap();

    // INSERT
    db.insert("users", b"user:1", b"Alice").unwrap();

    // GET
    let value = db.get("users", b"user:1").unwrap();
    assert_eq!(value, Some(b"Alice".to_vec()));

    // DELETE
    db.delete("users", b"user:1").unwrap();
    let value = db.get("users", b"user:1").unwrap();
    assert_eq!(value, None);
}

#[test]
fn test_database_sql_operations_still_work() {
    let db = Database::open_in_memory().unwrap();

    // CREATE TABLE
    db.execute_sql("CREATE TABLE users (id INT, name TEXT)")
        .unwrap();

    // INSERT
    db.execute_sql("INSERT INTO users (id, name) VALUES (1, 'Alice')")
        .unwrap();

    // SELECT
    let results = db.execute_sql("SELECT * FROM users").unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_parallel_parser_with_database_sql() {
    let db = Database::open_in_memory().unwrap();
    let parser = ParallelSqlParser::new();

    // Create table
    db.execute_sql("CREATE TABLE users (id INT, name TEXT)")
        .unwrap();

    // Parse multiple INSERT statements in parallel
    let sqls = vec![
        "INSERT INTO users (id, name) VALUES (1, 'Alice')",
        "INSERT INTO users (id, name) VALUES (2, 'Bob')",
        "INSERT INTO users (id, name) VALUES (3, 'Charlie')",
    ];

    let results = parser.parse_batch(&sqls).unwrap();
    assert_eq!(results.len(), 3);

    // Execute each parsed statement
    for statements in results {
        for statement in statements {
            // Verify that the statement can be planned
            use crate::sql::planner::LogicalPlanner;
            let planner = LogicalPlanner::new();
            let plan = planner.plan(&statement);
            assert!(plan.is_ok());
        }
    }
}

#[test]
fn test_parallel_engine_with_database_operations() {
    let db = Database::open_in_memory().unwrap();

    // Use the parallel engine to execute multiple operations
    db.parallel_engine.execute(|| {
        // These operations should work within the parallel context
        db.insert("test", b"key1", b"value1").unwrap();
        db.insert("test", b"key2", b"value2").unwrap();
        db.insert("test", b"key3", b"value3").unwrap();
    });

    // Verify the data was inserted
    assert_eq!(db.get("test", b"key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(db.get("test", b"key2").unwrap(), Some(b"value2".to_vec()));
    assert_eq!(db.get("test", b"key3").unwrap(), Some(b"value3".to_vec()));
}

#[test]
fn test_parallel_batch_insert_performance() {
    let db = Database::open_in_memory().unwrap();
    let parser = ParallelSqlParser::new();

    db.execute_sql("CREATE TABLE test_table (id INT, value TEXT)")
        .unwrap();

    // Generate batch of INSERT statements
    let sqls: Vec<String> = (0..10)
        .map(|i| format!("INSERT INTO test_table (id, value) VALUES ({}, 'value_{}')", i, i))
        .collect();
    let sql_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();

    // Parse in parallel
    let results = parser.parse_batch(&sql_refs).unwrap();
    assert_eq!(results.len(), 10);
}

// ════════════════════════════════════════════
// Edge Cases and Error Handling
// ════════════════════════════════════════════

#[test]
fn test_parallel_engine_zero_threads_error() {
    let result = ParallelExecutionEngine::new_fixed(0);
    assert!(result.is_err());
}

#[test]
fn test_parallel_parser_empty_batch() {
    let parser = ParallelSqlParser::new();
    let sqls: Vec<&str> = vec![];
    let results = parser.parse_batch(&sqls).unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_parallel_parser_single_item_batch() {
    let parser = ParallelSqlParser::new();
    let sqls = vec!["SELECT * FROM users"];
    let results = parser.parse_batch(&sqls).unwrap();
    assert_eq!(results.len(), 1);
}

#[test]
fn test_parallel_parser_callback_error_propagation() {
    let parser = ParallelSqlParser::new();
    let sqls = vec!["SELECT * FROM users", "SELECT * FROM orders"];

    let result = parser.parse_batch_with_callback(&sqls, |idx, _result| {
        if idx == 1 {
            Err(crate::error::DbxError::NotImplemented(
                "Test error".to_string(),
            ))
        } else {
            Ok(())
        }
    });

    assert!(result.is_err());
}
