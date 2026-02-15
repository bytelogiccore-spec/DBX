// Phase 0.2: Feature Flag 시스템
//
// TDD 방식으로 구현:
// 1. Red: 테스트 작성 (실패)
// 2. Green: 최소 구현 (통과)
// 3. Refactor: 코드 개선

use crate::error::DbxResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Feature Flag 정의
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Feature {
    /// 바이너리 직렬화
    BinarySerialization,

    /// 멀티스레딩
    MultiThreading,

    /// MVCC 확장
    MvccExtension,

    /// 쿼리 플랜 캐시
    QueryPlanCache,

    /// 병렬 쿼리 실행
    ParallelQuery,

    /// WAL 병렬 쓰기
    ParallelWal,

    /// 병렬 체크포인트
    ParallelCheckpoint,

    /// 스키마 버저닝
    SchemaVersioning,

    /// 인덱스 버저닝
    IndexVersioning,
}

impl Feature {
    /// Feature를 문자열로 변환
    pub fn as_str(&self) -> &'static str {
        match self {
            Feature::BinarySerialization => "binary_serialization",
            Feature::MultiThreading => "multi_threading",
            Feature::MvccExtension => "mvcc_extension",
            Feature::QueryPlanCache => "query_plan_cache",
            Feature::ParallelQuery => "parallel_query",
            Feature::ParallelWal => "parallel_wal",
            Feature::ParallelCheckpoint => "parallel_checkpoint",
            Feature::SchemaVersioning => "schema_versioning",
            Feature::IndexVersioning => "index_versioning",
        }
    }

    /// 문자열에서 Feature 파싱
    pub fn parse_feature(s: &str) -> Option<Self> {
        match s {
            "binary_serialization" => Some(Feature::BinarySerialization),
            "multi_threading" => Some(Feature::MultiThreading),
            "mvcc_extension" => Some(Feature::MvccExtension),
            "query_plan_cache" => Some(Feature::QueryPlanCache),
            "parallel_query" => Some(Feature::ParallelQuery),
            "parallel_wal" => Some(Feature::ParallelWal),
            "parallel_checkpoint" => Some(Feature::ParallelCheckpoint),
            "schema_versioning" => Some(Feature::SchemaVersioning),
            "index_versioning" => Some(Feature::IndexVersioning),
            _ => None,
        }
    }

    /// 환경 변수 이름
    pub fn env_var_name(&self) -> String {
        format!("DBX_FEATURE_{}", self.as_str().to_uppercase())
    }
}

/// Feature Flag 관리자
pub struct FeatureFlags {
    /// Feature 상태 (Feature → enabled)
    flags: Arc<RwLock<HashMap<Feature, bool>>>,

    /// 영속성 파일 경로
    persistence_path: Option<PathBuf>,
}

impl FeatureFlags {
    /// 새 Feature Flag 관리자 생성
    pub fn new() -> Self {
        Self {
            flags: Arc::new(RwLock::new(HashMap::new())),
            persistence_path: None,
        }
    }

