//! HTAP 실시간 동기화 설정
//!
//! Delta Store에 insert 후 Columnar Cache로 즉시 또는 비동기 배치로 전파하는
//! 실시간 동기화 모드를 정의합니다.
//!
//! # 동기화 모드
//!
//! - [`SyncMode::Immediate`] — 모든 insert 후 즉시 동기화
//! - [`SyncMode::Threshold(n)`] — n행 쌓이면 자동 flush (기존 방식)
//! - [`SyncMode::AsyncBatch`] — 백그라운드 스레드, max_latency_ms 주기 flush

/// 실시간 동기화 모드
#[derive(Debug, Clone, PartialEq)]
pub enum SyncMode {
    /// 모든 insert 직후 즉시 columnar cache에 반영
    Immediate,
    /// Delta 항목 수 >= threshold 이면 자동 flush (기존 방식)
    Threshold(usize),
    /// 백그라운드 스레드가 max_latency_ms마다 flush
    AsyncBatch { max_latency_ms: u64 },
}

/// HTAP 실시간 동기화 설정
#[derive(Debug, Clone)]
pub struct RealtimeSyncConfig {
    /// 동기화 모드
    pub mode: SyncMode,
    /// 배치 크기 (flush 당 최대 처리 행 수)
    pub batch_size: usize,
}

impl Default for RealtimeSyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::Threshold(10_000),
            batch_size: 1_000,
        }
    }
}

impl RealtimeSyncConfig {
    /// Immediate 동기화 설정 생성
    pub fn immediate() -> Self {
        Self {
            mode: SyncMode::Immediate,
            batch_size: 1,
        }
    }

    /// Threshold 기반 동기화 설정 생성
    pub fn threshold(n: usize) -> Self {
        Self {
            mode: SyncMode::Threshold(n),
            batch_size: n.min(10_000),
        }
    }

    /// 비동기 배치 동기화 설정 생성
    pub fn async_batch(max_latency_ms: u64, batch_size: usize) -> Self {
        Self {
            mode: SyncMode::AsyncBatch { max_latency_ms },
            batch_size,
        }
    }

    /// 현재 설정이 즉시 동기화 모드인지 확인
    pub fn is_immediate(&self) -> bool {
        matches!(self.mode, SyncMode::Immediate)
    }

    /// Delta 행 수가 flush 기준에 도달했는지 확인
    ///
    /// `Immediate` → 항상 true  
    /// `Threshold(n)` → row_count >= n  
    /// `AsyncBatch` → false (백그라운드 스레드가 결정)  
    pub fn should_flush(&self, row_count: usize) -> bool {
        match &self.mode {
            SyncMode::Immediate => true,
            SyncMode::Threshold(n) => row_count >= *n,
            SyncMode::AsyncBatch { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RealtimeSyncConfig::default();
        assert_eq!(config.mode, SyncMode::Threshold(10_000));
        assert_eq!(config.batch_size, 1_000);
    }

    #[test]
    fn test_immediate_mode() {
        let config = RealtimeSyncConfig::immediate();
        assert!(config.is_immediate());
        assert!(config.should_flush(0));
        assert!(config.should_flush(1_000_000));
    }

    #[test]
    fn test_threshold_should_flush() {
        let config = RealtimeSyncConfig::threshold(100);
        assert!(!config.should_flush(99));
        assert!(config.should_flush(100));
        assert!(config.should_flush(500));
    }

    #[test]
    fn test_async_batch_never_auto_flush() {
        let config = RealtimeSyncConfig::async_batch(50, 1_000);
        assert!(!config.should_flush(0));
        assert!(!config.should_flush(1_000_000));
        assert!(!config.is_immediate());
        assert_eq!(config.mode, SyncMode::AsyncBatch { max_latency_ms: 50 });
    }

    #[test]
    fn test_clone_and_debug() {
        let config = RealtimeSyncConfig::immediate();
        let cloned = config.clone();
        assert!(cloned.is_immediate());
        // Debug 출력 가능 확인
        let _ = format!("{:?}", cloned);
    }
}
