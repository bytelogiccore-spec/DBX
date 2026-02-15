//! Parallel SQL Parser — Batch SQL parsing using Rayon
//!
//! This module provides parallel SQL parsing capabilities to process multiple
//! SQL statements concurrently, improving throughput for batch operations.

use crate::error::{DbxError, DbxResult};
use rayon::prelude::*;
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::sync::Arc;

/// Parallel SQL parser for batch processing
pub struct ParallelSqlParser {
    dialect: GenericDialect,
    thread_pool: Option<Arc<rayon::ThreadPool>>,
}

impl ParallelSqlParser {
    /// Create a new parallel SQL parser
    pub fn new() -> Self {
        Self {
            dialect: GenericDialect {},
            thread_pool: None,
        }
    }

    /// Create a new parallel SQL parser with a custom thread pool
    pub fn with_thread_pool(thread_pool: Arc<rayon::ThreadPool>) -> Self {
        Self {
            dialect: GenericDialect {},
            thread_pool: Some(thread_pool),
        }
    }

    /// Parse a single SQL string into AST
    pub fn parse(&self, sql: &str) -> DbxResult<Vec<Statement>> {
        Parser::parse_sql(&self.dialect, sql).map_err(|e| DbxError::SqlParse {
            message: e.to_string(),
            sql: sql.to_string(),
        })
    }

    /// Parse multiple SQL strings in parallel with optimized scheduling
    ///
    /// Applies three optimization layers:
    /// 1. Dynamic thread pool: adjusts parallelism based on workload complexity
    /// 2. Adaptive batch splitting: distributes work by estimated query complexity
    /// 3. Lock-free result collection: pre-allocated indexed output
    ///
    /// # Arguments
    ///
    /// * `sqls` - A slice of SQL strings to parse
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::sql::parallel_parser::ParallelSqlParser;
    ///
    /// let parser = ParallelSqlParser::new();
    /// let sqls = vec![
    ///     "SELECT * FROM users",
    ///     "SELECT * FROM orders",
    ///     "SELECT * FROM products",
    /// ];
    /// let results = parser.parse_batch(&sqls).unwrap();
    /// assert_eq!(results.len(), 3);
    /// ```
    pub fn parse_batch(&self, sqls: &[&str]) -> DbxResult<Vec<Vec<Statement>>> {
        let len = sqls.len();
        if len == 0 {
            return Ok(Vec::new());
        }

        // Fast-path: small batches always sequential (no complexity estimation overhead)
        if len < 4 {
            return sqls
                .iter()
                .map(|sql| self.parse(sql))
                .collect::<DbxResult<Vec<_>>>();
        }

        // For medium+ batches, sample complexity to decide parallelism strategy
        let avg_complexity = if len <= 20 {
            sqls.iter()
                .map(|s| Self::estimate_complexity(s))
                .sum::<f64>()
                / len as f64
        } else {
            // Sample first 10 for speed
            sqls.iter()
                .take(10)
                .map(|s| Self::estimate_complexity(s))
                .sum::<f64>()
                / 10.0
        };

        // Dynamic threshold
        let parallel_threshold = if avg_complexity > 5.0 {
            4
        } else if avg_complexity > 2.0 {
            8
        } else {
            16 // Simple queries: only parallelize large batches
        };

        if len < parallel_threshold {
            return sqls
                .iter()
                .map(|sql| self.parse(sql))
                .collect::<DbxResult<Vec<_>>>();
        }

        // Parallel execution
        let results: Vec<Option<DbxResult<Vec<Statement>>>> = if let Some(pool) = &self.thread_pool
        {
            pool.install(|| self.parallel_parse_adaptive(sqls, avg_complexity))
        } else {
            self.parallel_parse_adaptive(sqls, avg_complexity)
        };

        results
            .into_iter()
            .map(|opt| {
                opt.unwrap_or_else(|| {
                    Err(DbxError::SqlParse {
                        message: "Missing parse result".to_string(),
                        sql: String::new(),
                    })
                })
            })
            .collect()
    }