    /// 영속성 경로 설정
    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.persistence_path = Some(path);
        self
    }

    /// Feature 활성화
    pub fn enable(&self, feature: Feature) {
        self.flags.write().unwrap().insert(feature, true);
    }

    /// Feature 비활성화
    pub fn disable(&self, feature: Feature) {
        self.flags.write().unwrap().insert(feature, false);
    }

    /// Feature 토글
    pub fn toggle(&self, feature: Feature, enabled: bool) {
        self.flags.write().unwrap().insert(feature, enabled);
    }

    /// Feature 활성화 여부 확인
    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.flags
            .read()
            .unwrap()
            .get(&feature)
            .copied()
            .unwrap_or(false)
    }

    /// 환경 변수에서 로드
    pub fn load_from_env(&self) {
        let all_features = [
            Feature::BinarySerialization,
            Feature::MultiThreading,
            Feature::MvccExtension,
            Feature::QueryPlanCache,
            Feature::ParallelQuery,
            Feature::ParallelWal,
            Feature::ParallelCheckpoint,
            Feature::SchemaVersioning,
            Feature::IndexVersioning,
        ];

        for feature in &all_features {
            let env_var = feature.env_var_name();
            if let Ok(value) = env::var(&env_var) {
                let enabled = value.to_lowercase() == "true" || value == "1";
                self.toggle(*feature, enabled);
            }
        }
    }

    /// 파일에서 로드
    pub fn load_from_file(&self) -> DbxResult<()> {
        if let Some(path) = self.persistence_path.as_ref().filter(|p| p.exists()) {
            let json = fs::read_to_string(path)?;
            let loaded: HashMap<String, bool> = serde_json::from_str(&json)?;

            let mut flags = self.flags.write().unwrap();
            for (key, value) in loaded {
                if let Some(feature) = Feature::parse_feature(&key) {
                    flags.insert(feature, value);
                }
            }
        }
        Ok(())
    }

    /// 파일에 저장
    pub fn save_to_file(&self) -> DbxResult<()> {
        if let Some(path) = &self.persistence_path {
            let flags = self.flags.read().unwrap();

            // Feature → String 변환
            let serializable: HashMap<String, bool> = flags
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), *v))
                .collect();

            let json = serde_json::to_string_pretty(&serializable)?;

            // 디렉토리 생성
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(path, json)?;
        }
        Ok(())
    }

    /// 모든 Feature 상태 조회
    pub fn all(&self) -> HashMap<Feature, bool> {
        self.flags.read().unwrap().clone()
    }

    /// 모든 Feature 초기화
    pub fn reset(&self) {
        self.flags.write().unwrap().clear();
    }
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // TDD: Red - 테스트 작성 (실패)

    #[test]
    fn test_feature_flag_enable_disable() {
        let flags = FeatureFlags::new();

        // 기본값: 비활성화
        assert!(!flags.is_enabled(Feature::BinarySerialization));

        // 활성화
        flags.enable(Feature::BinarySerialization);
        assert!(flags.is_enabled(Feature::BinarySerialization));

        // 비활성화
        flags.disable(Feature::BinarySerialization);
        assert!(!flags.is_enabled(Feature::BinarySerialization));
    }

    #[test]
    fn test_feature_flag_toggle() {
        let flags = FeatureFlags::new();

        flags.toggle(Feature::MultiThreading, true);
        assert!(flags.is_enabled(Feature::MultiThreading));

        flags.toggle(Feature::MultiThreading, false);
        assert!(!flags.is_enabled(Feature::MultiThreading));
    }

    #[test]
    fn test_feature_flag_persistence() {
        let temp_path = PathBuf::from("target/test_feature_flags.json");
        let flags = FeatureFlags::new().with_persistence(temp_path.clone());

        // Feature 설정
        flags.enable(Feature::BinarySerialization);
        flags.enable(Feature::MultiThreading);
        flags.disable(Feature::MvccExtension);

        // 저장
        flags.save_to_file().unwrap();

        // 새 인스턴스로 로드
        let flags2 = FeatureFlags::new().with_persistence(temp_path.clone());
        flags2.load_from_file().unwrap();

        // 확인
        assert!(flags2.is_enabled(Feature::BinarySerialization));
        assert!(flags2.is_enabled(Feature::MultiThreading));
        assert!(!flags2.is_enabled(Feature::MvccExtension));

        // 정리
        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn test_feature_flag_env_var() {
        let flags = FeatureFlags::new();

        // 환경 변수 설정 (unsafe)
        unsafe {
            env::set_var("DBX_FEATURE_BINARY_SERIALIZATION", "true");
            env::set_var("DBX_FEATURE_MULTI_THREADING", "1");
            env::set_var("DBX_FEATURE_MVCC_EXTENSION", "false");
        }

        // 환경 변수에서 로드
        flags.load_from_env();

        // 확인
        assert!(flags.is_enabled(Feature::BinarySerialization));
        assert!(flags.is_enabled(Feature::MultiThreading));
        assert!(!flags.is_enabled(Feature::MvccExtension));

        // 정리 (unsafe)
        unsafe {
            env::remove_var("DBX_FEATURE_BINARY_SERIALIZATION");
            env::remove_var("DBX_FEATURE_MULTI_THREADING");
            env::remove_var("DBX_FEATURE_MVCC_EXTENSION");
        }
    }

    #[test]
    fn test_feature_all_and_reset() {
        let flags = FeatureFlags::new();

        flags.enable(Feature::BinarySerialization);
        flags.enable(Feature::MultiThreading);

        // 모든 Feature 조회
        let all = flags.all();
        assert_eq!(all.len(), 2);
        assert!(
            all.get(&Feature::BinarySerialization)
                .copied()
                .unwrap_or(false)
        );

        // 초기화
        flags.reset();
        assert_eq!(flags.all().len(), 0);
    }

    #[test]
    fn test_feature_from_str() {
        assert_eq!(
            Feature::parse_feature("binary_serialization"),
            Some(Feature::BinarySerialization)
        );
        assert_eq!(
            Feature::parse_feature("multi_threading"),
            Some(Feature::MultiThreading)
        );
        assert_eq!(Feature::parse_feature("invalid"), None);
    }
}
