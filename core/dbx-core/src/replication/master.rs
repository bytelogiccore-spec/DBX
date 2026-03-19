//! Replication Master — WAL 변경사항을 Slave로 브로드캐스트

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;

use crate::replication::protocol::ReplicationMessage;

/// Replication Master
///
/// WAL append 시 브로드캐스트 채널로 메시지를 전송합니다.
/// 인메모리 `tokio::sync::broadcast` 채널 기반 MVP 구현.
pub struct ReplicationMaster {
    /// 브로드캐스트 송신자
    tx: broadcast::Sender<ReplicationMessage>,
    /// 현재 LSN (단조 증가)
    current_lsn: Arc<AtomicU64>,
}

impl ReplicationMaster {
    /// 새 Master 생성. capacity는 채널 버퍼 크기.
    pub fn new(capacity: usize) -> (Self, broadcast::Receiver<ReplicationMessage>) {
        let (tx, rx) = broadcast::channel(capacity);
        let master = Self {
            tx,
            current_lsn: Arc::new(AtomicU64::new(0)),
        };
        (master, rx)
    }

    /// WAL 데이터를 복제로 전송. LSN을 자동 증가.
    ///
    /// `data` — 직렬화된 WAL 레코드 바이트
    pub fn replicate(&self, data: Vec<u8>) -> u64 {
        let lsn = self.current_lsn.fetch_add(1, Ordering::SeqCst);
        let msg = ReplicationMessage::WalEntry { 
            node_id: 0,
            lsn, 
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64,
            data 
        };
        // 수신자가 없어도 에러 무시 (Slave가 아직 연결 안 됐을 수 있음)
        let _ = self.tx.send(msg);
        lsn
    }

    /// Heartbeat 전송
    pub fn heartbeat(&self) {
        let lsn = self.current_lsn.load(Ordering::SeqCst);
        let _ = self.tx.send(ReplicationMessage::Heartbeat { node_id: 0, lsn });
    }

    /// 새 Slave를 위한 구독자 추가
    pub fn subscribe(&self) -> broadcast::Receiver<ReplicationMessage> {
        self.tx.subscribe()
    }

    /// 현재 LSN
    pub fn current_lsn(&self) -> u64 {
        self.current_lsn.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_master_replicate_increments_lsn() {
        let (master, _rx) = ReplicationMaster::new(16);
        assert_eq!(master.current_lsn(), 0);
        let lsn1 = master.replicate(b"data1".to_vec());
        let lsn2 = master.replicate(b"data2".to_vec());
        assert_eq!(lsn1, 0);
        assert_eq!(lsn2, 1);
        assert_eq!(master.current_lsn(), 2);
    }

    #[tokio::test]
    async fn test_slave_receives_wal_entry() {
        let (master, mut rx) = ReplicationMaster::new(16);
        master.replicate(b"hello".to_vec());

        let msg = rx.recv().await.unwrap();
        if let ReplicationMessage::WalEntry { lsn, data, .. } = msg {
            assert_eq!(lsn, 0);
            assert_eq!(data, b"hello");
        } else {
            panic!("WalEntry 메시지 기대");
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let (master, mut rx1) = ReplicationMaster::new(16);
        let mut rx2 = master.subscribe();

        master.replicate(b"broadcast".to_vec());

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        assert_eq!(msg1.lsn(), msg2.lsn());
    }
}
