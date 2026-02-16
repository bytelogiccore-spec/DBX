//! SQL-based Trigger System
//!
//! SQL 표준 호환 Trigger 구현

use serde::{Deserialize, Serialize};

/// SQL Trigger 타이밍
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerTiming {
    Before,
    After,
}

/// SQL Trigger 작업
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerOperation {
    Insert,
    Update,
    Delete,
}

/// FOR EACH 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForEachType {
    Row,
    Statement,
}

/// SQL 표준 Trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Trigger 이름
    pub name: String,

    /// 타이밍 (BEFORE/AFTER)
    pub timing: TriggerTiming,

    /// 작업 (INSERT/UPDATE/DELETE)
    pub operation: TriggerOperation,

    /// 대상 테이블
    pub table: String,

    /// FOR EACH ROW/STATEMENT
    pub for_each: ForEachType,

    /// WHEN 조건 (SQL 표현식)
    pub condition: Option<String>,

    /// 실행할 SQL 문장들
    pub body: Vec<String>,

    /// 생성 시각
    pub created_at: u64,
}

impl Trigger {
    /// 새 SQL Trigger 생성
    pub fn new(
        name: impl Into<String>,
        timing: TriggerTiming,
        operation: TriggerOperation,
        table: impl Into<String>,
        for_each: ForEachType,
        condition: Option<String>,
        body: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            timing,
            operation,
            table: table.into(),
            for_each,
            condition,
            body,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Trigger를 JSON으로 직렬화
    pub fn to_json(&self) -> crate::error::DbxResult<String> {
        serde_json::to_string(self).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to serialize trigger: {}", e))
        })
    }

    /// JSON에서 Trigger 역직렬화
    pub fn from_json(json: &str) -> crate::error::DbxResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to deserialize trigger: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_trigger_creation() {
        let trigger = Trigger::new(
            "audit_log",
            TriggerTiming::After,
            TriggerOperation::Insert,
            "users",
            ForEachType::Row,
            Some("NEW.age > 18".to_string()),
            vec!["INSERT INTO audit_logs VALUES (NEW.id, 'INSERT')".to_string()],
        );

        assert_eq!(trigger.name, "audit_log");
        assert_eq!(trigger.timing, TriggerTiming::After);
        assert_eq!(trigger.operation, TriggerOperation::Insert);
        assert_eq!(trigger.table, "users");
        assert_eq!(trigger.for_each, ForEachType::Row);
        assert!(trigger.condition.is_some());
        assert_eq!(trigger.body.len(), 1);
    }

    #[test]
    fn test_sql_trigger_serialization() {
        let trigger = Trigger::new(
            "test_trigger",
            TriggerTiming::Before,
            TriggerOperation::Update,
            "products",
            ForEachType::Row,
            None,
            vec!["UPDATE logs SET count = count + 1".to_string()],
        );

        let json = trigger.to_json().unwrap();
        let deserialized = Trigger::from_json(&json).unwrap();

        assert_eq!(trigger.name, deserialized.name);
        assert_eq!(trigger.timing, deserialized.timing);
        assert_eq!(trigger.operation, deserialized.operation);
    }
}
