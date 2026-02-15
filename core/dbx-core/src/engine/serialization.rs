// Phase 1.1: 바이너리 직렬화 프레임워크
//
// TDD 방식으로 구현:
// 1. Red: 테스트 작성 (실패)
// 2. Green: 최소 구현 (통과)
// 3. Refactor: 코드 개선

use crate::error::{DbxError, DbxResult};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// 직렬화 함수 타입
pub type SerializeFn = Arc<dyn Fn(&[u8]) -> DbxResult<Vec<u8>> + Send + Sync>;

/// 역직렬화 함수 타입
pub type DeserializeFn = Arc<dyn Fn(&[u8]) -> DbxResult<Vec<u8>> + Send + Sync>;

/// 직렬화 레지스트리
pub struct SerializationRegistry {
    /// 타입별 직렬화 함수
    serializers: Arc<RwLock<HashMap<String, SerializeFn>>>,

    /// 타입별 역직렬화 함수
    deserializers: Arc<RwLock<HashMap<String, DeserializeFn>>>,
}

impl SerializationRegistry {
    /// 새 레지스트리 생성
    pub fn new() -> Self {
        Self {
            serializers: Arc::new(RwLock::new(HashMap::new())),
            deserializers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 직렬화 함수 등록
    pub fn register_serializer(&self, type_name: String, serializer: SerializeFn) {
        self.serializers
            .write()
            .unwrap()
            .insert(type_name, serializer);
    }

    /// 역직렬화 함수 등록
    pub fn register_deserializer(&self, type_name: String, deserializer: DeserializeFn) {
        self.deserializers
            .write()
            .unwrap()
            .insert(type_name, deserializer);
    }

    /// 데이터 직렬화
    pub fn serialize(&self, type_name: &str, data: &[u8]) -> DbxResult<Vec<u8>> {
        let serializers = self.serializers.read().unwrap();
        let serializer = serializers.get(type_name).ok_or_else(|| {
            DbxError::Serialization(format!("No serializer registered for type '{}'", type_name))
        })?;

        serializer(data)
    }

    /// 데이터 역직렬화
    pub fn deserialize(&self, type_name: &str, data: &[u8]) -> DbxResult<Vec<u8>> {
        let deserializers = self.deserializers.read().unwrap();
        let deserializer = deserializers.get(type_name).ok_or_else(|| {
            DbxError::Serialization(format!(
                "No deserializer registered for type '{}'",
                type_name
            ))
        })?;

        deserializer(data)
    }

    /// 등록된 타입 목록
    pub fn registered_types(&self) -> Vec<String> {
        self.serializers.read().unwrap().keys().cloned().collect()
    }

    /// 데이터 압축 (zstd)
    pub fn compress(&self, data: &[u8], level: i32) -> DbxResult<Vec<u8>> {
        zstd::encode_all(data, level)
            .map_err(|e| DbxError::Serialization(format!("Compression failed: {}", e)))
    }

    /// 데이터 압축 해제 (zstd)
    pub fn decompress(&self, data: &[u8]) -> DbxResult<Vec<u8>> {
        zstd::decode_all(data)
            .map_err(|e| DbxError::Serialization(format!("Decompression failed: {}", e)))
    }

    /// 체크섬 계산 (SHA256)
    pub fn checksum(&self, data: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// 체크섬 검증 (SHA256)
    pub fn verify_checksum(&self, data: &[u8], expected_checksum: &[u8]) -> bool {
        let actual_checksum = self.checksum(data);
        actual_checksum == expected_checksum
    }
}

impl Default for SerializationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 2단계 캐시 (L1: 메모리, L2: 디스크)
pub struct TwoLevelCache {
    /// L1 캐시 (메모리)
    l1_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,

    /// L1 캐시 최대 크기 (바이트)
    l1_max_size: usize,

    /// L1 캐시 현재 크기 (바이트)
    l1_current_size: Arc<RwLock<usize>>,

    /// L2 캐시 디렉토리
    l2_cache_dir: PathBuf,
}

impl TwoLevelCache {
    /// 새 2단계 캐시 생성
    pub fn new(l1_max_size: usize, l2_cache_dir: PathBuf) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(HashMap::new())),
            l1_max_size,
            l1_current_size: Arc::new(RwLock::new(0)),
            l2_cache_dir,
        }
    }

    /// 데이터 저장
    pub fn put(&self, key: String, value: Vec<u8>) -> DbxResult<()> {
        let value_size = value.len();

        // L1 캐시에 저장 시도
        let mut current_size = self.l1_current_size.write().unwrap();
        if *current_size + value_size <= self.l1_max_size {
            self.l1_cache
                .write()
                .unwrap()
                .insert(key.clone(), value.clone());
            *current_size += value_size;
        } else {
            // L1 캐시가 가득 차면 L2 캐시에 저장
            drop(current_size);
            self.put_l2(&key, &value)?;
        }

        Ok(())
    }

