// Phase 0.3: 롤백 메커니즘
//
// TDD 방식으로 구현:
// 1. Red: 테스트 작성 (실패)
// 2. Green: 최소 구현 (통과)
// 3. Refactor: 코드 개선

use crate::error::{DbxError, DbxResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// 체크포인트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// 체크포인트 ID
    pub id: String,

    /// 생성 시간 (Unix timestamp)
    pub timestamp: i64,

    /// 설명
    pub description: String,

    /// 상태 데이터 (JSON)
    pub state: HashMap<String, serde_json::Value>,
}

impl Checkpoint {
    /// 새 체크포인트 생성
    pub fn new(id: String, description: String) -> Self {
        Self {
            id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            description,
            state: HashMap::new(),
        }
    }

    /// 상태 데이터 추가
    pub fn add_state<T: Serialize>(&mut self, key: String, value: &T) -> DbxResult<()> {
        let json_value = serde_json::to_value(value)?;
        self.state.insert(key, json_value);
        Ok(())
    }

    /// 상태 데이터 조회
    pub fn get_state<T: for<'de> Deserialize<'de>>(&self, key: &str) -> DbxResult<T> {
        let json_value = self
            .state
            .get(key)
            .ok_or_else(|| DbxError::Serialization(format!("State key '{}' not found", key)))?;

        let value = serde_json::from_value(json_value.clone())?;
        Ok(value)
    }
}

/// 롤백 관리자
pub struct RollbackManager {
    /// 체크포인트 (ID → Checkpoint)
    checkpoints: Arc<RwLock<HashMap<String, Checkpoint>>>,

    /// 체크포인트 디렉토리
    checkpoint_dir: PathBuf,

    /// 자동 롤백 활성화
    auto_rollback_enabled: bool,
}

