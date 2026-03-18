//! Replication 프로토콜 메시지 정의

use serde::{Deserialize, Serialize};

/// Replication 메시지 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationMessage {
    /// WAL 항목 전송. lsn = 로그 시퀀스 번호.
    WalEntry { lsn: u64, data: Vec<u8> },
    /// 연결 유지 신호
    Heartbeat { lsn: u64 },
    /// 특정 LSN부터 재전송 요청 (Slave → Master)
    RequestFrom { lsn: u64 },
    /// 동기화 완료 확인 (Slave → Master)
    Acknowledge { lsn: u64 },
}

impl ReplicationMessage {
    /// 메시지의 LSN 반환
    pub fn lsn(&self) -> u64 {
        match self {
            ReplicationMessage::WalEntry { lsn, .. } => *lsn,
            ReplicationMessage::Heartbeat { lsn } => *lsn,
            ReplicationMessage::RequestFrom { lsn } => *lsn,
            ReplicationMessage::Acknowledge { lsn } => *lsn,
        }
    }

    /// WAL Entry 여부
    pub fn is_wal_entry(&self) -> bool {
        matches!(self, ReplicationMessage::WalEntry { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_lsn() {
        let msg = ReplicationMessage::WalEntry {
            lsn: 42,
            data: vec![1, 2, 3],
        };
        assert_eq!(msg.lsn(), 42);
        assert!(msg.is_wal_entry());
    }

    #[test]
    fn test_heartbeat() {
        let msg = ReplicationMessage::Heartbeat { lsn: 100 };
        assert_eq!(msg.lsn(), 100);
        assert!(!msg.is_wal_entry());
    }
}
