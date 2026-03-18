//! Latency Histogram — bucket-based latency distribution tracker.
//!
//! Tracks operation latency in microseconds using pre-defined buckets.
//! Thread-safe via `Mutex<HistogramInner>`.

use std::sync::Mutex;

/// Pre-defined upper bounds in microseconds.
/// [10µs, 50µs, 100µs, 500µs, 1ms, 5ms, 10ms, 50ms, 100ms, 500ms, 1s, +Inf]
const BUCKET_BOUNDS_US: &[u64] = &[
    10, 50, 100, 500, 1_000, 5_000, 10_000, 50_000, 100_000, 500_000, 1_000_000,
];

const NUM_BUCKETS: usize = 12; // 11 finite + 1 +Inf

struct HistogramInner {
    /// Cumulative counts per bucket (index 11 = +Inf)
    buckets: [u64; NUM_BUCKETS],
    /// Sum of all observed values (µs)
    sum_us: u64,
    /// Total observations
    count: u64,
}

impl HistogramInner {
    fn new() -> Self {
        Self {
            buckets: [0; NUM_BUCKETS],
            sum_us: 0,
            count: 0,
        }
    }

    fn observe(&mut self, value_us: u64) {
        self.sum_us = self.sum_us.saturating_add(value_us);
        self.count += 1;

        // Insert into all buckets where upper_bound >= value
        for (i, &bound) in BUCKET_BOUNDS_US.iter().enumerate() {
            if value_us <= bound {
                self.buckets[i] += 1;
            }
        }
        // +Inf bucket always gets the observation
        self.buckets[NUM_BUCKETS - 1] += 1;
    }

    fn reset(&mut self) {
        self.buckets = [0; NUM_BUCKETS];
        self.sum_us = 0;
        self.count = 0;
    }
}

/// Thread-safe latency histogram.
pub struct Histogram {
    inner: Mutex<HistogramInner>,
    /// Name used for Prometheus export
    pub name: &'static str,
    /// Help text for Prometheus export
    pub help: &'static str,
}

impl Histogram {
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Self {
            inner: Mutex::new(HistogramInner::new()),
            name,
            help,
        }
    }

    /// Record a latency observation in microseconds.
    pub fn observe(&self, value_us: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.observe(value_us);
        }
    }

    /// Reset all histogram data.
    pub fn reset(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.reset();
        }
    }

    /// Export Prometheus histogram lines.
    pub fn export_prometheus(&self, out: &mut String) {
        let inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return,
        };

        out.push_str(&format!("# HELP {} {}\n", self.name, self.help));
        out.push_str(&format!("# TYPE {} histogram\n", self.name));

        // Cumulative bucket lines
        for (i, &bound) in BUCKET_BOUNDS_US.iter().enumerate() {
            let label = if bound < 1_000 {
                format!("{}µs", bound)
            } else if bound < 1_000_000 {
                format!("{}ms", bound / 1_000)
            } else {
                format!("{}s", bound / 1_000_000)
            };
            out.push_str(&format!(
                "{}_bucket{{le=\"{}\"}} {}\n",
                self.name, label, inner.buckets[i]
            ));
        }
        // +Inf bucket
        out.push_str(&format!(
            "{}_bucket{{le=\"+Inf\"}} {}\n",
            self.name,
            inner.buckets[NUM_BUCKETS - 1]
        ));
        out.push_str(&format!("{}_sum {}\n", self.name, inner.sum_us));
        out.push_str(&format!("{}_count {}\n", self.name, inner.count));
    }

    /// Return (sum_us, count) snapshot.
    pub fn snapshot(&self) -> (u64, u64) {
        self.inner
            .lock()
            .map(|inner| (inner.sum_us, inner.count))
            .unwrap_or((0, 0))
    }

    /// Return bucket counts (excluding +Inf) and total count.
    pub fn bucket_snapshot(&self) -> ([u64; NUM_BUCKETS], u64) {
        self.inner
            .lock()
            .map(|inner| (inner.buckets, inner.count))
            .unwrap_or(([0; NUM_BUCKETS], 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_buckets() {
        let h = Histogram::new("test", "test histogram");
        h.observe(5); // → 10µs bucket
        h.observe(100); // → 100µs bucket
        h.observe(2_000_000); // → +Inf only

        let (buckets, count) = h.bucket_snapshot();
        assert_eq!(count, 3);
        // 10µs bucket should have 1 (only 5µs)
        assert_eq!(buckets[0], 1);
        // +Inf bucket has all 3
        assert_eq!(buckets[NUM_BUCKETS - 1], 3);
    }

    #[test]
    fn test_reset() {
        let h = Histogram::new("test", "test histogram");
        h.observe(100);
        h.reset();
        let (sum, count) = h.snapshot();
        assert_eq!(sum, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_prometheus_export() {
        let h = Histogram::new("dbx_query_latency_us", "Query latency");
        h.observe(80);
        let mut out = String::new();
        h.export_prometheus(&mut out);
        assert!(out.contains("dbx_query_latency_us_count 1"));
        assert!(out.contains("# TYPE dbx_query_latency_us histogram"));
    }
}
