//! Adaptive algorithm selection for GPU operations

use super::strategy::GpuHashStrategy;

/// Adaptive algorithm selection strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuGroupByStrategy {
    /// Use hash-based GROUP BY (good for low-medium cardinality)
    Hash(GpuHashStrategy),
    /// Use radix sort-based GROUP BY (good for high cardinality)
    RadixSort,
}

impl GpuGroupByStrategy {
    /// Choose optimal GROUP BY strategy based on estimated cardinality
    ///
    /// Heuristics:
    /// - < 100K groups: Use hash-based (Linear/Cuckoo/RobinHood)
    /// - >= 100K groups: Use radix sort
    pub fn choose_for_cardinality(estimated_groups: usize) -> Self {
        const RADIX_SORT_THRESHOLD: usize = 100_000;

        if estimated_groups >= RADIX_SORT_THRESHOLD {
            GpuGroupByStrategy::RadixSort
        } else {
            // For hash-based, choose strategy based on group count
            let chosen_hash = if estimated_groups < 100 {
                GpuHashStrategy::Linear
            } else if estimated_groups < 10_000 {
                GpuHashStrategy::Cuckoo
            } else {
                GpuHashStrategy::RobinHood
            };
            GpuGroupByStrategy::Hash(chosen_hash)
        }
    }

    /// Choose optimal strategy with manual override
    pub fn choose_with_override(estimated_groups: usize, force_radix: bool) -> Self {
        if force_radix {
            GpuGroupByStrategy::RadixSort
        } else {
            Self::choose_for_cardinality(estimated_groups)
        }
    }
}

/// Estimate cardinality (number of unique groups) using HyperLogLog-inspired sampling
/// This is a simplified version for quick estimation
#[allow(dead_code)]
pub fn estimate_cardinality_i32(keys: &[i32]) -> usize {
    use std::collections::HashSet;

    let n = keys.len();

    // For small datasets, just count unique values
    if n <= 10_000 {
        let unique: HashSet<i32> = keys.iter().copied().collect();
        return unique.len();
    }

    // For large datasets, sample and extrapolate
    // Sample 10% or max 100K elements
    let sample_size = (n / 10).min(100_000);
    let step = n / sample_size;

    let mut sample_unique = HashSet::new();
    for i in (0..n).step_by(step) {
        sample_unique.insert(keys[i]);
    }

    // Extrapolate: unique_in_sample / sample_size * total_size
    // Add 20% buffer for estimation error
    let estimated = (sample_unique.len() * n / sample_size) * 12 / 10;
    estimated.min(n) // Can't have more groups than elements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_estimation_exact() {
        // All unique
        let keys: Vec<i32> = (0..1000).collect();
        let est = estimate_cardinality_i32(&keys);
        assert!(est >= 900 && est <= 1100, "Estimated: {}", est);

        // All same
        let keys = vec![42; 1000];
        let est = estimate_cardinality_i32(&keys);
        assert!(est <= 10, "Estimated: {}", est);
    }

    #[test]
    fn test_strategy_selection() {
        // Low cardinality -> Linear hash
        let strategy = GpuGroupByStrategy::choose_for_cardinality(50);
        assert_eq!(strategy, GpuGroupByStrategy::Hash(GpuHashStrategy::Linear));

        // Medium cardinality -> Cuckoo hash
        let strategy = GpuGroupByStrategy::choose_for_cardinality(500);
        assert_eq!(strategy, GpuGroupByStrategy::Hash(GpuHashStrategy::Cuckoo));

        // High cardinality (< 100K) -> RobinHood hash
        let strategy = GpuGroupByStrategy::choose_for_cardinality(50_000);
        assert_eq!(
            strategy,
            GpuGroupByStrategy::Hash(GpuHashStrategy::RobinHood)
        );

        // Very high cardinality -> Radix sort
        let strategy = GpuGroupByStrategy::choose_for_cardinality(200_000);
        assert_eq!(strategy, GpuGroupByStrategy::RadixSort);
    }
}
