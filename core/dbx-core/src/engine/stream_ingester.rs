//! 스트리밍 수집 파이프라인 — CDC 스타일 INSERT/UPDATE/DELETE 이벤트 처리
//!
//! # 개요
//!
//! `StreamIngester`는 채널 기반의 스트리밍 수집 파이프라인을 제공합니다.
//! Kafka, Kinesis 등 메시지 스트림에서 수신하는 CDC 이벤트(INSERT/UPDATE/DELETE)를
//! `StreamEvent` enum으로 표현하여 background 스레드가 DBX에 일괄 반영합니다.
//!
//! # 사용 예
//!
//! ```rust,no_run
//! use dbx_core::{Database, engine::stream_ingester::{StreamIngester, StreamEvent}};
//! use std::{sync::Arc, time::Duration};
//!
//! let db = Arc::new(Database::open_in_memory().unwrap());
//! let ingester = StreamIngester::new(Arc::clone(&db), "orders", 1000, Duration::from_millis(100));
//! let tx = ingester.sender();
//!
//! tx.send(vec![
//!     StreamEvent::Insert { key: "order:1".into(), value: b"[1, \"pending\"]".to_vec() },
//!     StreamEvent::Update { key: "order:2".into(), value: b"[2, \"shipped\"]".to_vec() },
//!     StreamEvent::Delete { key: "order:3".into() },
//! ]).unwrap();
//!
//! ingester.flush().unwrap();
//! ```

use crate::engine::Database;
use crate::error::DbxResult;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

/// 스트림 이벤트 — INSERT / UPDATE / DELETE를 통합 표현
///
/// CDC (Change Data Capture) 및 이벤트 소싱 패턴에서
/// 스트림으로 전달되는 모든 DML 연산을 지원합니다.
///
/// # 주의
///
/// `Update`는 DBX의 key 기반 upsert 시맨틱을 활용하므로
/// 내부적으로 `db.insert()`를 재사용합니다.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 새 레코드 삽입
    Insert { key: String, value: Vec<u8> },
    /// 기존 레코드 갱신 (키 기반 upsert)
    Update { key: String, value: Vec<u8> },
    /// 레코드 삭제 (키 기반)
    Delete { key: String },
}

/// 채널 기반 스트리밍 수집 파이프라인
///
/// 백그라운드 스레드가 채널에서 `StreamEvent` 배치를 수신하여
/// `batch_size` 또는 `max_latency` 조건 충족 시 DBX에 일괄 반영합니다.
pub struct StreamIngester {
    sender: mpsc::SyncSender<Vec<StreamEvent>>,
    _handle: thread::JoinHandle<()>,
}

impl StreamIngester {
    /// 스트림 인제스터 생성
    ///
    /// # 인수
    ///
    /// * `db` - 대상 데이터베이스 (Arc 공유)
    /// * `table` - 이벤트를 적용할 테이블 이름
    /// * `batch_size` - 이 이벤트 수에 도달하면 즉시 flush
    /// * `max_latency` - 버퍼가 차지 않아도 이 시간마다 강제 flush
    pub fn new(
        db: Arc<Database>,
        table: &str,
        batch_size: usize,
        max_latency: Duration,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Vec<StreamEvent>>(batch_size * 4);
        let table = table.to_string();

        let handle = thread::spawn(move || {
            let mut buffer: Vec<StreamEvent> = Vec::with_capacity(batch_size);
            let mut deadline = std::time::Instant::now() + max_latency;

            loop {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                match rx.recv_timeout(remaining) {
                    Ok(events) => buffer.extend(events),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // 채널 닫힘: 남은 버퍼 모두 flush 후 종료
                        Self::flush_buffer(&db, &table, &mut buffer);
                        break;
                    }
                }

                if buffer.len() >= batch_size || std::time::Instant::now() >= deadline {
                    Self::flush_buffer(&db, &table, &mut buffer);
                    deadline = std::time::Instant::now() + max_latency;
                }
            }
        });

