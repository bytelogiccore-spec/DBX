//! Replication 프로토콜 메시지 정의

use serde::{Deserialize, Serialize};

/// Replication 메시지 타입
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReplicationMessage {
    /// WAL 항목 전송.
    /// `node_id`: 발신 노드 식별자
    /// `lsn`: 로그 시퀀스 번호
    /// `timestamp`: LWW (Last-Writer-Wins) 기반 충돌 해결용
    WalEntry {
        node_id: u32,
        lsn: u64,
        timestamp: u64,
        data: Vec<u8>,
    },
    /// 연결 유지 신호
    Heartbeat { node_id: u32, lsn: u64 },
    /// 특정 LSN부터 재전송 요청
    RequestFrom { node_id: u32, lsn: u64 },
    /// 동기화 완료 확인
    Acknowledge { node_id: u32, lsn: u64 },
    /// 마스터 승격(Failover) 표를 요청
    /// `term`: 선출 임기 번호 (중복 선출 방지)
    VoteRequest {
        node_id: u32,
        term: u64,
        last_lsn: u64,
    },
    /// 마스터 승격(Failover) 투표 응답
    /// `voter_id`: 투표한 노드 ID (집계용)
    VoteResponse {
        node_id: u32,
        voter_id: u32,
        term: u64,
        granted: bool,
    },
    /// 새 마스터 승격 알림
    Promotion { node_id: u32, term: u64 },
}

impl ReplicationMessage {
    /// 메시지의 LSN 반환
    pub fn lsn(&self) -> u64 {
        match self {
            ReplicationMessage::WalEntry { lsn, .. } => *lsn,
            ReplicationMessage::Heartbeat { lsn, .. } => *lsn,
            ReplicationMessage::RequestFrom { lsn, .. } => *lsn,
            ReplicationMessage::Acknowledge { lsn, .. } => *lsn,
            ReplicationMessage::VoteRequest { last_lsn, .. } => *last_lsn,
            _ => 0,
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
            node_id: 1,
            lsn: 42,
            timestamp: 123456789,
            data: vec![1, 2, 3],
        };
        assert_eq!(msg.lsn(), 42);
        assert!(msg.is_wal_entry());
    }

    #[test]
    fn test_heartbeat() {
        let msg = ReplicationMessage::Heartbeat {
            node_id: 1,
            lsn: 100,
        };
        assert_eq!(msg.lsn(), 100);
        assert!(!msg.is_wal_entry());
    }
}
