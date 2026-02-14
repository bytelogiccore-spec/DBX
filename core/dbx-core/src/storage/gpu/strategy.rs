//! GPU hash and reduction strategies

use crate::error::{DbxError, DbxResult};

/// GPU Hash Strategy for GROUP BY operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuHashStrategy {
    /// Linear probing - best for small group counts (< 100)
    #[default]
    Linear,
    /// Cuckoo hashing - best for medium group counts (100-1000)
    Cuckoo,
    /// Robin Hood hashing - best for large group counts (> 1000) or large datasets
    RobinHood,
}

impl GpuHashStrategy {
    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> DbxResult<Self> {
        match s.to_lowercase().as_str() {
            "linear" => Ok(GpuHashStrategy::Linear),
            "cuckoo" => Ok(GpuHashStrategy::Cuckoo),
            "robin_hood" | "robinhood" => Ok(GpuHashStrategy::RobinHood),
            _ => Err(DbxError::Gpu(format!(
                "Invalid GPU hash strategy: '{}'. Valid options: linear, cuckoo, robin_hood",
                s
            ))),
        }
    }

    /// Get strategy name
    pub fn as_str(&self) -> &'static str {
        match self {
            GpuHashStrategy::Linear => "linear",
            GpuHashStrategy::Cuckoo => "cuckoo",
            GpuHashStrategy::RobinHood => "robin_hood",
        }
    }
}

/// Reduction strategy for SUM/COUNT operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuReductionStrategy {
    /// Automatically choose based on data size
    #[default]
    Auto,
    /// Single-pass with atomic operations (good for small data)
    SinglePass,
    /// Multi-pass reduction (good for large data, eliminates atomic contention)
    MultiPass,
    /// Histogram-based aggregation (best for low cardinality < 1000)
    Histogram,
}

impl GpuReductionStrategy {
    /// Parse from string (case-insensitive)
    pub fn parse(s: &str) -> DbxResult<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(GpuReductionStrategy::Auto),
            "single" | "single_pass" => Ok(GpuReductionStrategy::SinglePass),
            "multi" | "multi_pass" => Ok(GpuReductionStrategy::MultiPass),
            "histogram" => Ok(GpuReductionStrategy::Histogram),
            _ => Err(DbxError::Gpu(format!(
                "Invalid GPU reduction strategy: '{}'. Valid options: auto, single_pass, multi_pass, histogram",
                s
            ))),
        }
    }

    /// Get strategy name
    pub fn as_str(&self) -> &'static str {
        match self {
            GpuReductionStrategy::Auto => "auto",
            GpuReductionStrategy::SinglePass => "single_pass",
            GpuReductionStrategy::MultiPass => "multi_pass",
            GpuReductionStrategy::Histogram => "histogram",
        }
    }

    /// Choose optimal strategy based on data size
    /// For SUM: single-pass is generally better for current GPU architecture
    /// Multi-pass only beneficial for extremely large datasets (>100M rows)
    pub fn choose_for_sum(&self, data_size: usize) -> GpuReductionStrategy {
        match self {
            GpuReductionStrategy::Auto => {
                // Based on benchmarks: single-pass is better for most cases
                // Only use multi-pass for very large datasets
                if data_size > 100_000_000 {
                    GpuReductionStrategy::MultiPass
                } else {
                    GpuReductionStrategy::SinglePass
                }
            }
            strategy => *strategy,
        }
    }
}