    /// Adaptive parallel parsing with weighted work distribution
    fn parallel_parse_adaptive(
        &self,
        sqls: &[&str],
        avg_complexity: f64,
    ) -> Vec<Option<DbxResult<Vec<Statement>>>> {
        let len = sqls.len();

        if avg_complexity > 5.0 {
            // High complexity: use work-stealing with fine-grained tasks
            // Each query is its own task — Rayon's work-stealing handles load balancing
            sqls.par_iter().map(|sql| Some(self.parse(sql))).collect()
        } else {
            // Low/medium complexity: chunk-based parallelism to reduce scheduling overhead
            let num_threads = rayon::current_num_threads();
            let chunk_size = (len / num_threads).max(1);

            // Pre-allocate result slots
            let mut results: Vec<Option<DbxResult<Vec<Statement>>>> = Vec::with_capacity(len);
            results.resize_with(len, || None);

            // Use parallel chunks with index tracking
            let chunk_results: Vec<(usize, Vec<DbxResult<Vec<Statement>>>)> = sqls
                .par_chunks(chunk_size)
                .enumerate()
                .map(|(chunk_idx, chunk)| {
                    let start_idx = chunk_idx * chunk_size;
                    let parsed: Vec<DbxResult<Vec<Statement>>> =
                        chunk.iter().map(|sql| self.parse(sql)).collect();
                    (start_idx, parsed)
                })
                .collect();

            // Merge results into pre-allocated slots (single-threaded, no locks needed)
            for (start_idx, chunk_results_vec) in chunk_results {
                for (offset, result) in chunk_results_vec.into_iter().enumerate() {
                    if start_idx + offset < len {
                        results[start_idx + offset] = Some(result);
                    }
                }
            }

            results
        }
    }

    /// Fast complexity estimation using byte-level scanning (zero allocation)
    fn estimate_complexity(sql: &str) -> f64 {
        let bytes = sql.as_bytes();
        let len = bytes.len();
        let mut score = 1.0;

        // Byte-level case-insensitive keyword counting
        score += Self::count_keyword_ci(bytes, b"JOIN") as f64 * 2.0;
        let select_count = Self::count_keyword_ci(bytes, b"SELECT");
        score += select_count.saturating_sub(1) as f64 * 3.0;
        if Self::contains_keyword_ci(bytes, b"WITH ") {
            score += 4.0;
        }
        score += Self::count_keyword_ci(bytes, b"UNION") as f64 * 2.5;

        // Length as proxy
        score += (len as f64 / 200.0).min(5.0);
        score
    }

    /// Count occurrences of keyword (case-insensitive, ASCII only)
    #[inline]
    fn count_keyword_ci(haystack: &[u8], needle: &[u8]) -> usize {
        if needle.len() > haystack.len() {
            return 0;
        }
        let mut count = 0;
        for i in 0..=(haystack.len() - needle.len()) {
            if haystack[i..i + needle.len()]
                .iter()
                .zip(needle.iter())
                .all(|(h, n)| h.to_ascii_uppercase() == *n)
            {
                count += 1;
            }
        }
        count
    }

    /// Check if keyword exists (case-insensitive, ASCII only)
    #[inline]
    fn contains_keyword_ci(haystack: &[u8], needle: &[u8]) -> bool {
        Self::count_keyword_ci(haystack, needle) > 0
    }

