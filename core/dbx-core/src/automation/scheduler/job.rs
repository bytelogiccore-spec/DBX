//! Scheduled Job
//!
//! 스케줄된 작업 정의

use super::schedule::Schedule;
use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// 스케줄된 작업
pub struct ScheduledJob {
    name: String,
    schedule: Schedule,
    callable_name: String,
    args: Vec<Value>,
    signature: Signature,
}

impl ScheduledJob {
    /// 새 스케줄 작업 생성
    pub fn new(
        name: impl Into<String>,
        schedule: Schedule,
        callable_name: impl Into<String>,
        args: Vec<Value>,
    ) -> Self {
        Self {
            name: name.into(),
            schedule,
            callable_name: callable_name.into(),
            args,
            signature: Signature {
                params: vec![],
                return_type: crate::automation::callable::DataType::Null,
                is_variadic: false,
            },
        }
    }
    
    /// 작업 이름
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// 스케줄
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }
    
    /// 스케줄 (가변)
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }
    
    /// 호출할 Callable 이름
    pub fn callable_name(&self) -> &str {
        &self.callable_name
    }
    
    /// 인자
    pub fn args(&self) -> &[Value] {
        &self.args
    }
}

impl Callable for ScheduledJob {
    fn call(&self, _ctx: &ExecutionContext, _args: &[Value]) -> DbxResult<Value> {
        // ScheduledJob은 직접 호출되지 않음
        // Scheduler가 callable_name을 사용하여 실제 UDF를 호출
        Ok(Value::Null)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn signature(&self) -> &Signature {
        &self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::scheduler::ScheduleType;
    use std::time::Duration;
    
    #[test]
    fn test_scheduled_job_creation() {
        let schedule = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let job = ScheduledJob::new(
            "test_job",
            schedule,
            "test_udf",
            vec![Value::Int(42)],
        );
        
        assert_eq!(job.name(), "test_job");
        assert_eq!(job.callable_name(), "test_udf");
        assert_eq!(job.args().len(), 1);
    }
}
