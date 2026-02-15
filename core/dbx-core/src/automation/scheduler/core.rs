//! Scheduler Engine
//!
//! 스케줄된 작업 실행 엔진

use super::job::ScheduledJob;
use crate::automation::ExecutionEngine;
use crate::automation::callable::ExecutionContext;
use crate::error::DbxResult;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// 스케줄러
pub struct Scheduler {
    /// 등록된 작업들
    jobs: RwLock<HashMap<String, ScheduledJob>>,

    /// 실행 엔진 (UDF 호출용)
    execution_engine: Arc<ExecutionEngine>,
}

impl Scheduler {
    /// 새 스케줄러 생성
    pub fn new(execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            execution_engine,
        }
    }

    /// 작업 등록
    pub fn register(&self, job: ScheduledJob) -> DbxResult<()> {
        let name = job.name().to_string();
        let mut jobs = self
            .jobs
            .write()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;

        if jobs.contains_key(&name) {
            return Err(crate::error::DbxError::DuplicateCallable(name));
        }

        jobs.insert(name, job);
        Ok(())
    }

    /// 작업 등록 해제
    pub fn unregister(&self, name: &str) -> DbxResult<()> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;

        jobs.remove(name)
            .ok_or_else(|| crate::error::DbxError::CallableNotFound(name.to_string()))?;

        Ok(())
    }

    /// 실행 준비된 작업 확인 및 실행
    pub fn tick(&self, ctx: &ExecutionContext) -> DbxResult<Vec<String>> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;

        let mut executed = Vec::new();

        for (name, job) in jobs.iter_mut() {
            if job.schedule().is_ready() {
                // UDF 실행
                let result = self
                    .execution_engine
                    .execute(job.callable_name(), ctx, job.args());

                if result.is_ok() {
                    executed.push(name.clone());
                }

                // 스케줄 업데이트
                job.schedule_mut().update();
            }
        }

        Ok(executed)
    }

    /// 등록된 작업 목록
    pub fn list(&self) -> DbxResult<Vec<String>> {
        let jobs = self
            .jobs
            .read()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;

        Ok(jobs.keys().cloned().collect())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new(Arc::new(ExecutionEngine::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::ScalarUDF;
    use crate::automation::callable::{DataType, Signature, Value};
    use crate::automation::scheduler::{Schedule, ScheduleType};
    use crate::engine::Database;
    use std::time::Duration;

    #[test]
    fn test_scheduler_register() {
        let scheduler = Scheduler::default();

        let schedule = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let job = ScheduledJob::new("test_job", schedule, "test_udf", vec![]);

        scheduler.register(job).unwrap();

        let jobs = scheduler.list().unwrap();
        assert_eq!(jobs.len(), 1);
        assert!(jobs.contains(&"test_job".to_string()));
    }

    #[test]
    fn test_scheduler_duplicate() {
        let scheduler = Scheduler::default();

        let schedule1 = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let job1 = ScheduledJob::new("test_job", schedule1, "test_udf", vec![]);

        let schedule2 = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let job2 = ScheduledJob::new("test_job", schedule2, "test_udf", vec![]);

        scheduler.register(job1).unwrap();
        let result = scheduler.register(job2);
        assert!(result.is_err());
    }

    #[test]
    fn test_scheduler_tick() {
        let engine = Arc::new(ExecutionEngine::new());

        // UDF 등록
        let udf = Arc::new(ScalarUDF::new(
            "test_udf",
            Signature {
                params: vec![],
                return_type: DataType::Null,
                is_variadic: false,
            },
            |_args| Ok(Value::Null),
        ));
        engine.register(udf).unwrap();

        let scheduler = Scheduler::new(engine);

        // 즉시 실행되는 작업 등록
        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(0)));
        let job = ScheduledJob::new("test_job", schedule, "test_udf", vec![]);
        scheduler.register(job).unwrap();

        // 실행
        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // 약간의 지연 후 tick
        std::thread::sleep(Duration::from_millis(100));
        let executed = scheduler.tick(&ctx).unwrap();

        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0], "test_job");
    }
}