    /// Parse multiple SQL strings in parallel, collecting only successful results
    ///
    /// Returns a vector of successful parse results and a vector of errors.
    /// This is useful when you want to continue processing even if some SQL strings fail.
    ///
    /// # Arguments
    ///
    /// * `sqls` - A slice of SQL strings to parse
    ///
    /// # Returns
    ///
    /// A tuple of (successful_results, errors)
    pub fn parse_batch_partial(
        &self,
        sqls: &[&str],
    ) -> (Vec<Vec<Statement>>, Vec<(usize, DbxError)>) {
        let results = if let Some(pool) = &self.thread_pool {
            pool.install(|| {
                sqls.par_iter()
                    .enumerate()
                    .map(|(idx, sql)| (idx, self.parse(sql)))
                    .collect::<Vec<_>>()
            })
        } else {
            sqls.par_iter()
                .enumerate()
                .map(|(idx, sql)| (idx, self.parse(sql)))
                .collect::<Vec<_>>()
        };

        let mut successes = Vec::new();
        let mut errors = Vec::new();

        for (idx, result) in results {
            match result {
                Ok(statements) => successes.push(statements),
                Err(e) => errors.push((idx, e)),
            }
        }

        (successes, errors)
    }

    /// Parse multiple SQL strings and execute a callback for each result
    ///
    /// This is useful for streaming processing where you want to handle each
    /// result as it becomes available.
    ///
    /// # Arguments
    ///
    /// * `sqls` - A slice of SQL strings to parse
    /// * `callback` - A function to call for each parse result
    pub fn parse_batch_with_callback<F>(&self, sqls: &[&str], mut callback: F) -> DbxResult<()>
    where
        F: FnMut(usize, DbxResult<Vec<Statement>>) -> DbxResult<()>,
    {
        let results = if let Some(pool) = &self.thread_pool {
            pool.install(|| {
                sqls.par_iter()
                    .enumerate()
                    .map(|(idx, sql)| (idx, self.parse(sql)))
                    .collect::<Vec<_>>()
            })
        } else {
            sqls.par_iter()
                .enumerate()
                .map(|(idx, sql)| (idx, self.parse(sql)))
                .collect::<Vec<_>>()
        };

        for (idx, result) in results {
            callback(idx, result)?;
        }

        Ok(())
    }
}

impl Default for ParallelSqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single() {
        let parser = ParallelSqlParser::new();
        let result = parser.parse("SELECT * FROM users");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_parse_batch_small() {
        let parser = ParallelSqlParser::new();
        let sqls = vec!["SELECT * FROM users", "SELECT * FROM orders"];
        let results = parser.parse_batch(&sqls).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].len(), 1);
        assert_eq!(results[1].len(), 1);
    }

    #[test]
    fn test_parse_batch_large() {
        let parser = ParallelSqlParser::new();
        let sqls = vec![
            "SELECT * FROM users",
            "SELECT * FROM orders",
            "SELECT * FROM products",
            "SELECT * FROM categories",
            "SELECT * FROM reviews",
        ];
        let results = parser.parse_batch(&sqls).unwrap();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_parse_batch_with_error() {
        let parser = ParallelSqlParser::new();
        let sqls = vec!["SELECT * FROM users", "INVALID SQL", "SELECT * FROM orders"];
        let result = parser.parse_batch(&sqls);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_batch_partial() {
        let parser = ParallelSqlParser::new();
        let sqls = vec!["SELECT * FROM users", "INVALID SQL", "SELECT * FROM orders"];
        let (successes, errors) = parser.parse_batch_partial(&sqls);
        assert_eq!(successes.len(), 2);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 1); // Error at index 1
    }

    #[test]
    fn test_parse_batch_with_callback() {
        let parser = ParallelSqlParser::new();
        let sqls = vec!["SELECT * FROM users", "SELECT * FROM orders"];
        let mut count = 0;
        parser
            .parse_batch_with_callback(&sqls, |_idx, result| {
                assert!(result.is_ok());
                count += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_with_custom_thread_pool() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap();
        let parser = ParallelSqlParser::with_thread_pool(Arc::new(pool));
        let sqls = vec![
            "SELECT * FROM users",
            "SELECT * FROM orders",
            "SELECT * FROM products",
        ];
        let results = parser.parse_batch(&sqls).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_parse_multiple_statements() {
        let parser = ParallelSqlParser::new();
        let result = parser.parse("SELECT * FROM users; SELECT * FROM orders;");
        assert!(result.is_ok());
        let statements = result.unwrap();
        assert_eq!(statements.len(), 2);
    }
}
