//! DbxMetrics — Global atomic metrics registry.
//!
//! All counters use `AtomicU64` for lock-free, zero-overhead tracking.
//! Histograms use `Mutex<HistogramInner>` for bucket tracking.

use crate::monitoring::histogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};

/// Snapshot of current metrics values (non-atomic copy for reporting).
#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    // Operation Counters
    pub inserts_total: u64,
    pub gets_total: u64,
    pub deletes_total: u64,
    pub sql_queries_total: u64,
    pub flush_total: u64,

    // Tier Hit Rates
    pub delta_hits: u64,
    pub delta_misses: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub wos_hits: u64,
    pub wos_misses: u64,

    // Sharding Stats
    pub scatter_writes_total: u64,
    pub scatter_reads_total: u64,

    // Partition Stats
    pub partition_prune_hits: u64,

    // WAL Stats
    pub wal_appends_total: u64,
    pub wal_compactions_total: u64,

    // Latency (avg µs)
    pub avg_query_latency_us: u64,
    pub avg_insert_latency_us: u64,

    // Computed hit rates (0.0 - 1.0)
    pub delta_hit_rate: f64,
    pub cache_hit_rate: f64,
    pub wos_hit_rate: f64,
}

/// Global metrics registry for DBX.
///
/// Embedded in `Database` as `Arc<DbxMetrics>` — cloning the Arc is cheap.
pub struct DbxMetrics {
    // ── Operation Counters ──────────────────────────
    pub inserts_total: AtomicU64,
    pub gets_total: AtomicU64,
    pub deletes_total: AtomicU64,
    pub sql_queries_total: AtomicU64,
    pub flush_total: AtomicU64,

    // ── Tier Hit/Miss Counters ───────────────────────
    pub delta_hits: AtomicU64,
    pub delta_misses: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub wos_hits: AtomicU64,
    pub wos_misses: AtomicU64,

    // ── Sharding Stats ──────────────────────────────
    pub scatter_writes_total: AtomicU64,
    pub scatter_reads_total: AtomicU64,

    // ── Partition Stats ─────────────────────────────
    pub partition_prune_hits: AtomicU64,

    // ── WAL Stats ───────────────────────────────────
    pub wal_appends_total: AtomicU64,
    pub wal_compactions_total: AtomicU64,

    // ── Latency Histograms ──────────────────────────
    pub query_latency_us: Histogram,
    pub insert_latency_us: Histogram,
}

impl DbxMetrics {
    pub fn new() -> Self {
        Self {
            inserts_total: AtomicU64::new(0),
            gets_total: AtomicU64::new(0),
            deletes_total: AtomicU64::new(0),
            sql_queries_total: AtomicU64::new(0),
            flush_total: AtomicU64::new(0),

            delta_hits: AtomicU64::new(0),
            delta_misses: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            wos_hits: AtomicU64::new(0),
            wos_misses: AtomicU64::new(0),

            scatter_writes_total: AtomicU64::new(0),
            scatter_reads_total: AtomicU64::new(0),

            partition_prune_hits: AtomicU64::new(0),

            wal_appends_total: AtomicU64::new(0),
            wal_compactions_total: AtomicU64::new(0),

            query_latency_us: Histogram::new(
                "dbx_query_latency_us",
                "SQL query execution latency in microseconds",
            ),
            insert_latency_us: Histogram::new(
                "dbx_insert_latency_us",
                "INSERT operation latency in microseconds",
            ),
        }
    }

