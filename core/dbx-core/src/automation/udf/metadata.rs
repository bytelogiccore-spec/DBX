//! UDF Metadata
//!
//! UDF 메타데이터 (직렬화 가능)
//!
//! 주의: UDF 함수 로직 자체는 Rust 클로저이므로 직렬화 불가능.
//! 메타데이터만 저장하고, 실제 함수는 코드로 등록해야 함.

use serde::{Deserialize, Serialize};

/// UDF 타입
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UdfType {
    Scalar,
    Aggregate,
    Table,
}

/// UDF 메타데이터 (직렬화 가능)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdfMetadata {
    /// UDF 이름
    pub name: String,

    /// UDF 타입
    pub udf_type: UdfType,

    /// 파라미터 타입 목록 (문자열 표현)
    pub param_types: Vec<String>,

    /// 반환 타입 (문자열 표현)
    pub return_type: String,

    /// 가변 인자 여부
    pub is_variadic: bool,

    /// 설명 (선택적)
    pub description: Option<String>,

    /// 생성 시각
    pub created_at: u64,
}

impl UdfMetadata {
    /// 새 UDF 메타데이터 생성
    pub fn new(
        name: impl Into<String>,
        udf_type: UdfType,
        param_types: Vec<String>,
        return_type: impl Into<String>,
        is_variadic: bool,
    ) -> Self {
        Self {
            name: name.into(),
            udf_type,
            param_types,
            return_type: return_type.into(),
            is_variadic,
            description: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// 설명 추가
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// JSON으로 직렬화
    pub fn to_json(&self) -> crate::error::DbxResult<String> {
        serde_json::to_string(self).map_err(|e| {
            crate::error::DbxError::Serialization(format!(
                "Failed to serialize UDF metadata: {}",
                e
            ))
        })
    }

    /// JSON에서 역직렬화
    pub fn from_json(json: &str) -> crate::error::DbxResult<Self> {
        serde_json::from_str(json).map_err(|e| {
            crate::error::DbxError::Serialization(format!(
                "Failed to deserialize UDF metadata: {}",
                e
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_udf_metadata_creation() {
        let meta = UdfMetadata::new(
            "my_function",
            UdfType::Scalar,
            vec!["Int".to_string(), "String".to_string()],
            "Boolean",
            false,
        );

        assert_eq!(meta.name, "my_function");
        assert_eq!(meta.udf_type, UdfType::Scalar);
        assert_eq!(meta.param_types.len(), 2);
        assert_eq!(meta.return_type, "Boolean");
        assert!(!meta.is_variadic);
    }

    #[test]
    fn test_udf_metadata_serialization() {
        let meta = UdfMetadata::new(
            "test_udf",
            UdfType::Aggregate,
            vec!["Float".to_string()],
            "Float",
            true,
        )
        .with_description("Test aggregate function");

        let json = meta.to_json().unwrap();
        let deserialized = UdfMetadata::from_json(&json).unwrap();

        assert_eq!(meta.name, deserialized.name);
        assert_eq!(meta.udf_type, deserialized.udf_type);
        assert_eq!(meta.param_types, deserialized.param_types);
        assert_eq!(meta.return_type, deserialized.return_type);
        assert_eq!(meta.is_variadic, deserialized.is_variadic);
        assert_eq!(meta.description, deserialized.description);
    }
}
