//! Prometheus Exposition Format Exporter
//!
//! Converts `DbxMetrics` to Prometheus text format (version 0.0.4).
//! See: https://prometheus.io/docs/instrumenting/exposition_formats/

use crate::monitoring::metrics::DbxMetrics;

/// Export all DBX metrics in Prometheus text exposition format.
///
/// # Returns
/// A `String` in Prometheus text format, ready to serve at `/metrics`.
///
/// # Example output
/// ```text
/// # HELP dbx_inserts_total Total INSERT operations
/// # TYPE dbx_inserts_total counter
/// dbx_inserts_total 1234
/// ```
pub fn export_prometheus(metrics: &DbxMetrics) -> String {
    let mut out = String::with_capacity(4096);
    let snap = metrics.snapshot();

    // ── Operation Counters ──────────────────────────────────────────────
    append_counter(
        &mut out,
        "dbx_inserts_total",
        snap.inserts_total,
        "Total number of INSERT operations",
    );
    append_counter(
        &mut out,
        "dbx_gets_total",
        snap.gets_total,
        "Total number of GET operations",
    );
    append_counter(
        &mut out,
        "dbx_deletes_total",
        snap.deletes_total,
        "Total number of DELETE operations",
    );
    append_counter(
        &mut out,
        "dbx_sql_queries_total",
        snap.sql_queries_total,
        "Total number of SQL queries executed",
    );
    append_counter(
        &mut out,
        "dbx_flush_total",
        snap.flush_total,
        "Total number of Delta Store flush operations",
    );

    // ── Tier Hit/Miss Counters ───────────────────────────────────────────
    append_counter(
        &mut out,
        "dbx_delta_hits_total",
        snap.delta_hits,
        "Cache hits in Tier 1 (Delta Store)",
    );
    append_counter(
        &mut out,
        "dbx_delta_misses_total",
        snap.delta_misses,
        "Cache misses in Tier 1 (Delta Store)",
    );
    append_gauge_f64(
        &mut out,
        "dbx_delta_hit_rate",
        snap.delta_hit_rate,
        "Hit rate of Tier 1 Delta Store (0.0 - 1.0)",
    );

    append_counter(
        &mut out,
        "dbx_cache_hits_total",
        snap.cache_hits,
        "Cache hits in Tier 2 (Columnar Cache)",
    );
    append_counter(
        &mut out,
        "dbx_cache_misses_total",
        snap.cache_misses,
        "Cache misses in Tier 2 (Columnar Cache)",
    );
    append_gauge_f64(
        &mut out,
        "dbx_cache_hit_rate",
        snap.cache_hit_rate,
        "Hit rate of Tier 2 Columnar Cache (0.0 - 1.0)",
    );

    append_counter(
        &mut out,
        "dbx_wos_hits_total",
        snap.wos_hits,
        "Cache hits in Tier 3 (Write-Optimized Store)",
    );
    append_counter(
        &mut out,
        "dbx_wos_misses_total",
        snap.wos_misses,
        "Cache misses in Tier 3 (Write-Optimized Store)",
    );
    append_gauge_f64(
        &mut out,
        "dbx_wos_hit_rate",
        snap.wos_hit_rate,
        "Hit rate of Tier 3 WOS (0.0 - 1.0)",
    );

    // ── Sharding Stats ──────────────────────────────────────────────────
    append_counter(
        &mut out,
        "dbx_scatter_writes_total",
        snap.scatter_writes_total,
        "Total number of scatter write operations (sharding)",
    );
    append_counter(
        &mut out,
        "dbx_scatter_reads_total",
        snap.scatter_reads_total,
        "Total number of scatter read operations (sharding)",
    );

    // ── Partition Stats ─────────────────────────────────────────────────
    append_counter(
        &mut out,
        "dbx_partition_prune_hits_total",
        snap.partition_prune_hits,
        "Total number of partition pruning hits",
    );

    // ── WAL Stats ───────────────────────────────────────────────────────
    append_counter(
        &mut out,
        "dbx_wal_appends_total",
        snap.wal_appends_total,
        "Total number of WAL append operations",
    );
    append_counter(
        &mut out,
        "dbx_wal_compactions_total",
        snap.wal_compactions_total,
        "Total number of WAL compaction operations",
    );

    // ── Latency Gauges (avg µs) ─────────────────────────────────────────
    append_gauge(
        &mut out,
        "dbx_avg_query_latency_us",
        snap.avg_query_latency_us,
        "Average SQL query latency in microseconds",
    );
    append_gauge(
        &mut out,
        "dbx_avg_insert_latency_us",
        snap.avg_insert_latency_us,
        "Average INSERT latency in microseconds",
    );

    // ── Latency Histograms ──────────────────────────────────────────────
    metrics.query_latency_us.export_prometheus(&mut out);
    metrics.insert_latency_us.export_prometheus(&mut out);

    out
}

fn append_counter(out: &mut String, name: &str, value: u64, help: &str) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} counter\n"));
    out.push_str(&format!("{name} {value}\n\n"));
}

fn append_gauge(out: &mut String, name: &str, value: u64, help: &str) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} gauge\n"));
    out.push_str(&format!("{name} {value}\n\n"));
}

fn append_gauge_f64(out: &mut String, name: &str, value: f64, help: &str) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} gauge\n"));
    out.push_str(&format!("{name} {:.4}\n\n", value));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prometheus_format() {
        let m = DbxMetrics::new();
        m.inc_inserts();
        m.inc_inserts();
        m.inc_sql_queries();
        m.query_latency_us.observe(500);

        let output = export_prometheus(&m);

        assert!(
            output.contains("dbx_inserts_total 2"),
            "inserts_total should be 2"
        );
        assert!(
            output.contains("dbx_sql_queries_total 1"),
            "sql_queries_total should be 1"
        );
        assert!(output.contains("# TYPE dbx_inserts_total counter"));
        assert!(output.contains("# TYPE dbx_cache_hit_rate gauge"));
        assert!(output.contains("# TYPE dbx_query_latency_us histogram"));
        assert!(output.contains("dbx_query_latency_us_count 1"));
    }

    #[test]
    fn test_hit_rate_export() {
        let m = DbxMetrics::new();
        m.inc_cache_hit();
        m.inc_cache_hit();
        m.inc_cache_miss();

        let output = export_prometheus(&m);
        // Hit rate should be ~0.6667
        assert!(output.contains("dbx_cache_hit_rate 0.6667"));
    }
}
