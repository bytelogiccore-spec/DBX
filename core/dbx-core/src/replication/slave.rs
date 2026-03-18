//! Replication Slave — Master WAL 스트림을 소비하여 로컬 DB에 재생

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::replication::protocol::ReplicationMessage;

/// Slave 재생 에러
#[derive(Debug)]
pub enum ReplayError {
    /// 채널 수신 오류 (Lagged, Closed 등)
    ChannelError(String),
    /// 재생 적용 오류
    ApplyError(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplayError::ChannelError(s) => write!(f, "channel error: {s}"),
            ReplayError::ApplyError(s) => write!(f, "apply error: {s}"),
        }
    }
}

/// Replication Slave
///
/// Master로부터 `ReplicationMessage::WalEntry`를 수신하여
/// `apply_fn` 콜백으로 로컬 DB에 재생합니다.
///
/// MVP: 인메모리 `tokio::sync::broadcast` 채널 기반.
pub struct ReplicationSlave {
    /// 마지막으로 적용된 LSN
    last_applied_lsn: Arc<AtomicU64>,
}

impl ReplicationSlave {
    pub fn new() -> Self {
        Self {
            last_applied_lsn: Arc::new(AtomicU64::new(u64::MAX)), // 아직 미적용
        }
    }

    /// 마지막으로 적용된 LSN 반환 (u64::MAX = 아직 없음)
    pub fn last_applied_lsn(&self) -> Option<u64> {
        let v = self.last_applied_lsn.load(Ordering::SeqCst);
        if v == u64::MAX { None } else { Some(v) }
    }

    /// 브로드캐스트 채널로부터 메시지를 수신하고 apply_fn으로 재생.
    ///
    /// `count` 개의 메시지를 처리 후 반환 (0 = 무한 루프).
    /// 실제 서버에서는 tokio task로 spawn.
    pub async fn replay_n<F>(
        &self,
        rx: &mut broadcast::Receiver<ReplicationMessage>,
        count: usize,
        mut apply_fn: F,
    ) -> Result<usize, ReplayError>
    where
        F: FnMut(u64, &[u8]) -> Result<(), String>,
    {
        let mut applied = 0;

        loop {
            if count > 0 && applied >= count {
                break;
            }

            let msg = rx.recv().await.map_err(|e| {
                ReplayError::ChannelError(e.to_string())
            })?;

            if let ReplicationMessage::WalEntry { lsn, data } = msg {
                // LSN 순서 검증 (선택적)
                let last = self.last_applied_lsn.load(Ordering::SeqCst);
                if last != u64::MAX && lsn <= last {
                    // 이미 적용된 LSN → 스킵
                    continue;
                }

                apply_fn(lsn, &data).map_err(ReplayError::ApplyError)?;
                self.last_applied_lsn.store(lsn, Ordering::SeqCst);
                applied += 1;
            }
            // Heartbeat 등 다른 메시지는 무시
        }

        Ok(applied)
    }
}

impl Default for ReplicationSlave {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replication::master::ReplicationMaster;

    #[tokio::test]
    async fn test_slave_replays_wal_entries() {
        let (master, mut rx) = ReplicationMaster::new(16);
        let slave = ReplicationSlave::new();

        // 3개 WAL 항목 전송
        master.replicate(b"entry_0".to_vec());
        master.replicate(b"entry_1".to_vec());
        master.replicate(b"entry_2".to_vec());

        let mut replayed = Vec::new();
        let count = slave
            .replay_n(&mut rx, 3, |lsn, data| {
                replayed.push((lsn, data.to_vec()));
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(count, 3);
        assert_eq!(slave.last_applied_lsn(), Some(2));
        assert_eq!(replayed[0], (0, b"entry_0".to_vec()));
        assert_eq!(replayed[2], (2, b"entry_2".to_vec()));
    }

    #[tokio::test]
    async fn test_slave_apply_error_propagates() {
        let (master, mut rx) = ReplicationMaster::new(16);
        let slave = ReplicationSlave::new();

        master.replicate(b"bad_data".to_vec());

        let result = slave
            .replay_n(&mut rx, 1, |_lsn, _data| {
                Err("intentional error".to_string())
            })
            .await;

        assert!(matches!(result, Err(ReplayError::ApplyError(_))));
    }
}
