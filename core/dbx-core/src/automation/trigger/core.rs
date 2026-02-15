//! Trigger implementation
//!
//! 트리거 정의 및 실행

use super::event::{TriggerEvent, TriggerEventType};
use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// 트리거 조건
pub enum TriggerCondition {
    /// 항상 실행
    Always,

    /// UDF 조건 (조건 함수 이름)
    UdfCondition(String),

    /// 커스텀 조건 함수
    Custom(Box<dyn Fn(&TriggerEvent) -> bool + Send + Sync>),
}

/// Type alias for trigger action function
type TriggerActionFn = Box<dyn Fn(&ExecutionContext, &TriggerEvent) -> DbxResult<()> + Send + Sync>;

/// 트리거 액션
pub enum TriggerAction {
    /// UDF 호출
    CallUdf(String),

    /// 커스텀 액션
    Custom(TriggerActionFn),
}

/// 트리거
pub struct Trigger {
    name: String,
    event_type: TriggerEventType,
    table: String,
    condition: TriggerCondition,
    action: TriggerAction,
    signature: Signature,
}

impl Trigger {
    /// 새 트리거 생성
    pub fn new(
        name: impl Into<String>,
        event_type: TriggerEventType,
        table: impl Into<String>,
        condition: TriggerCondition,
        action: TriggerAction,
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

    /// 이벤트가 이 트리거와 매칭되는지 확인
    pub fn matches(&self, event: &TriggerEvent) -> bool {
        self.event_type == event.event_type && self.table == event.table
    }

    /// 조건 평가
    pub fn evaluate_condition(&self, ctx: &ExecutionContext, event: &TriggerEvent) -> bool {
        match &self.condition {
            TriggerCondition::Always => true,
            TriggerCondition::UdfCondition(udf_name) => {
                // UDF를 호출하여 조건 평가 (truthy 판단)
                match ctx.dbx.call_udf(udf_name, &[]) {
                    Ok(value) => value.is_truthy(),
                    Err(_) => false, // UDF 호출 실패 시 조건 불충족
                }
            }
            TriggerCondition::Custom(func) => func(event),
        }
    }

    /// 액션 실행
    pub fn execute_action(&self, ctx: &ExecutionContext, event: &TriggerEvent) -> DbxResult<()> {
        match &self.action {
            TriggerAction::CallUdf(udf_name) => {
                // 이벤트 데이터를 인자로 변환하여 UDF 호출
                let args: Vec<Value> = event.data.values().cloned().collect();
                ctx.dbx.call_udf(udf_name, &args)?;
                Ok(())
            }
            TriggerAction::Custom(func) => func(ctx, event),
        }
    }

    /// 트리거 이름
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 이벤트 타입
    pub fn event_type(&self) -> &TriggerEventType {
        &self.event_type
    }

    /// 테이블 이름
    pub fn table(&self) -> &str {
        &self.table
    }

    /// 이벤트 매칭 + 조건 평가 + 액션 실행 (통합)
    pub fn fire(&self, ctx: &ExecutionContext, event: &TriggerEvent) -> DbxResult<bool> {
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

impl Callable for Trigger {
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
    fn test_trigger_creation() {
        let trigger = Trigger::new(
            "test_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        assert_eq!(trigger.name(), "test_trigger");
        assert_eq!(trigger.event_type(), &TriggerEventType::AfterInsert);
        assert_eq!(trigger.table(), "users");
    }

    #[test]
    fn test_trigger_matching() {
        let trigger = Trigger::new(
            "test_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users");
        assert!(trigger.matches(&event));

        let event2 = TriggerEvent::new(TriggerEventType::AfterInsert, "posts");
        assert!(!trigger.matches(&event2));
    }

    #[test]
    fn test_trigger_condition() {
        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let trigger = Trigger::new(
            "test_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Custom(Box::new(|event| event.data.contains_key("id"))),
            TriggerAction::Custom(Box::new(|_ctx, _event| Ok(()))),
        );

        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users")
            .with_data("id", Value::Int(1));

        assert!(trigger.evaluate_condition(&ctx, &event));
    }

    #[test]
    fn test_trigger_action() {
        let executed = Arc::new(std::sync::Mutex::new(false));
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

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users");

        trigger.execute_action(&ctx, &event).unwrap();
        assert!(*executed.lock().unwrap());
    }

    #[test]
    fn test_trigger_fire_integration() {
        let executed = Arc::new(std::sync::Mutex::new(false));
        let executed_clone = Arc::clone(&executed);

        let trigger = Trigger::new(
            "fire_test",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |_ctx, _event| {
                *executed_clone.lock().unwrap() = true;
                Ok(())
            })),
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // 매칭되는 이벤트
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users");
        assert_eq!(trigger.fire(&ctx, &event).unwrap(), true);
        assert!(*executed.lock().unwrap());

        // 매칭 안 되는 이벤트
        let event2 = TriggerEvent::new(TriggerEventType::AfterDelete, "posts");
        assert_eq!(trigger.fire(&ctx, &event2).unwrap(), false);
    }

    #[test]
    fn test_trigger_call_udf_action() {
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

        let trigger = Trigger::new(
            "udf_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::CallUdf("log_insert".to_string()),
        );

        let ctx = ExecutionContext::new(Arc::new(db));
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users")
            .with_data("id", Value::Int(42));

        // CallUdf 액션이 실제로 UDF를 호출
        assert!(trigger.fire(&ctx, &event).is_ok());
    }
}
