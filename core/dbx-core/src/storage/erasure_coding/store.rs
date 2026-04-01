use crate::error::{DbxError, DbxResult};
use reed_solomon_erasure::galois_8::ReedSolomon;
use std::fs;
use std::path::{Path, PathBuf};

/// Erasure Coding Store (Cold Tier)
/// 
/// 데이터를 K개의 데이터 청크와 M개의 패리티 청크로 나누어 디스크(혹은 원격 노드)에 저장합니다.
pub struct ErasureCodingStore {
    base_dir: PathBuf,
    k: usize,
    m: usize,
}

impl ErasureCodingStore {
    pub fn new<P: AsRef<Path>>(base_dir: P, k: u8, m: u8) -> Self {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir).unwrap_or_default();
        Self {
            base_dir,
            k: k as usize,
            m: m as usize,
        }
    }

    /// 인코딩 (데이터 -> K 프래그먼트 + M 패리티) 후 디스크에 저장
    pub fn encode_and_store(&self, key: &str, data: &[u8]) -> DbxResult<()> {
        let rs = ReedSolomon::new(self.k, self.m).map_err(|e| DbxError::Storage(e.to_string()))?;
        
        let chunk_size = (data.len() + self.k - 1) / self.k;
        let mut shards = vec![vec![0u8; chunk_size]; self.k + self.m];

        // 1. 데이터 파티셔닝
        for (i, shard) in shards.iter_mut().take(self.k).enumerate() {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, data.len());
            if start < data.len() {
                let len = end - start;
                shard[..len].copy_from_slice(&data[start..end]);
            }
        }

        // 2. 패리티 생성
        rs.encode(&mut shards).map_err(|e| DbxError::Storage(e.to_string()))?;

        // 3. 디스크에 샤드 기록 (시뮬레이션 용 로컬 폴더 분산)
        let object_dir = self.base_dir.join(key);
        fs::create_dir_all(&object_dir)?;

        for (i, shard) in shards.iter().enumerate() {
            let shard_path = object_dir.join(format!("shard_{}.blk", i));
            fs::write(&shard_path, shard)?;
        }

        // 4. 메타데이터 (원본 길이) 기록
        fs::write(object_dir.join("metadata.json"), format!("{{\"length\":{}}}", data.len()))?;

        Ok(())
    }

    /// 복구 (K 프래그먼트 + M 패리티 중 K개 이상으로 데이터 복원)
    pub fn retrieve_and_decode(&self, key: &str) -> DbxResult<Option<Vec<u8>>> {
        let object_dir = self.base_dir.join(key);
        if !object_dir.exists() {
            return Ok(None);
        }

        let meta_str = fs::read_to_string(object_dir.join("metadata.json"))?;
        
        // JSON 파싱 (간이 구현)
        let length_str = meta_str.split(':').nth(1).unwrap_or("0").trim_end_matches('}').trim();
        let original_len: usize = length_str.parse().unwrap_or(0);

        let rs = ReedSolomon::new(self.k, self.m).map_err(|e| DbxError::Storage(e.to_string()))?;
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; self.k + self.m];
        
        // 1. 남아있는 샤드들 읽기
        for i in 0..(self.k + self.m) {
            let shard_path = object_dir.join(format!("shard_{}.blk", i));
            if let Ok(data) = fs::read(&shard_path) {
                shards[i] = Some(data);
            }
        }

        // 2. 유실된 Шад 복원
        rs.reconstruct(&mut shards).map_err(|e| DbxError::Storage(e.to_string()))?;

        // 3. 원본 데이터 합치기
        let mut output = Vec::with_capacity(original_len);
        let mut bytes_written = 0;
        
        for shard in shards.into_iter().take(self.k) {
            if let Some(data) = shard {
                let remaining = original_len - bytes_written;
                let take = std::cmp::min(remaining, data.len());
                output.extend_from_slice(&data[..take]);
                bytes_written += take;
            }
        }

        Ok(Some(output))
    }
    
    /// Delete EC fragments
    pub fn delete(&self, key: &str) -> DbxResult<bool> {
        let object_dir = self.base_dir.join(key);
        if object_dir.exists() {
            fs::remove_dir_all(object_dir)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_erasure_coding_store() {
        let dir = tempdir().unwrap();
        let store = ErasureCodingStore::new(dir.path(), 4, 2);

        let key = "test_object_1";
        let data = b"Hello World Erasure Coding Testing!";

        // 1. 저장 (Encode)
        store.encode_and_store(key, data).unwrap();

        // 2. 샤드 삭제 시뮬레이션 (2개 유실)
        let obj_dir = dir.path().join(key);
        fs::remove_file(obj_dir.join("shard_0.blk")).unwrap();
        fs::remove_file(obj_dir.join("shard_3.blk")).unwrap();

        // 3. 복원 (Retrieve & Decode)
        let recovered = store.retrieve_and_decode(key).unwrap().unwrap();
        assert_eq!(recovered, data);
    }
}
