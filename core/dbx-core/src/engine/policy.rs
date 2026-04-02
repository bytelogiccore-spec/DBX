use serde::{Deserialize, Serialize};

/// 스토리지에 데이터를 분산/압축/저장하는 전략
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum StorageStrategy {
    /// N대의 노드에 단순 파티셔닝 (중복 없음)
    PureSharding,
    /// 다중 마스터 복제 (N벌의 데이터 보관)
    Replication { factor: u8 },
    /// 패리티 기반 소거 부호화 분산 (K개의 데이터 원본과 M개의 패리티)
    ErasureCoding { k: u8, m: u8 },
}

impl Default for StorageStrategy {
    fn default() -> Self {
        Self::Replication { factor: 3 }
    }
}

/// 테이블에 부여되는 수명주기(Lifecycle) 및 저장 정책
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TablePolicy {
    /// Hot Tier 보관 기간 (일 단위) - None이면 영구 보관
    pub hot_ttl_days: Option<u32>,
    pub hot_strategy: StorageStrategy,

    /// Warm Tier 보관 기간 (일 단위) - None이면 영구 보관
    pub warm_ttl_days: Option<u32>,
    pub warm_strategy: StorageStrategy,

    /// Cold Tier 보관 기간 (일 단위) - None이면 영구 보관
    pub cold_ttl_days: Option<u32>,
    pub cold_strategy: StorageStrategy,
}

impl Default for TablePolicy {
    fn default() -> Self {
        Self {
            hot_ttl_days: Some(30),
            hot_strategy: StorageStrategy::Replication { factor: 3 },
            warm_ttl_days: Some(180),
            warm_strategy: StorageStrategy::PureSharding,
            cold_ttl_days: None,
            cold_strategy: StorageStrategy::ErasureCoding { k: 4, m: 2 },
        }
    }
}

impl TablePolicy {
    /// TablePolicy 객체를 JSON 문자열로 직렬화하여 Arrow Schema 메타데이터에 심을 수 있게 합니다.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    /// JSON 문자열을 TablePolicy 객체로 역직렬화 합니다.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}