    /// Atomically increment a counter.
    #[inline]
    pub fn inc_inserts(&self) {
        self.inserts_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_gets(&self) {
        self.gets_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_deletes(&self) {
        self.deletes_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_sql_queries(&self) {
        self.sql_queries_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_flush(&self) {
        self.flush_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_delta_hit(&self) {
        self.delta_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_delta_miss(&self) {
        self.delta_misses.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_wos_hit(&self) {
        self.wos_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_wos_miss(&self) {
        self.wos_misses.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_scatter_write(&self) {
        self.scatter_writes_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_scatter_read(&self) {
        self.scatter_reads_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_partition_prune_hit(&self) {
        self.partition_prune_hits.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_wal_append(&self) {
        self.wal_appends_total.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn inc_wal_compaction(&self) {
        self.wal_compactions_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Create a non-atomic snapshot for reporting/display.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let delta_hits = self.delta_hits.load(Ordering::Relaxed);
        let delta_misses = self.delta_misses.load(Ordering::Relaxed);
        let delta_total = delta_hits + delta_misses;

        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let cache_total = cache_hits + cache_misses;

        let wos_hits = self.wos_hits.load(Ordering::Relaxed);
        let wos_misses = self.wos_misses.load(Ordering::Relaxed);
        let wos_total = wos_hits + wos_misses;

        let (q_sum, q_count) = self.query_latency_us.snapshot();
        let (i_sum, i_count) = self.insert_latency_us.snapshot();

        MetricsSnapshot {
            inserts_total: self.inserts_total.load(Ordering::Relaxed),
            gets_total: self.gets_total.load(Ordering::Relaxed),
            deletes_total: self.deletes_total.load(Ordering::Relaxed),
            sql_queries_total: self.sql_queries_total.load(Ordering::Relaxed),
            flush_total: self.flush_total.load(Ordering::Relaxed),

            delta_hits,
            delta_misses,
            cache_hits,
            cache_misses,
            wos_hits,
            wos_misses,

            scatter_writes_total: self.scatter_writes_total.load(Ordering::Relaxed),
            scatter_reads_total: self.scatter_reads_total.load(Ordering::Relaxed),

            partition_prune_hits: self.partition_prune_hits.load(Ordering::Relaxed),

            wal_appends_total: self.wal_appends_total.load(Ordering::Relaxed),
            wal_compactions_total: self.wal_compactions_total.load(Ordering::Relaxed),

            avg_query_latency_us: if q_count > 0 { q_sum / q_count } else { 0 },
            avg_insert_latency_us: if i_count > 0 { i_sum / i_count } else { 0 },

            delta_hit_rate: if delta_total > 0 {
                delta_hits as f64 / delta_total as f64
            } else {
                0.0
            },
            cache_hit_rate: if cache_total > 0 {
                cache_hits as f64 / cache_total as f64
            } else {
                0.0
            },
            wos_hit_rate: if wos_total > 0 {
                wos_hits as f64 / wos_total as f64
            } else {
                0.0
            },
        }
    }

    /// Reset all counters and histograms.
    pub fn reset(&self) {
        self.inserts_total.store(0, Ordering::Relaxed);
        self.gets_total.store(0, Ordering::Relaxed);
        self.deletes_total.store(0, Ordering::Relaxed);
        self.sql_queries_total.store(0, Ordering::Relaxed);
        self.flush_total.store(0, Ordering::Relaxed);
        self.delta_hits.store(0, Ordering::Relaxed);
        self.delta_misses.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.wos_hits.store(0, Ordering::Relaxed);
        self.wos_misses.store(0, Ordering::Relaxed);
        self.scatter_writes_total.store(0, Ordering::Relaxed);
        self.scatter_reads_total.store(0, Ordering::Relaxed);
        self.partition_prune_hits.store(0, Ordering::Relaxed);
        self.wal_appends_total.store(0, Ordering::Relaxed);
        self.wal_compactions_total.store(0, Ordering::Relaxed);
        self.query_latency_us.reset();
        self.insert_latency_us.reset();
    }
}

impl Default for DbxMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inc_counters() {
        let m = DbxMetrics::new();
        m.inc_inserts();
        m.inc_inserts();
        m.inc_gets();
        let snap = m.snapshot();
        assert_eq!(snap.inserts_total, 2);
        assert_eq!(snap.gets_total, 1);
    }

    #[test]
    fn test_hit_rate_calculation() {
        let m = DbxMetrics::new();
        m.inc_delta_hit();
        m.inc_delta_hit();
        m.inc_delta_miss();
        let snap = m.snapshot();
        assert!((snap.delta_hit_rate - 2.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let m = DbxMetrics::new();
        m.inc_inserts();
        m.inc_inserts();
        m.reset();
        let snap = m.snapshot();
        assert_eq!(snap.inserts_total, 0);
    }

    #[test]
    fn test_latency_histogram() {
        let m = DbxMetrics::new();
        m.query_latency_us.observe(500);
        m.query_latency_us.observe(1200);
        let snap = m.snapshot();
        assert_eq!(snap.avg_query_latency_us, 850); // (500 + 1200) / 2
    }
}
