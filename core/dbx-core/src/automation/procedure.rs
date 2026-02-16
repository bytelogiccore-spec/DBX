//! Stored Procedure
//!
//! SQL 표준 Stored Procedure 구현

use serde::{Deserialize, Serialize};

/// Stored Procedure 파라미터
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureParameter {
    pub name: String,
    pub data_type: String,
}

/// Stored Procedure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcedure {
    /// Procedure 이름
    pub name: String,

    /// 파라미터 목록
    pub parameters: Vec<ProcedureParameter>,

    /// 실행할 SQL 문장들
    pub body: Vec<String>,

    /// 생성 시각
    pub created_at: u64,
}

impl StoredProcedure {
    /// 새 Stored Procedure 생성
    pub fn new(
        name: impl Into<String>,
        parameters: Vec<ProcedureParameter>,
        body: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            parameters,
            body,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Procedure를 JSON으로 직렬화
    pub fn to_json(&self) -> crate::error::DbxResult<String> {
        serde_json::to_string(self).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to serialize procedure: {}", e))
        })
    }

    /// JSON에서 Procedure 역직렬화
    pub fn from_json(json: &str) -> crate::error::DbxResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::error::DbxError::Serialization(format!("Failed to deserialize procedure: {}", e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stored_procedure_creation() {
        let params = vec![
            ProcedureParameter {
                name: "user_id".to_string(),
                data_type: "INT".to_string(),
            },
            ProcedureParameter {
                name: "amount".to_string(),
                data_type: "DECIMAL".to_string(),
            },
        ];

        let proc = StoredProcedure::new(
            "update_balance",
            params,
            vec![
                "UPDATE accounts SET balance = balance + amount WHERE id = user_id".to_string(),
                "INSERT INTO transactions VALUES (user_id, amount, NOW())".to_string(),
            ],
        );

        assert_eq!(proc.name, "update_balance");
        assert_eq!(proc.parameters.len(), 2);
        assert_eq!(proc.body.len(), 2);
    }

    #[test]
    fn test_stored_procedure_serialization() {
        let proc = StoredProcedure::new(
            "test_proc",
            vec![ProcedureParameter {
                name: "id".to_string(),
                data_type: "INT".to_string(),
            }],
            vec!["SELECT * FROM users WHERE id = id".to_string()],
        );

        let json = proc.to_json().unwrap();
        let deserialized = StoredProcedure::from_json(&json).unwrap();

        assert_eq!(proc.name, deserialized.name);
        assert_eq!(proc.parameters.len(), deserialized.parameters.len());
        assert_eq!(proc.body.len(), deserialized.body.len());
    }
}
