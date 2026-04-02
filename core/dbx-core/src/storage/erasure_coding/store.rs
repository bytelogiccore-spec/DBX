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

    pub fn k(&self) -> usize {
        self.k
    }
    pub fn m(&self) -> usize {
        self.m
    }

    /// 인코딩 (데이터 -> K 프래그먼트 + M 패리티)
    pub fn encode(&self, data: &[u8]) -> DbxResult<Vec<Vec<u8>>> {
        let rs = ReedSolomon::new(self.k, self.m).map_err(|e| DbxError::Storage(e.to_string()))?;

        let chunk_size = data.len().div_ceil(self.k);
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
        rs.encode(&mut shards)
            .map_err(|e| DbxError::Storage(e.to_string()))?;
        Ok(shards)
    }

    /// 개별 샤드를 디스크에 저장
    pub fn store_shard(&self, key: &str, shard_id: usize, data: &[u8]) -> DbxResult<()> {
        let object_dir = self.base_dir.join(key);
        fs::create_dir_all(&object_dir)?;
        let shard_path = object_dir.join(format!("shard_{}.blk", shard_id));
        fs::write(&shard_path, data)?;
        Ok(())
    }

    /// 샤드를 디스크에서 읽기
    pub fn fetch_shard(&self, key: &str, shard_id: usize) -> DbxResult<Option<Vec<u8>>> {
        let shard_path = self
            .base_dir
            .join(key)
            .join(format!("shard_{}.blk", shard_id));
        if shard_path.exists() {
            Ok(Some(fs::read(shard_path)?))
        } else {
            Ok(None)
        }
    }

    /// 샤드들을 디스크에 저장
    pub fn store_shards(
        &self,
        key: &str,
        shards: &[Vec<u8>],
        original_len: usize,
    ) -> DbxResult<()> {
        for (i, shard) in shards.iter().enumerate() {
            self.store_shard(key, i, shard)?;
        }

        let object_dir = self.base_dir.join(key);
        // 메타데이터 (원본 길이) 기록
        fs::write(
            object_dir.join("metadata.json"),
            format!("{{\"length\":{}}}", original_len),
        )?;
        Ok(())
    }

    /// 인코딩 후 디스크에 저장 (편의용 메서드)
    pub fn encode_and_store(&self, key: &str, data: &[u8]) -> DbxResult<()> {
        let shards = self.encode(data)?;
        self.store_shards(key, &shards, data.len())
    }

    /// 샤드들로부터 데이터 복구
    pub fn decode(
        &self,
        mut shards: Vec<Option<Vec<u8>>>,
        original_len: usize,
    ) -> DbxResult<Vec<u8>> {
        let rs = ReedSolomon::new(self.k, self.m).map_err(|e| DbxError::Storage(e.to_string()))?;

        // 1. 유실된 샤드 복원
        rs.reconstruct(&mut shards)
            .map_err(|e| DbxError::Storage(e.to_string()))?;

        // 2. 원본 데이터 합치기
        // original_len이 usize::MAX 같은 비정상적인 값일 경우를 대비해 capacity 제한
        let chunk_size = shards.iter().flatten().next().map(|s| s.len()).unwrap_or(0);
        let max_len = chunk_size * self.k;
        let capacity = std::cmp::min(original_len, max_len);
        let mut output = Vec::with_capacity(capacity);

        let mut bytes_written = 0;

        for data in shards.into_iter().take(self.k).flatten() {
            let remaining = original_len.saturating_sub(bytes_written);
            let take = std::cmp::min(remaining, data.len());
            output.extend_from_slice(&data[..take]);
            bytes_written += take;
        }

        Ok(output)
    }

    /// 복구 (K 프래그먼트 + M 패리티 중 K개 이상으로 데이터 복원)
    pub fn retrieve_and_decode(&self, key: &str) -> DbxResult<Option<Vec<u8>>> {
        let object_dir = self.base_dir.join(key);
        if !object_dir.exists() {
            return Ok(None);
        }

        let meta_str = fs::read_to_string(object_dir.join("metadata.json"))?;

        // JSON 파싱 (간이 구현)
        let length_str = meta_str
            .split(':')
            .nth(1)
            .unwrap_or("0")
            .trim_end_matches('}')
            .trim();
        let original_len: usize = length_str.parse().unwrap_or(0);

        let rs = ReedSolomon::new(self.k, self.m).map_err(|e| DbxError::Storage(e.to_string()))?;
        let mut shards: Vec<Option<Vec<u8>>> = vec![None; self.k + self.m];

        // 1. 남아있는 샤드들 읽기
        for (i, shard) in shards.iter_mut().enumerate().take(self.k + self.m) {
            let shard_path = object_dir.join(format!("shard_{}.blk", i));
            if let Ok(data) = fs::read(&shard_path) {
                *shard = Some(data);
            }
        }

        // 2. 유실된 Шад 복원
        rs.reconstruct(&mut shards)
            .map_err(|e| DbxError::Storage(e.to_string()))?;

        // 3. 원본 데이터 합치기
        let mut output = Vec::with_capacity(original_len);
        let mut bytes_written = 0;

        for data in shards.into_iter().take(self.k).flatten() {
            let remaining = original_len - bytes_written;
            let take = std::cmp::min(remaining, data.len());
            output.extend_from_slice(&data[..take]);
            bytes_written += take;
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
