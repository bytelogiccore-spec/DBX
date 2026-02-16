//! SQL Schedule
//!
//! SQL 표준 스케줄 구조체

use serde::{Deserialize, Serialize};

/// SQL 스케줄
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// 스케줄 이름
    pub name: String,

    /// Cron 표현식
    pub cron_expr: String,

    /// 실행할 SQL 문장들
    pub sql_body: Vec<String>,

    /// 활성화 여부
    pub enabled: bool,

    /// 생성 시각
    pub created_at: u64,

    /// 마지막 실행 시각 (선택적)
    pub last_run: Option<u64>,

    /// 다음 실행 시각 (선택적)
    pub next_run: Option<u64>,
}

impl Schedule {
    /// 새 스케줄 생성
    pub fn new(
        name: impl Into<String>,
        cron_expr: impl Into<String>,
        sql_body: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            cron_expr: cron_expr.into(),
            sql_body,
            enabled: true,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_run: None,
            next_run: None,
        }
    }

    /// 스케줄 비활성화
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// 스케줄 활성화
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 마지막 실행 시각 업데이트
    pub fn update_last_run(&mut self) {
        self.last_run = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }

    /// JSON으로 직렬화
    pub fn to_json(&self) -> crate::error::DbxResult<String> {
        serde_json::to_string(self).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to serialize schedule: {}", e))
        })
    }

    /// JSON에서 역직렬화
    pub fn from_json(json: &str) -> crate::error::DbxResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to deserialize schedule: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_creation() {
        let schedule = Schedule::new(
            "cleanup_job",
            "0 0 * * *",
            vec!["DELETE FROM logs WHERE created_at < NOW() - INTERVAL 30 DAY".to_string()],
        );

        assert_eq!(schedule.name, "cleanup_job");
        assert_eq!(schedule.cron_expr, "0 0 * * *");
        assert_eq!(schedule.sql_body.len(), 1);
        assert!(schedule.enabled);
    }

    #[test]
    fn test_schedule_enable_disable() {
        let mut schedule = Schedule::new("test", "0 0 * * *", vec![]);

        assert!(schedule.enabled);

        schedule.disable();
        assert!(!schedule.enabled);

        schedule.enable();
        assert!(schedule.enabled);
    }

    #[test]
    fn test_schedule_serialization() {
        let schedule = Schedule::new(
            "test_schedule",
            "*/5 * * * *",
            vec!["UPDATE stats SET count = count + 1".to_string()],
        );

        let json = schedule.to_json().unwrap();
        let deserialized = Schedule::from_json(&json).unwrap();

        assert_eq!(schedule.name, deserialized.name);
        assert_eq!(schedule.cron_expr, deserialized.cron_expr);
        assert_eq!(schedule.sql_body, deserialized.sql_body);
        assert_eq!(schedule.enabled, deserialized.enabled);
    }
}
