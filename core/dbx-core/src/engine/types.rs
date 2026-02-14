//! Database Engine Types — enums and type definitions

/// WAL 내구성 수준 정책
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum DurabilityLevel {
    /// 모든 쓰기 작업에 대해 WAL fsync 수행 (최대 안전)
    Full = 0,
    /// 성능을 위해 WAL fsync를 백그라운드에서 지연 수행 (Lazy WAL)
    Lazy = 1,
    /// 성능 최대화를 위해 WAL을 아예 기록하지 않음 (메모리 전용 내구성)
    None = 2,
}

/// 백그라운드에서 처리할 작업 정의
#[derive(Clone)]
pub(crate) enum BackgroundJob {
    /// WAL 동기화 (fsync)
    WalSync,
    /// 암호화된 WAL 동기화
    EncryptedWalSync,
    /// 인덱스 업데이트
    IndexUpdate {
        table: String,
        column: String,
        key: Vec<u8>,
        row_id: usize,
    },
}
