//! Execution engine for callable objects
//!
//! 통합 실행 엔진으로 UDF, 트리거, 스케줄 작업을 실행합니다.

use super::callable::{Callable, ExecutionContext, Value};
use crate::error::{DbxError, DbxResult};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// 통합 실행 엔진
pub struct ExecutionEngine {
    /// 등록된 callable 객체들
    callables: RwLock<HashMap<String, Arc<dyn Callable>>>,
    /// 메트릭 수집기
    metrics: Arc<RwLock<ExecutionMetrics>>,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        Self {
            callables: RwLock::new(HashMap::new()),
            metrics: Arc::new(RwLock::new(ExecutionMetrics::new())),
        }
    }

    /// Callable 등록
    pub fn register(&self, callable: Arc<dyn Callable>) -> DbxResult<()> {
        let name = callable.name().to_string();
        let mut callables = self.callables.write().map_err(|_| DbxError::LockPoisoned)?;

        if callables.contains_key(&name) {
            return Err(DbxError::DuplicateCallable(name));
        }

        callables.insert(name, callable);
        Ok(())
    }

    /// Callable 등록 해제
    pub fn unregister(&self, name: &str) -> DbxResult<()> {
        let mut callables = self.callables.write().map_err(|_| DbxError::LockPoisoned)?;

        callables
            .remove(name)
            .ok_or_else(|| DbxError::CallableNotFound(name.to_string()))?;

        Ok(())
    }

    /// Callable 실행
    pub fn execute(&self, name: &str, ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
        // Callable 조회
        let callables = self.callables.read().map_err(|_| DbxError::LockPoisoned)?;

        let callable = callables
            .get(name)
            .ok_or_else(|| DbxError::CallableNotFound(name.to_string()))?
            .clone();

        drop(callables); // 락 해제

        // 메트릭 시작
        let start = Instant::now();

        // 실행
        let result = callable.call(ctx, args);

        // 메트릭 기록
        let elapsed = start.elapsed();
        let success = result.is_ok();

        if let Ok(mut metrics) = self.metrics.write() {
            metrics.record(name, elapsed, success);
        }

        result
    }

    /// 등록된 callable 목록
    pub fn list(&self) -> DbxResult<Vec<String>> {
        let callables = self.callables.read().map_err(|_| DbxError::LockPoisoned)?;

        Ok(callables.keys().cloned().collect())
    }

    /// 메트릭 조회
    pub fn metrics(&self) -> DbxResult<ExecutionMetrics> {
        let metrics = self.metrics.read().map_err(|_| DbxError::LockPoisoned)?;

        Ok(metrics.clone())
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 실행 메트릭
#[derive(Debug, Clone)]
pub struct ExecutionMetrics {
    /// 함수별 호출 횟수
    pub call_counts: HashMap<String, u64>,
    /// 함수별 총 실행 시간
    pub total_durations: HashMap<String, Duration>,
    /// 함수별 실패 횟수
    pub error_counts: HashMap<String, u64>,
}

impl ExecutionMetrics {
    pub fn new() -> Self {
        Self {
            call_counts: HashMap::new(),
            total_durations: HashMap::new(),
            error_counts: HashMap::new(),
        }
    }

    /// 메트릭 기록
    pub fn record(&mut self, name: &str, duration: Duration, success: bool) {
        // 호출 횟수
        *self.call_counts.entry(name.to_string()).or_insert(0) += 1;

        // 실행 시간
        *self
            .total_durations
            .entry(name.to_string())
            .or_insert(Duration::ZERO) += duration;

        // 실패 횟수
        if !success {
            *self.error_counts.entry(name.to_string()).or_insert(0) += 1;
        }
    }

    /// 평균 실행 시간
    pub fn avg_duration(&self, name: &str) -> Option<Duration> {
        let total = self.total_durations.get(name)?;
        let count = self.call_counts.get(name)?;

        if *count == 0 {
            return None;
        }

        Some(*total / (*count as u32))
    }

    /// 성공률
    pub fn success_rate(&self, name: &str) -> Option<f64> {
        let total = *self.call_counts.get(name)?;
        let errors = self.error_counts.get(name).copied().unwrap_or(0);

        if total == 0 {
            return None;
        }

        Some((total - errors) as f64 / total as f64)
    }
}

impl Default for ExecutionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::callable::{DataType, Signature};

    struct TestCallable {
        name: String,
        signature: Signature,
    }

    impl TestCallable {
        fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                signature: Signature {
                    params: vec![DataType::Int],
                    return_type: DataType::Int,
                    is_variadic: false,
                },
            }
        }
    }

    impl Callable for TestCallable {
        fn call(&self, _ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
            // 단순히 첫 번째 인자를 반환
            Ok(args.get(0).cloned().unwrap_or(Value::Null))
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn signature(&self) -> &Signature {
            &self.signature
        }
    }

    #[test]
    fn test_register_and_execute() {
        let engine = ExecutionEngine::new();
        let callable = Arc::new(TestCallable::new("test_func"));

        // 등록
        engine.register(callable).unwrap();

        // 실행
        let ctx =
            ExecutionContext::new(Arc::new(crate::engine::Database::open_in_memory().unwrap()));
        let result = engine
            .execute("test_func", &ctx, &[Value::Int(42)])
            .unwrap();

        assert_eq!(result.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_duplicate_registration() {
        let engine = ExecutionEngine::new();
        let callable1 = Arc::new(TestCallable::new("test_func"));
        let callable2 = Arc::new(TestCallable::new("test_func"));

        engine.register(callable1).unwrap();
        let result = engine.register(callable2);

        assert!(result.is_err());
    }

    #[test]
    fn test_metrics() {
        let engine = ExecutionEngine::new();
        let callable = Arc::new(TestCallable::new("test_func"));

        engine.register(callable).unwrap();

        let ctx =
            ExecutionContext::new(Arc::new(crate::engine::Database::open_in_memory().unwrap()));

        // 여러 번 실행
        for _ in 0..10 {
            let _ = engine.execute("test_func", &ctx, &[Value::Int(42)]);
        }

        let metrics = engine.metrics().unwrap();
        assert_eq!(metrics.call_counts.get("test_func"), Some(&10));
    }
}
