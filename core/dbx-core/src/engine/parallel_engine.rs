//! Parallel Execution Engine — Rayon-based parallel query execution
//!
//! This module provides a parallel execution engine that leverages Rayon's thread pool
//! to execute queries in parallel, improving performance for large workloads.

use crate::error::{DbxError, DbxResult};
use rayon::ThreadPoolBuilder;
use std::sync::Arc;

/// Parallelization policy for the execution engine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParallelizationPolicy {
    /// Automatically determine the number of threads based on system resources
    #[default]
    Auto,
    /// Use a fixed number of threads
    Fixed(usize),
    /// Dynamically adjust thread count based on workload
    Adaptive,
}

/// 병렬 처리 설정 — CPU 사용량 상한 및 임계값 제어
///
/// `Database::open_with_config()` 또는 `DbConfig`를 통해 전달합니다.
///
/// # 예시
/// ```rust
/// use dbx_core::engine::parallel_engine::ParallelismConfig;
/// let config = ParallelismConfig {
///     cpu_cap: 0.5,               // CPU 50%까지만 사용
///     min_rows_for_parallel: 5000, // 5000행 이상에서만 병렬화
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ParallelismConfig {
    /// 최대 CPU 코어 사용 비율 (0.0 초과 ~ 1.0 이하).
    /// 예: 0.5 → 8코어 머신에서 최대 4개 스레드.
    /// 기본값: 1.0 (모든 코어 허용, 단 최대 16개 캡)
    pub cpu_cap: f64,
    /// 병렬 처리 최소 행 수. 이 미만이면 단일 스레드 실행.
    /// 기본값: 1000
    pub min_rows_for_parallel: usize,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        Self {
            cpu_cap: 1.0,
            min_rows_for_parallel: 1_000,
        }
    }
}

impl ParallelismConfig {
    /// PC를 여유롭게 사용하는 보수적 설정 (CPU 50%, 임계값 5000행)
    pub fn conservative() -> Self {
        Self {
            cpu_cap: 0.5,
            min_rows_for_parallel: 5_000,
        }
    }

    /// 모든 코어를 최대로 활용하는 공격적 설정
    pub fn aggressive() -> Self {
        Self {
            cpu_cap: 1.0,
            min_rows_for_parallel: 500,
        }
    }
}

/// Parallel execution engine using Rayon thread pool
pub struct ParallelExecutionEngine {
    thread_pool: Arc<rayon::ThreadPool>,
    policy: ParallelizationPolicy,
    /// 전체 설정 (병렬 및 실시간 동기화)
    pub db_config: DbConfig,
}

impl ParallelExecutionEngine {
    /// Create a new parallel execution engine with the specified policy
    pub fn new(policy: ParallelizationPolicy) -> DbxResult<Self> {
        Self::new_with_config(policy, DbConfig::default())
    }

    /// Create a new parallel execution engine with policy and parallelism config.
    ///
    /// `config.parallelism.cpu_cap`에 따라 스레드 수를 제한합니다.
    pub fn new_with_config(policy: ParallelizationPolicy, config: DbConfig) -> DbxResult<Self> {
        let cpu_cap = config.parallelism.cpu_cap.clamp(0.01, 1.0);
        let base_threads = Self::determine_thread_count(policy);
        let capped_threads = ((base_threads as f64 * cpu_cap).ceil() as usize).max(1);

        let thread_pool = ThreadPoolBuilder::new()
            .num_threads(capped_threads)
            .thread_name(|i| format!("dbx-parallel-{}", i))
            .build()
            .map_err(|e| {
                DbxError::NotImplemented(format!("Failed to create thread pool: {}", e))
            })?;

        Ok(Self {
            thread_pool: Arc::new(thread_pool),
            policy,
            db_config: config,
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

    /// Get the parallelism config
    pub fn config(&self) -> &ParallelismConfig {
        &self.db_config.parallelism
    }

    /// Get the full database config
    pub fn db_config(&self) -> &DbConfig {
        &self.db_config
    }

    /// Check if parallelization is beneficial given a row count,
    /// respecting `min_rows_for_parallel` from the config.
    pub fn should_parallelize_rows(&self, row_count: usize) -> bool {
        row_count >= self.db_config.parallelism.min_rows_for_parallel && self.thread_count() > 1
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
                let adjusted_threshold =
                    (base_threshold / avg_complexity.max(0.1)).max(1.0) as usize;

                if workload_size < adjusted_threshold {
                    1
                } else {
                    let optimal = (workload_size / adjusted_threshold).min(thread_count);
                    optimal.max(1)
                }
            }
            ParallelizationPolicy::Fixed(_) => thread_count,
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
        if sql_upper.contains("ORDER BY") {
            score += 0.5;
        }
        if sql_upper.contains("GROUP BY") {
            score += 1.0;
        }
        if sql_upper.contains("HAVING") {
            score += 1.0;
        }

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

use crate::storage::realtime_sync::RealtimeSyncConfig;

/// 데이터베이스 전체 설정
///
/// `Database::open_with_config()`에 전달하여 병렬 처리 등을 제어합니다.
///
/// # 예시
/// ```rust
/// use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig};
/// let config = DbConfig {
///     parallelism: ParallelismConfig::conservative(),
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct DbConfig {
    /// 병렬 처리 설정
    pub parallelism: ParallelismConfig,
    /// HTAP 실시간 동기화 설정 (기본은 Threshold(10,000))
    pub sync: RealtimeSyncConfig,
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
