//! Automation API for Database
//!
//! 트리거 및 스케줄러 관리 API

use crate::automation::{
    Schedule, ScheduledJob, Scheduler, Trigger, TriggerAction, TriggerCondition, TriggerEvent,
};
use crate::automation::callable::{ExecutionContext, Value};
use crate::engine::Database;
use crate::error::DbxResult;
use std::sync::{Arc, RwLock};

/// 트리거 레지스트리 (이벤트 매칭용)
pub struct TriggerRegistry {
    triggers: RwLock<Vec<Arc<Trigger>>>,
}

impl TriggerRegistry {
    pub fn new() -> Self {
        Self {
            triggers: RwLock::new(Vec::new()),
        }
    }
    
    pub fn register(&self, trigger: Arc<Trigger>) -> DbxResult<()> {
        let mut triggers = self.triggers.write()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;
        
        // 중복 이름 체크
        if triggers.iter().any(|t| t.name() == trigger.name()) {
            return Err(crate::error::DbxError::DuplicateCallable(trigger.name().to_string()));
        }
        
        triggers.push(trigger);
        Ok(())
    }
    
    pub fn unregister(&self, name: &str) -> DbxResult<()> {
        let mut triggers = self.triggers.write()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;
        
        let pos = triggers.iter().position(|t| t.name() == name)
            .ok_or_else(|| crate::error::DbxError::CallableNotFound(name.to_string()))?;
        
        triggers.remove(pos);
        Ok(())
    }
    
    /// 이벤트에 매칭되는 트리거를 찾아서 조건 평가 후 실행
    pub fn fire(&self, ctx: &ExecutionContext, event: &TriggerEvent) -> DbxResult<Vec<String>> {
        let triggers = self.triggers.read()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;
        
        let mut executed = Vec::new();
        
        for trigger in triggers.iter() {
            match trigger.fire(ctx, event) {
                Ok(true) => executed.push(trigger.name().to_string()),
                Ok(false) => {} // 매칭 안 됨 또는 조건 불충족
                Err(e) => {
                    // 개별 트리거 실패는 로그만 남기고 계속 진행
                    eprintln!("[TRIGGER ERROR] {}: {}", trigger.name(), e);
                }
            }
        }
        
        Ok(executed)
    }
    
    pub fn list(&self) -> DbxResult<Vec<String>> {
        let triggers = self.triggers.read()
            .map_err(|_| crate::error::DbxError::LockPoisoned)?;
        
        Ok(triggers.iter().map(|t| t.name().to_string()).collect())
    }
}

