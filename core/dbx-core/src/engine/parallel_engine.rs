//! Parallel Execution Engine — Rayon-based parallel query execution
//!
//! This module provides a parallel execution engine that leverages Rayon's thread pool
//! to execute queries in parallel, improving performance for large workloads.

use crate::error::{DbxError, DbxResult};
use rayon::ThreadPoolBuilder;
use std::sync::Arc;

/// Parallelization policy for the execution engine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelizationPolicy {
    /// Automatically determine the number of threads based on system resources
    Auto,
    /// Use a fixed number of threads
    Fixed(usize),
    /// Dynamically adjust thread count based on workload
    Adaptive,
}

impl Default for ParallelizationPolicy {
    fn default() -> Self {
        Self::Auto
    }
}

/// Parallel execution engine using Rayon thread pool
pub struct ParallelExecutionEngine {
    thread_pool: Arc<rayon::ThreadPool>,
    policy: ParallelizationPolicy,
}

impl ParallelExecutionEngine {
    /// Create a new parallel execution engine with the specified policy
    pub fn new(policy: ParallelizationPolicy) -> DbxResult<Self> {
        let num_threads = Self::determine_thread_count(policy);
        
        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("dbx-parallel-{}", i))
            .build()
            .map_err(|e| DbxError::NotImplemented(format!("Failed to create thread pool: {}", e)))?;

        Ok(Self {
            thread_pool: Arc::new(thread_pool),
            policy,
        })
    }

    /// Create a new parallel execution engine with automatic thread count
    pub fn new_auto() -> DbxResult<Self> {
        Self::new(ParallelizationPolicy::Auto)
    }

    /// Create a new parallel execution engine with a fixed number of threads
    pub fn new_fixed(num_threads: usize) -> DbxResult<Self> {
        if num_threads == 0 {
            return Err(DbxError::InvalidArguments(
                "Thread count must be greater than 0".to_string(),
            ));
        }
        Self::new(ParallelizationPolicy::Fixed(num_threads))
    }

    /// Get the current parallelization policy
    pub fn policy(&self) -> ParallelizationPolicy {
        self.policy
    }

    /// Get the number of threads in the thread pool
    pub fn thread_count(&self) -> usize {
        self.thread_pool.current_num_threads()
    }

    /// Get a reference to the thread pool
    pub fn thread_pool(&self) -> &rayon::ThreadPool {
        &self.thread_pool
    }

    /// Execute a closure in the thread pool
    pub fn execute<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        self.thread_pool.install(f)
    }

    /// Determine the optimal thread count based on the policy
    fn determine_thread_count(policy: ParallelizationPolicy) -> usize {
        match policy {
            ParallelizationPolicy::Auto => {
                // Use number of logical CPUs, but cap at 16 to avoid overhead
                let num_cpus = num_cpus::get();
                num_cpus.min(16)
            }
            ParallelizationPolicy::Fixed(n) => n,
            ParallelizationPolicy::Adaptive => {
                // For adaptive, start with half of available CPUs
                // This can be adjusted dynamically later
                let num_cpus = num_cpus::get();
                (num_cpus / 2).max(1)
            }
        }
    }

    /// Auto-tune the thread count based on workload size
    ///
    /// Returns the recommended number of parallel tasks for the given workload size
    pub fn auto_tune(&self, workload_size: usize) -> usize {
        self.auto_tune_weighted(workload_size, 1.0)
    }

    /// Auto-tune with complexity weight factor
    ///
    /// Higher complexity = fewer items per thread needed to justify parallelism
    pub fn auto_tune_weighted(&self, workload_size: usize, avg_complexity: f64) -> usize {
        let thread_count = self.thread_count();
        
        match self.policy {
            ParallelizationPolicy::Auto | ParallelizationPolicy::Adaptive => {
                // Base threshold adjusted by complexity
                // Simple queries (complexity ~1.0): need 1000 items per thread
                // Complex queries (complexity ~10.0): need 100 items per thread
                let base_threshold: f64 = 1000.0;
                let adjusted_threshold = (base_threshold / avg_complexity.max(0.1)).max(1.0) as usize;
                
                if workload_size < adjusted_threshold {
                    1
                } else {
                    let optimal = (workload_size / adjusted_threshold).min(thread_count);
                    optimal.max(1)
                }
            }
            ParallelizationPolicy::Fixed(_) => {
                thread_count
            }
        }
    }

    /// Estimate SQL query complexity based on heuristics
    ///
    /// Returns a complexity score (1.0 = simple SELECT, higher = more complex)
    pub fn estimate_query_complexity(sql: &str) -> f64 {
        let sql_upper = sql.to_uppercase();
        let mut score = 1.0;
        
        // JOIN adds complexity
        let join_count = sql_upper.matches("JOIN").count();
        score += join_count as f64 * 2.0;
        
        // Subqueries
        let subquery_depth = sql_upper.matches("SELECT").count().saturating_sub(1);
        score += subquery_depth as f64 * 3.0;
        
        // CTE (WITH)
        if sql_upper.contains("WITH ") {
            score += 4.0;
        }
        
        // UNION
        let union_count = sql_upper.matches("UNION").count();
        score += union_count as f64 * 2.5;
        
        // Aggregate functions
        for func in ["COUNT(", "SUM(", "AVG(", "MAX(", "MIN("] {
            score += sql_upper.matches(func).count() as f64 * 0.5;
        }
        
        // Window functions
        if sql_upper.contains("OVER(") || sql_upper.contains("OVER (") {
            score += 3.0;
        }
        
        // ORDER BY, GROUP BY
        if sql_upper.contains("ORDER BY") { score += 0.5; }
        if sql_upper.contains("GROUP BY") { score += 1.0; }
        if sql_upper.contains("HAVING") { score += 1.0; }
        
        // Query length as proxy for complexity
        score += (sql.len() as f64 / 200.0).min(5.0);
        
        score
    }

    /// Check if parallelization is beneficial for the given workload size
    pub fn should_parallelize(&self, workload_size: usize) -> bool {
        self.auto_tune(workload_size) > 1
    }
}

impl Default for ParallelExecutionEngine {
    fn default() -> Self {
        Self::new_auto().expect("Failed to create default parallel execution engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_auto() {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        assert_eq!(engine.policy(), ParallelizationPolicy::Auto);
        assert!(engine.thread_count() > 0);
    }

    #[test]
    fn test_new_fixed() {
        let engine = ParallelExecutionEngine::new_fixed(4).unwrap();
        assert_eq!(engine.policy(), ParallelizationPolicy::Fixed(4));
        assert_eq!(engine.thread_count(), 4);
    }

    #[test]
    fn test_new_fixed_zero_threads() {
        let result = ParallelExecutionEngine::new_fixed(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute() {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        let result = engine.execute(|| 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_auto_tune_small_workload() {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        let parallelism = engine.auto_tune(500);
        assert_eq!(parallelism, 1); // Too small, should use single thread
    }

    #[test]
    fn test_auto_tune_large_workload() {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        let parallelism = engine.auto_tune(100_000);
        assert!(parallelism > 1); // Large enough for parallelization
    }

    #[test]
    fn test_should_parallelize() {
        let engine = ParallelExecutionEngine::new_auto().unwrap();
        assert!(!engine.should_parallelize(500)); // Too small
        assert!(engine.should_parallelize(100_000)); // Large enough
    }

    #[test]
    fn test_fixed_policy_always_uses_all_threads() {
        let engine = ParallelExecutionEngine::new_fixed(8).unwrap();
        let parallelism = engine.auto_tune(100);
        assert_eq!(parallelism, 8); // Fixed policy always uses all threads
    }
}