    /// 데이터 조회
    pub fn get(&self, key: &str) -> DbxResult<Option<Vec<u8>>> {
        // L1 캐시 확인
        if let Some(value) = self.l1_cache.read().unwrap().get(key) {
            return Ok(Some(value.clone()));
        }

        // L2 캐시 확인
        self.get_l2(key)
    }

    /// L2 캐시에 저장
    fn put_l2(&self, key: &str, value: &[u8]) -> DbxResult<()> {
        // 디렉토리 생성
        fs::create_dir_all(&self.l2_cache_dir)?;

        // 파일 경로
        let file_path = self.l2_cache_dir.join(format!("{}.bin", key));

        // 파일 쓰기
        fs::write(file_path, value)?;

        Ok(())
    }

    /// L2 캐시에서 조회
    fn get_l2(&self, key: &str) -> DbxResult<Option<Vec<u8>>> {
        let file_path = self.l2_cache_dir.join(format!("{}.bin", key));

        if !file_path.exists() {
            return Ok(None);
        }

        let data = fs::read(file_path)?;
        Ok(Some(data))
    }

    /// 캐시 초기화
    pub fn clear(&self) -> DbxResult<()> {
        // L1 캐시 초기화
        self.l1_cache.write().unwrap().clear();
        *self.l1_current_size.write().unwrap() = 0;

        // L2 캐시 초기화
        if self.l2_cache_dir.exists() {
            fs::remove_dir_all(&self.l2_cache_dir)?;
        }

        Ok(())
    }

    /// L1 캐시 크기 조회
    pub fn l1_size(&self) -> usize {
        *self.l1_current_size.read().unwrap()
    }

    /// L1 캐시 항목 수
    pub fn l1_count(&self) -> usize {
        self.l1_cache.read().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TDD: Red - 테스트 작성 (실패)

    #[test]
    fn test_serialization_registry() {
        let registry = SerializationRegistry::new();

        // 직렬화 함수 등록 (단순 복사)
        let serializer: SerializeFn = Arc::new(|data| Ok(data.to_vec()));
        registry.register_serializer("test_type".to_string(), serializer);

        // 역직렬화 함수 등록 (단순 복사)
        let deserializer: DeserializeFn = Arc::new(|data| Ok(data.to_vec()));
        registry.register_deserializer("test_type".to_string(), deserializer);

        // 직렬화
        let data = b"hello world";
        let serialized = registry.serialize("test_type", data).unwrap();
        assert_eq!(serialized, data);

        // 역직렬화
        let deserialized = registry.deserialize("test_type", &serialized).unwrap();
        assert_eq!(deserialized, data);

        // 등록된 타입 확인
        let types = registry.registered_types();
        assert!(types.contains(&"test_type".to_string()));
    }

    #[test]
    fn test_two_level_cache_l1() {
        let cache = TwoLevelCache::new(1024, PathBuf::from("target/test_cache_l1"));

        // 작은 데이터 저장 (L1 캐시에 저장됨)
        let key = "test_key".to_string();
        let value = b"test_value".to_vec();
        cache.put(key.clone(), value.clone()).unwrap();

        // L1 캐시에서 조회
        let retrieved = cache.get(&key).unwrap();
        assert_eq!(retrieved, Some(value));

        // L1 캐시 크기 확인
        assert!(cache.l1_size() > 0);
        assert_eq!(cache.l1_count(), 1);

        // 정리
        let _ = cache.clear();
    }

    #[test]
    fn test_two_level_cache_l2() {
        let cache = TwoLevelCache::new(10, PathBuf::from("target/test_cache_l2"));

        // 큰 데이터 저장 (L2 캐시에 저장됨)
        let key = "large_key".to_string();
        let value = vec![0u8; 100]; // 100 바이트
        cache.put(key.clone(), value.clone()).unwrap();

        // L2 캐시에서 조회
        let retrieved = cache.get(&key).unwrap();
        assert_eq!(retrieved, Some(value));

        // 정리
        let _ = cache.clear();
    }

    #[test]
    fn test_two_level_cache_clear() {
        let cache = TwoLevelCache::new(1024, PathBuf::from("target/test_cache_clear"));

        // 데이터 저장
        cache.put("key1".to_string(), b"value1".to_vec()).unwrap();
        cache.put("key2".to_string(), b"value2".to_vec()).unwrap();

        // 초기화
        cache.clear().unwrap();

        // 확인
        assert_eq!(cache.l1_size(), 0);
        assert_eq!(cache.l1_count(), 0);
        assert_eq!(cache.get("key1").unwrap(), None);
    }
}
