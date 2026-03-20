//! Grid 통신 멀티플렉싱 프로토콜 정의
//!
//! 단일 네트워크 통신망을 통해 복제, 분산 트랜잭션, 분산 락킹 등을
//! 효율적으로 다중화(Multiplexing)하기 위한 메시지 컨테이너입니다.

use crate::replication::protocol::ReplicationMessage;
use serde::{Deserialize, Serialize};

/// 단일 QUIC/TCP 전송 계층에서 교환되는 최상위 그리드 통신 메시지
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GridMessage {
    /// 하위 호환 및 기존 마스터-슬레이브 복제용 메시지 래핑
    Replication(ReplicationMessage),

    /// Network-Aware Lock Manager 제어 메시지
    Lock(LockMessage),
}

/// 분산 락(Network Lock) 전용 프로토콜
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LockMessage {
    /// 특정 테이블 및 키에 대한 Lease 획득 요청
    Acquire {
        table: String,
        key: Vec<u8>,
        lease_ms: u64,
        node_id: u32,
        req_id: u64,
    },
    /// Lock 획득 요청에 대한 응답
    AcquireAck {
        req_id: u64,
        granted: bool,
        fencing_token: u64,
    },
    /// 성공적으로 사용을 마친 후 Lock 반환
    Release {
        table: String,
        key: Vec<u8>,
        fencing_token: u64,
        node_id: u32,
    },
    /// 네트워크 단절에 의한 Lock 탈취 방지용 Heartbeat 만료 연장
    Heartbeat {
        node_id: u32,
        fencing_tokens: Vec<u64>,
    },
}

impl GridMessage {
    /// 이 메시지가 Replication 부류인지 검사합니다.
    pub fn is_replication(&self) -> bool {
        matches!(self, GridMessage::Replication(_))
    }

    /// 이 메시지가 Lock 제어 부류인지 검사합니다.
    pub fn is_lock(&self) -> bool {
        matches!(self, GridMessage::Lock(_))
    }
}
