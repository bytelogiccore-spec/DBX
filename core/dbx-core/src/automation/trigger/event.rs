//! Trigger Events
//!
//! 트리거를 발생시키는 이벤트 정의

use crate::automation::callable::Value;
use std::collections::HashMap;

/// 트리거 이벤트 타입
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TriggerEventType {
    /// INSERT 이벤트
    BeforeInsert,
    AfterInsert,
    
    /// UPDATE 이벤트
    BeforeUpdate,
    AfterUpdate,
    
    /// DELETE 이벤트
    BeforeDelete,
    AfterDelete,
    
    /// 스케줄 이벤트
    Scheduled,
}

/// 트리거 이벤트
#[derive(Debug, Clone)]
pub struct TriggerEvent {
    /// 이벤트 타입
    pub event_type: TriggerEventType,
    
    /// 테이블 이름
    pub table: String,
    
    /// 이벤트 데이터 (old/new values)
    pub data: HashMap<String, Value>,
    
    /// 타임스탬프
    pub timestamp: u64,
}

impl TriggerEvent {
    /// 새 이벤트 생성
    pub fn new(event_type: TriggerEventType, table: impl Into<String>) -> Self {
        Self {
            event_type,
            table: table.into(),
            data: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
    
    /// 데이터 추가
    pub fn with_data(mut self, key: impl Into<String>, value: Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trigger_event_creation() {
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users")
            .with_data("id", Value::Int(1))
            .with_data("name", Value::String("Alice".to_string()));
        
        assert_eq!(event.event_type, TriggerEventType::AfterInsert);
        assert_eq!(event.table, "users");
        assert_eq!(event.data.len(), 2);
    }
}