impl RollbackManager {
    /// 새 롤백 관리자 생성
    pub fn new() -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_dir: PathBuf::from("target/checkpoints"),
            auto_rollback_enabled: false,
        }
    }

    /// 체크포인트 디렉토리 설정
    pub fn with_checkpoint_dir(mut self, dir: PathBuf) -> Self {
        self.checkpoint_dir = dir;
        self
    }

    /// 자동 롤백 활성화
    pub fn with_auto_rollback(mut self, enabled: bool) -> Self {
        self.auto_rollback_enabled = enabled;
        self
    }

    /// 체크포인트 생성
    pub fn create_checkpoint(&self, id: String, description: String) -> DbxResult<Checkpoint> {
        let checkpoint = Checkpoint::new(id.clone(), description);

        // 메모리에 저장
        self.checkpoints
            .write()
            .unwrap()
            .insert(id.clone(), checkpoint.clone());

        // 파일에 저장
        self.save_checkpoint(&checkpoint)?;

        Ok(checkpoint)
    }

    /// 체크포인트 저장
    fn save_checkpoint(&self, checkpoint: &Checkpoint) -> DbxResult<()> {
        // 디렉토리 생성
        fs::create_dir_all(&self.checkpoint_dir)?;

        // 파일 경로
        let file_path = self.checkpoint_dir.join(format!("{}.json", checkpoint.id));

        // JSON 직렬화
        let json = serde_json::to_string_pretty(checkpoint)?;

        // 파일 쓰기
        fs::write(file_path, json)?;

        Ok(())
    }

    /// 체크포인트 로드
    fn load_checkpoint(&self, id: &str) -> DbxResult<Checkpoint> {
        let file_path = self.checkpoint_dir.join(format!("{}.json", id));

        if !file_path.exists() {
            return Err(DbxError::Serialization(format!(
                "Checkpoint '{}' not found",
                id
            )));
        }

        let json = fs::read_to_string(file_path)?;
        let checkpoint: Checkpoint = serde_json::from_str(&json)?;

        Ok(checkpoint)
    }

    /// 체크포인트로 롤백
    pub fn rollback_to_checkpoint(&self, id: &str) -> DbxResult<Checkpoint> {
        // 파일에서 로드
        let checkpoint = self.load_checkpoint(id)?;

        // 메모리에 저장
        self.checkpoints
            .write()
            .unwrap()
            .insert(id.to_string(), checkpoint.clone());

        Ok(checkpoint)
    }

    /// 체크포인트 조회
    pub fn get_checkpoint(&self, id: &str) -> Option<Checkpoint> {
        self.checkpoints.read().unwrap().get(id).cloned()
    }

    /// 모든 체크포인트 조회
    pub fn list_checkpoints(&self) -> Vec<Checkpoint> {
        self.checkpoints.read().unwrap().values().cloned().collect()
    }

    /// 체크포인트 삭제
    pub fn delete_checkpoint(&self, id: &str) -> DbxResult<()> {
        // 메모리에서 삭제
        self.checkpoints.write().unwrap().remove(id);

        // 파일 삭제
        let file_path = self.checkpoint_dir.join(format!("{}.json", id));
        if file_path.exists() {
            fs::remove_file(file_path)?;
        }

        Ok(())
    }

    /// 자동 롤백 트리거
    pub fn trigger_auto_rollback(&self, reason: &str) -> DbxResult<()> {
        if !self.auto_rollback_enabled {
            return Ok(());
        }

        // 가장 최근 체크포인트로 롤백
        let checkpoints = self.list_checkpoints();
        if let Some(latest) = checkpoints.iter().max_by_key(|c| c.timestamp) {
            eprintln!("Auto-rollback triggered: {}", reason);
            eprintln!("Rolling back to checkpoint: {}", latest.id);
            self.rollback_to_checkpoint(&latest.id)?;
        }

        Ok(())
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TDD: Red - 테스트 작성 (실패)

    #[test]
    fn test_checkpoint_creation() {
        let manager =
            RollbackManager::new().with_checkpoint_dir(PathBuf::from("target/test_checkpoints"));

        let checkpoint = manager
            .create_checkpoint("test_cp_1".to_string(), "Test checkpoint".to_string())
            .unwrap();

        assert_eq!(checkpoint.id, "test_cp_1");
        assert_eq!(checkpoint.description, "Test checkpoint");
        assert!(checkpoint.timestamp > 0);

        // 조회 확인
        let loaded = manager.get_checkpoint("test_cp_1");
        assert!(loaded.is_some());

        // 정리
        let _ = manager.delete_checkpoint("test_cp_1");
    }

    #[test]
    fn test_rollback_to_checkpoint() {
        let manager =
            RollbackManager::new().with_checkpoint_dir(PathBuf::from("target/test_checkpoints"));

        // 체크포인트 생성
        let mut checkpoint = manager
            .create_checkpoint("test_cp_2".to_string(), "Rollback test".to_string())
            .unwrap();

        // 상태 데이터 추가
        checkpoint
            .add_state("key1".to_string(), &"value1".to_string())
            .unwrap();
        checkpoint.add_state("key2".to_string(), &42).unwrap();

        // 저장
        manager
            .checkpoints
            .write()
            .unwrap()
            .insert("test_cp_2".to_string(), checkpoint.clone());
        manager.save_checkpoint(&checkpoint).unwrap();

        // 메모리 초기화
        manager.checkpoints.write().unwrap().clear();

        // 롤백
        let restored = manager.rollback_to_checkpoint("test_cp_2").unwrap();

        // 확인
        assert_eq!(restored.id, "test_cp_2");
        let value1: String = restored.get_state("key1").unwrap();
        let value2: i32 = restored.get_state("key2").unwrap();
        assert_eq!(value1, "value1");
        assert_eq!(value2, 42);

        // 정리
        let _ = manager.delete_checkpoint("test_cp_2");
    }

    #[test]
    fn test_auto_rollback_on_regression() {
        let manager = RollbackManager::new()
            .with_checkpoint_dir(PathBuf::from("target/test_checkpoints"))
            .with_auto_rollback(true);

        // 체크포인트 생성
        manager
            .create_checkpoint("test_cp_3".to_string(), "Auto-rollback test".to_string())
            .unwrap();

        // 자동 롤백 트리거
        manager
            .trigger_auto_rollback("Performance regression detected")
            .unwrap();

        // 확인
        let checkpoint = manager.get_checkpoint("test_cp_3");
        assert!(checkpoint.is_some());

        // 정리
        let _ = manager.delete_checkpoint("test_cp_3");
    }
}
