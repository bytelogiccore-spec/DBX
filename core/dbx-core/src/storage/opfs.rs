use crate::DbxResult;

/// OPFS (Origin Private File System) 지원을 위한 StorageBackend 확장 트레이트
pub trait OpfsBackend: Send + Sync {
    /// OPFS에서 데이터 읽기
    fn opfs_read(&self, key: &[u8]) -> DbxResult<Option<Vec<u8>>>;

    /// OPFS에 데이터 쓰기
    fn opfs_write(&self, key: &[u8], value: &[u8]) -> DbxResult<()>;

    /// OPFS에서 데이터 삭제
    fn opfs_delete(&self, key: &[u8]) -> DbxResult<()>;

    /// OPFS 동기화 (fsync)
    fn opfs_sync(&self) -> DbxResult<()>;
}

// TODO: WASM 환경에서 FileSystemSyncAccessHandle을 사용한 실제 구현
// 현재는 트레이트 정의만 제공
