//! EventHook implementation
//!
//! 이벤트 훅 정의 및 실행

use super::event::{EventHookEvent, EventHookEventType};
use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// 이벤트 훅 조건
pub enum EventHookCondition {
    /// 항상 실행
    Always,

    /// UDF 조건 (조건 함수 이름)
    UdfCondition(String),

    /// 커스텀 조건 함수
    Custom(Box<dyn Fn(&EventHookEvent) -> bool + Send + Sync>),
}

/// Type alias for event hook action function
type EventHookActionFn =
    Box<dyn Fn(&ExecutionContext, &EventHookEvent) -> DbxResult<()> + Send + Sync>;

/// 이벤트 훅 액션
pub enum EventHookAction {
    /// UDF 호출
    CallUdf(String),

    /// 커스텀 액션
    Custom(EventHookActionFn),
}

/// 이벤트 훅
pub struct EventHook {
    name: String,
    event_type: EventHookEventType,
    table: String,
    condition: EventHookCondition,
    action: EventHookAction,
    signature: Signature,
}

impl EventHook {
    /// 새 이벤트 훅 생성
    pub fn new(
        name: impl Into<String>,
        event_type: EventHookEventType,
        table: impl Into<String>,
        condition: EventHookCondition,
        action: EventHookAction,
    ) -> Self {
        Self {
            name: name.into(),
            event_type,
            table: table.into(),
            condition,
            action,
            signature: Signature {
                params: vec![],
                return_type: crate::automation::callable::DataType::Null,
                is_variadic: false,
            },
        }
    }

    /// 이벤트가 이 훅과 매칭되는지 확인
    pub fn matches(&self, event: &EventHookEvent) -> bool {
        self.event_type == event.event_type && self.table == event.table
    }

    /// 조건 평가
    pub fn evaluate_condition(&self, ctx: &ExecutionContext, event: &EventHookEvent) -> bool {
        match &self.condition {
            EventHookCondition::Always => true,
            EventHookCondition::UdfCondition(udf_name) => {
                // UDF를 호출하여 조건 평가 (truthy 판단)
                match ctx.dbx.call_udf(udf_name, &[]) {
                    Ok(value) => value.is_truthy(),
                    Err(_) => false, // UDF 호출 실패 시 조건 불충족
                }
            }
            EventHookCondition::Custom(func) => func(event),
        }
    }

    /// 액션 실행
    pub fn execute_action(&self, ctx: &ExecutionContext, event: &EventHookEvent) -> DbxResult<()> {
        match &self.action {
            EventHookAction::CallUdf(udf_name) => {
                // 이벤트 데이터를 인자로 변환하여 UDF 호출
                let args: Vec<Value> = event.data.values().cloned().collect();
                ctx.dbx.call_udf(udf_name, &args)?;
                Ok(())
            }
            EventHookAction::Custom(func) => func(ctx, event),
        }
    }

    /// 이벤트 훅 이름
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 이벤트 타입
    pub fn event_type(&self) -> &EventHookEventType {
        &self.event_type
    }

    /// 테이블 이름
    pub fn table(&self) -> &str {
        &self.table
    }

    /// 이벤트 매칭 + 조건 평가 + 액션 실행 (통합)
    pub fn fire(&self, ctx: &ExecutionContext, event: &EventHookEvent) -> DbxResult<bool> {
        if !self.matches(event) {
            return Ok(false);
        }
        if !self.evaluate_condition(ctx, event) {
            return Ok(false);
        }
        self.execute_action(ctx, event)?;
        Ok(true)
    }
}

impl Callable for EventHook {
    fn call(&self, _ctx: &ExecutionContext, _args: &[Value]) -> DbxResult<Value> {
        // 트리거는 이벤트 기반이므로 직접 호출되지 않음
        // 대신 fire_trigger를 통해 실행됨
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
    use crate::engine::Database;
    use std::sync::Arc;

    #[test]
    fn test_event_hook_creation() {
        let hook = EventHook::new(
            "test_trigger",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Always,
            EventHookAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        assert_eq!(hook.name(), "test_trigger");
        assert_eq!(hook.event_type(), &EventHookEventType::AfterInsert);
        assert_eq!(hook.table(), "users");
    }

    #[test]
    fn test_event_hook_matching() {
        let hook = EventHook::new(
            "test_trigger",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Always,
            EventHookAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        let event = EventHookEvent::new(EventHookEventType::AfterInsert, "users");
        assert!(hook.matches(&event));

        let event2 = EventHookEvent::new(EventHookEventType::AfterInsert, "posts");
        assert!(!hook.matches(&event2));
    }

    #[test]
    fn test_event_hook_condition() {
        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let hook = EventHook::new(
            "test_trigger",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Custom(Box::new(|event| event.data.contains_key("id"))),
            EventHookAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        let event = EventHookEvent::new(EventHookEventType::AfterInsert, "users")
            .with_data("id", Value::Int(1));

        assert!(hook.evaluate_condition(&ctx, &event));
    }

    #[test]
    fn test_event_hook_action() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let executed_clone = Arc::clone(&executed);

        let hook = EventHook::new(
            "test_trigger",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Always,
            EventHookAction::Custom(Box::new(move |_ctx, _event| {
                *executed_clone.lock().unwrap() = true;
                Ok(())
            })),
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));
        let event = EventHookEvent::new(EventHookEventType::AfterInsert, "users");

        hook.execute_action(&ctx, &event).unwrap();
        assert!(*executed.lock().unwrap());
    }

    #[test]
    fn test_event_hook_fire_integration() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let executed_clone = Arc::clone(&executed);

        let hook = EventHook::new(
            "fire_test",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Always,
            EventHookAction::Custom(Box::new(move |_ctx, _event| {
                *executed_clone.lock().unwrap() = true;
                Ok(())
            })),
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // 매칭되는 이벤트
        let event = EventHookEvent::new(EventHookEventType::AfterInsert, "users");
        assert!(hook.fire(&ctx, &event).unwrap());
        assert!(*executed.lock().unwrap());

        // 매칭 안 되는 이벤트
        let event2 = EventHookEvent::new(EventHookEventType::AfterDelete, "posts");
        assert!(!hook.fire(&ctx, &event2).unwrap());
    }

    #[test]
    fn test_event_hook_call_udf_action() {
        use crate::automation::callable::{DataType, Signature};

        let db = Database::open_in_memory().unwrap();

        // UDF 등록
        db.register_scalar_udf(
            "log_insert",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: true,
            },
            |args| {
                // 인자를 그대로 반환 (로깅 시뮬레이션)
                Ok(args.first().cloned().unwrap_or(Value::Null))
            },
        )
        .unwrap();

        let hook = EventHook::new(
            "udf_trigger",
            EventHookEventType::AfterInsert,
            "users",
            EventHookCondition::Always,
            EventHookAction::CallUdf("log_insert".to_string()),
        );

        let ctx = ExecutionContext::new(Arc::new(db));
        let event = EventHookEvent::new(EventHookEventType::AfterInsert, "users")
            .with_data("id", Value::Int(42));

        // CallUdf 액션이 실제로 UDF를 호출
        assert!(hook.fire(&ctx, &event).is_ok());
    }
}
