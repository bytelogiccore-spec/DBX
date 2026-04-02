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

    /// 분산 스토리지(EC 샤드 등) 제어 메시지
    Storage(StorageMessage),

    /// 분산 쿼리(스트리밍, 집계 등) 실행 메시지
    Query(QueryMessage),
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

/// 스토리지(Grid EC) 전용 프로토콜
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageMessage {
    /// 샤드 저장 요청
    StoreShard {
        key: String,
        shard_id: usize,
        data: Vec<u8>,
    },
    /// 샤드 조회 요청
    FetchShard { key: String, shard_id: usize },
    /// 샤드 응답
    ShardResponse {
        key: String,
        shard_id: usize,
        data: Option<Vec<u8>>,
    },
}

/// 분산 쿼리 코디네이팅 전용 프로토콜
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryMessage {
    /// 하위 노드에 쿼리 파편(Fragment) 실행 요청
    ExecuteFragment {
        execution_id: String,
        stage_id: usize,
        /// bincode 직렬화된 여러 개의 PhysicalPlan 바이너리
        plans_bytes: Vec<Vec<u8>>,
        /// 코디네이터 주소 — 워커가 결과를 역전송할 목적지
        coordinator_addr: String,
    },
    /// 코디네이터에게 특정 워커가 스테이지의 모든 플랜 실행을 완료했음을 보고
    FragmentCompleted {
        execution_id: String,
        stage_id: usize,
    },
    /// 코디네이터로 RecordBatch 배압(Backpressure) 전송 스트림. (Arrow IPC 포맷 - FlatBuffer 내장)
    ExchangeData {
        execution_id: String,
        /// (신규) 다중 Exchange 스트림을 식별하기 위한 고유 ID
        exchange_id: usize,
        /// 송신 워커 노드 식별자 (멀티 워커 집계 시 출처 추적용)
        node_id: u32,
        is_eof: bool,        // 데이터 전송 완료 스트림 플래그
        batch_data: Vec<u8>, // Arrow IPC (FlatBuffer metadata + raw payload)
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

    /// 이 메시지가 스토리지 제어 부류인지 검사합니다.
    pub fn is_storage(&self) -> bool {
        matches!(self, GridMessage::Storage(_))
    }

    /// 이 메시지가 쿼리 스트리밍 제어 부류인지 검사합니다.
    pub fn is_query(&self) -> bool {
        matches!(self, GridMessage::Query(_))
    }

    /// bincode를 사용해 메시지를 직렬화합니다.
    pub fn serialize(&self) -> crate::error::DbxResult<Vec<u8>> {
        bincode::serialize(self).map_err(|e| crate::error::DbxError::Serialization(e.to_string()))
    }

    /// bincode를 사용해 바이트 배열로부터 메시지를 역직렬화합니다.
    pub fn deserialize(bytes: &[u8]) -> crate::error::DbxResult<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| crate::error::DbxError::Serialization(e.to_string()))
    }
}

/// Arrow RecordBatch → Arrow IPC 바이너리 직렬화
pub fn serialize_batch_to_ipc(
    batch: &arrow::array::RecordBatch,
) -> crate::error::DbxResult<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut writer = arrow::ipc::writer::StreamWriter::try_new(&mut buf, &batch.schema())
            .map_err(|e| crate::error::DbxError::Serialization(e.to_string()))?;
        writer
            .write(batch)
            .map_err(|e| crate::error::DbxError::Serialization(e.to_string()))?;
        writer
            .finish()
            .map_err(|e| crate::error::DbxError::Serialization(e.to_string()))?;
    }
    Ok(buf)
}