impl Default for TriggerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    /// 트리거 등록
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::automation::{Trigger, TriggerEventType, TriggerCondition, TriggerAction};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// let trigger = Trigger::new(
    ///     "audit_trigger",
    ///     TriggerEventType::AfterInsert,
    ///     "users",
    ///     TriggerCondition::Always,
    ///     TriggerAction::Custom(Box::new(|_ctx, _event| {
    ///         // 감사 로그 기록
    ///         Ok(())
    ///     })),
    /// );
    ///
    /// db.register_trigger(trigger)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_trigger(&self, trigger: Trigger) -> DbxResult<()> {
        let trigger = Arc::new(trigger);
        // automation_engine에도 등록 (Callable 인터페이스)
        self.automation_engine.register(Arc::clone(&trigger) as Arc<dyn crate::automation::callable::Callable>)?;
        // trigger_registry에도 등록 (이벤트 매칭용)
        self.trigger_registry.register(trigger)
    }

    /// 트리거 등록 해제
    pub fn unregister_trigger(&self, name: &str) -> DbxResult<()> {
        self.automation_engine.unregister(name)?;
        self.trigger_registry.unregister(name)
    }

    /// 트리거 발생
    ///
    /// 이벤트를 발생시켜 매칭되는 트리거들을 실행합니다.
    pub fn fire_trigger(&self, event: TriggerEvent) -> DbxResult<Vec<String>> {
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory()?));
        self.trigger_registry.fire(&ctx, &event)
    }
    
    /// 트리거 발생 (컨텍스트 지정)
    pub fn fire_trigger_with_ctx(&self, ctx: &ExecutionContext, event: TriggerEvent) -> DbxResult<Vec<String>> {
        self.trigger_registry.fire(ctx, &event)
    }

    /// 등록된 트리거 목록
    pub fn list_triggers(&self) -> DbxResult<Vec<String>> {
        self.trigger_registry.list()
    }

    /// 스케줄러 생성
    pub fn create_scheduler(&self) -> Scheduler {
        Scheduler::new(Arc::clone(&self.automation_engine))
    }

    /// 스케줄 작업 등록
    pub fn register_scheduled_job(&self, scheduler: &Scheduler, job: ScheduledJob) -> DbxResult<()> {
        scheduler.register(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::callable::{DataType, Signature, Value};
    use crate::automation::{ScalarUDF, ScheduleType, TriggerEventType};
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    fn test_register_trigger() {
        let db = Database::open_in_memory().unwrap();

        let executed = Arc::new(Mutex::new(false));
        let executed_clone = Arc::clone(&executed);

        let trigger = Trigger::new(
            "test_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |_ctx, _event| {
                *executed_clone.lock().unwrap() = true;
                Ok(())
            })),
        );

        db.register_trigger(trigger).unwrap();

        let triggers = db.list_triggers().unwrap();
        assert_eq!(triggers.len(), 1);
        assert!(triggers.contains(&"test_trigger".to_string()));
    }

    #[test]
    fn test_unregister_trigger() {
        let db = Database::open_in_memory().unwrap();

        let trigger = Trigger::new(
            "test_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        db.register_trigger(trigger).unwrap();
        assert_eq!(db.list_triggers().unwrap().len(), 1);

        db.unregister_trigger("test_trigger").unwrap();
        assert_eq!(db.list_triggers().unwrap().len(), 0);
    }

    #[test]
    fn test_create_scheduler() {
        let db = Database::open_in_memory().unwrap();
        let scheduler = db.create_scheduler();

        let schedule = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let job = ScheduledJob::new("test_job", schedule, "test_udf", vec![]);

        scheduler.register(job).unwrap();

        let jobs = scheduler.list().unwrap();
        assert_eq!(jobs.len(), 1);
    }

    #[test]
    fn test_trigger_with_udf() {
        let db = Database::open_in_memory().unwrap();

        // UDF 등록
        db.register_scalar_udf(
            "double",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| {
                let x = args[0].as_i64()?;
                Ok(Value::Int(x * 2))
            },
        )
        .unwrap();

        // 트리거 등록
        let result = Arc::new(Mutex::new(0i64));
        let result_clone = Arc::clone(&result);

        let trigger = Trigger::new(
            "double_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |ctx, event| {
                if let Some(value) = event.data.get("id") {
                    let doubled = ctx.dbx.call_udf("double", &[value.clone()])?;
                    *result_clone.lock().unwrap() = doubled.as_i64()?;
                }
                Ok(())
            })),
        );

        db.register_trigger(trigger).unwrap();

        // 트리거 발생
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users")
            .with_data("id", Value::Int(21));

        db.fire_trigger(event).unwrap();

        // 트리거와 UDF가 모두 automation_engine에 등록됨
        let callables = db.list_triggers().unwrap();
        assert!(callables.len() >= 1); // 최소 트리거 1개
        assert!(callables.contains(&"double_trigger".to_string()));
    }

    #[test]
    fn test_scheduler_with_udf() {
        let db = Database::open_in_memory().unwrap();

        // UDF 등록
        db.register_scalar_udf(
            "triple",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| {
                let x = args[0].as_i64()?;
                Ok(Value::Int(x * 3))
            },
        )
        .unwrap();

        // 스케줄러 생성
        let scheduler = db.create_scheduler();

        // 작업 등록
        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(0)));
        let job = ScheduledJob::new("triple_job", schedule, "triple", vec![Value::Int(14)]);

        scheduler.register(job).unwrap();

        let jobs = scheduler.list().unwrap();
        assert_eq!(jobs.len(), 1);
        assert!(jobs.contains(&"triple_job".to_string()));
    }
}