        Self {
            sender: tx,
            _handle: handle,
        }
    }

    /// 버퍼의 이벤트를 DB에 적용
    ///
    /// DML 종류에 따라 insert / delete를 선택합니다.
    fn flush_buffer(db: &Database, table: &str, buffer: &mut Vec<StreamEvent>) {
        for event in buffer.drain(..) {
            match event {
                StreamEvent::Insert { key, value } => {
                    let _ = db.insert(table, key.as_bytes(), &value);
                }
                StreamEvent::Update { key, value } => {
                    // UPDATE = key 기반 upsert (DBX insert는 덮어쓰기 시맨틱)
                    let _ = db.insert(table, key.as_bytes(), &value);
                }
                StreamEvent::Delete { key } => {
                    let _ = db.delete(table, key.as_bytes());
                }
            }
        }
    }

    /// 이벤트 배치 송신용 sender 클론 반환
    ///
    /// 여러 스레드에서 동시에 이벤트를 전송할 수 있습니다.
    pub fn sender(&self) -> mpsc::SyncSender<Vec<StreamEvent>> {
        self.sender.clone()
    }

    /// 채널을 닫고, 백그라운드 스레드가 남은 이벤트를 모두 flush한 뒤 종료할 때까지 대기
    ///
    /// 이 메서드 호출 후에는 `sender`를 통한 이벤트 전송이 불가능합니다.
    pub fn flush(self) -> DbxResult<()> {
        // sender를 drop하면 채널이 닫힘 → 백그라운드 스레드 Disconnected 감지
        // → 남은 이벤트 flush → 스레드 종료
        drop(self.sender);
        self._handle.join().ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_db() -> Arc<Database> {
        Arc::new(Database::open_in_memory().unwrap())
    }

    #[test]
    fn test_stream_ingester_insert_update_delete() {
        let db = make_db();
        let ingester = StreamIngester::new(Arc::clone(&db), "kv", 100, Duration::from_millis(50));
        let tx = ingester.sender();

        // INSERT 2건
        tx.send(vec![
            StreamEvent::Insert { key: "k1".into(), value: b"v1".to_vec() },
            StreamEvent::Insert { key: "k2".into(), value: b"v2".to_vec() },
        ]).unwrap();

        // UPDATE k1
        tx.send(vec![StreamEvent::Update {
            key: "k1".into(),
            value: b"v1-updated".to_vec(),
        }]).unwrap();

        // DELETE k2
        tx.send(vec![StreamEvent::Delete { key: "k2".into() }]).unwrap();

        // ⚠️ [핵심 원인 해결]
        // `tx`가 아직 살아있으면 백그라운드 스레드는 채널이 닫혔다고 판단하지 않고
        // recv_timeout 상태에서 무한 대기(Deadlock)에 빠집니다.
        // `ingester.flush()`가 내부적으로 자신의 sender를 버리지만,
        // 로컬 변수인 `tx`가 여전히 활성화되어 있기 때문입니다.
        drop(tx);

        ingester.flush().unwrap();

        // k1은 갱신된 값, k2는 삭제됨
        let v1 = db.get("kv", b"k1").unwrap();
        assert_eq!(v1, Some(b"v1-updated".to_vec()));
        let v2 = db.get("kv", b"k2").unwrap();
        assert!(v2.is_none());
    }

    #[test]
    fn test_stream_ingester_high_throughput() {
        let db = make_db();
        let ingester = StreamIngester::new(
            Arc::clone(&db), "telemetry", 500, Duration::from_millis(100),
        );
        let tx = ingester.sender();

        // 1,000건 INSERT (10 배치 × 100건)
        for i in 0u32..10 {
            let batch: Vec<StreamEvent> = (0..100)
                .map(|j| StreamEvent::Insert {
                    key: format!("key_{}", i * 100 + j),
                    value: format!("val_{}", i * 100 + j).into_bytes(),
                })
                .collect();
            tx.send(batch).unwrap();
        }

        drop(tx); // Deadlock 방지
        ingester.flush().unwrap();

        // 샘플링 검증 (key_0 ~ key_9 존재 확인)
        for i in 0..10u32 {
            let key = format!("key_{}", i);
            assert!(
                db.get("telemetry", key.as_bytes()).unwrap().is_some(),
                "key_{} should exist",
                i
            );
        }
    }

    #[test]
    fn test_stream_ingester_mixed_dml() {
        let db = make_db();
        let ingester = StreamIngester::new(
            Arc::clone(&db), "inventory", 500, Duration::from_millis(50),
        );
        let tx = ingester.sender();

        // 100건 INSERT
        let inserts: Vec<StreamEvent> = (0u32..100)
            .map(|i| StreamEvent::Insert {
                key: format!("item_{}", i),
                value: format!("qty={}", 10).into_bytes(),
            })
            .collect();
        tx.send(inserts).unwrap();

        // 짝수 key DELETE (0,2,4,6,8 → 5건)
        let deletes: Vec<StreamEvent> = (0u32..100)
            .filter(|i| i % 2 == 0)
            .map(|i| StreamEvent::Delete { key: format!("item_{}", i) })
            .collect();
        tx.send(deletes).unwrap();

        drop(tx); // Deadlock 방지
        ingester.flush().unwrap();

        // 홀수 key는 존재, 짝수 key는 없어야 함
        for i in 0u32..100 {
            let key = format!("item_{}", i);
            let val = db.get("inventory", key.as_bytes()).unwrap();
            if i % 2 == 1 {
                assert!(val.is_some(), "item_{} should exist", i);
            } else {
                assert!(val.is_none(), "item_{} should be deleted", i);
            }
        }
    }
}
