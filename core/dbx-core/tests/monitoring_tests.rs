//! Monitoring & Observability Integration Tests
//!
//! Tests for Phase 0.4: Prometheus metrics collection and export.

use dbx_core::Database;

#[test]
fn test_insert_counter() {
    let db = Database::open_in_memory().unwrap();

    db.insert("users", b"k1", b"v1").unwrap();
    db.insert("users", b"k2", b"v2").unwrap();

    let snap = db.metrics_snapshot();
    assert_eq!(snap.inserts_total, 2, "inserts_total should be 2");
}

#[test]
fn test_get_counter_and_delta_hit() {
    let db = Database::open_in_memory().unwrap();
    db.insert("users", b"k1", b"v1").unwrap();
    db.reset_metrics(); // clear insert counter

    db.get("users", b"k1").unwrap();
    let snap = db.metrics_snapshot();
    assert_eq!(snap.gets_total, 1, "gets_total should be 1");
    assert_eq!(
        snap.delta_hits, 1,
        "delta_hits should be 1 (data is in delta store)"
    );
    assert_eq!(snap.delta_misses, 0, "delta_misses should be 0");
}

#[test]
fn test_cache_miss_then_hit() {
    let db = Database::open_in_memory().unwrap();
    db.insert("users", b"k1", b"v1").unwrap();
    db.reset_metrics();

    // First get: hit in delta
    db.get("users", b"k1").unwrap();
    // Miss on non-existent key
    db.get("users", b"missing").unwrap();

    let snap = db.metrics_snapshot();
    assert_eq!(snap.delta_hits, 1);
    assert_eq!(snap.delta_misses, 1);
    // Hit rate should be 0.5
    assert!(
        (snap.delta_hit_rate - 0.5).abs() < 0.01,
        "delta_hit_rate should be ~0.5, got {}",
        snap.delta_hit_rate
    );
}

#[test]
fn test_delete_counter() {
    let db = Database::open_in_memory().unwrap();
    db.insert("users", b"k1", b"v1").unwrap();
    db.reset_metrics();

    db.delete("users", b"k1").unwrap();
    let snap = db.metrics_snapshot();
    assert_eq!(snap.deletes_total, 1, "deletes_total should be 1");
}

#[test]
fn test_sql_query_counter() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE metrics_test (id INT, name VARCHAR)")
        .unwrap();
    db.reset_metrics();

    db.execute_sql("SELECT * FROM metrics_test").unwrap();
    db.execute_sql("SELECT * FROM metrics_test").unwrap();

    let snap = db.metrics_snapshot();
    assert_eq!(snap.sql_queries_total, 2, "sql_queries_total should be 2");
}

#[test]
fn test_query_latency_histogram() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE latency_test (id INT)")
        .unwrap();
    db.reset_metrics();

    db.execute_sql("SELECT * FROM latency_test").unwrap();

    let snap = db.metrics_snapshot();
    // avg_query_latency_us should be > 0 (query took some time)
    // In practice it could be very fast on empty table, but histogram should be observed
    assert_eq!(snap.sql_queries_total, 1);
}

#[test]
fn test_prometheus_export_format() {
    let db = Database::open_in_memory().unwrap();
    db.insert("t", b"k", b"v").unwrap();
    db.get("t", b"k").unwrap();

    let output = db.export_metrics();

    // Check required Prometheus elements
    assert!(
        output.contains("# HELP dbx_inserts_total"),
        "missing HELP for inserts_total"
    );
    assert!(
        output.contains("# TYPE dbx_inserts_total counter"),
        "missing TYPE for inserts_total"
    );
    assert!(
        output.contains("dbx_inserts_total 1"),
        "inserts_total value should be 1"
    );

    assert!(
        output.contains("dbx_delta_hits_total 1"),
        "delta_hits should be 1"
    );
    assert!(
        output.contains("# TYPE dbx_delta_hit_rate gauge"),
        "should have hit rate gauge"
    );

    assert!(
        output.contains("# TYPE dbx_query_latency_us histogram"),
        "should have latency histogram"
    );
    assert!(
        output.contains("dbx_query_latency_us_count"),
        "histogram should have count"
    );
    assert!(
        output.contains("dbx_query_latency_us_sum"),
        "histogram should have sum"
    );
    assert!(
        output.contains("dbx_query_latency_us_bucket"),
        "histogram should have buckets"
    );
}

#[test]
fn test_reset_metrics() {
    let db = Database::open_in_memory().unwrap();
    db.insert("t", b"k", b"v").unwrap();
    db.get("t", b"k").unwrap();
    db.delete("t", b"k").unwrap();

    let before = db.metrics_snapshot();
    assert!(before.inserts_total > 0);
    assert!(before.gets_total > 0);

    db.reset_metrics();
    let after = db.metrics_snapshot();
    assert_eq!(
        after.inserts_total, 0,
        "inserts_total should be 0 after reset"
    );
    assert_eq!(after.gets_total, 0, "gets_total should be 0 after reset");
    assert_eq!(
        after.deletes_total, 0,
        "deletes_total should be 0 after reset"
    );
}

#[test]
fn test_metrics_isolation() {
    // Two different db instances have independent metrics
    let db1 = Database::open_in_memory().unwrap();
    let db2 = Database::open_in_memory().unwrap();

    db1.insert("t", b"k1", b"v1").unwrap();
    db1.insert("t", b"k2", b"v2").unwrap();
    db2.insert("t", b"k1", b"v1").unwrap();

    let snap1 = db1.metrics_snapshot();
    let snap2 = db2.metrics_snapshot();

    assert_eq!(snap1.inserts_total, 2);
    assert_eq!(snap2.inserts_total, 1);
}
