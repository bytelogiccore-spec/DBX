//! Distributed Lock Manager (DLM)
//!
//! 그리드 환경 내에서 분산 트랜잭션, 원자적 연산을 수행할 때
//! Lease 및 Fencing Token 기반으로 안전하게 동시성을 제어하는 매니저입니다.

use crate::grid::protocol::LockMessage;
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot};

/// Network-Aware 분산 락 에러
#[derive(Debug)]
pub enum DlmError {
    Timeout,
    Denied,
    NetworkError,
}

pub struct DistributedLockManager {
    /// 현재 노드의 고유 ID
    pub node_id: u32,
    /// 락 인가를 총괄할 마스터(또는 특정 파티션의 리더) 노드 ID
    pub coordinator_id: u32,

    /// 라우터로부터 들어오는 Lock 채널 (Inbound)
    lock_in_rx: tokio::sync::Mutex<broadcast::Receiver<LockMessage>>,

    /// 라우터/네트워크로 나가는 Lock 채널 (Outbound)
    lock_out_tx: broadcast::Sender<LockMessage>,

    /// [클라이언트 용] 네트워크 응답을 대기 중인 로컬 채널들 `req_id -> sender`
    pending_reqs: DashMap<u64, oneshot::Sender<(bool, u64)>>,

    /// [코디네이터 용] 현재 발급된 락 목록 `(table, key) -> (owner_node_id, fencing_token, expiration_time)`
    granted_locks: DashMap<(String, Vec<u8>), (u32, u64, Instant)>,

    /// Fencing Token 채번기
    next_fencing_token: AtomicU64,
    /// Request ID 채번기
    next_req_id: AtomicU64,
}

impl DistributedLockManager {
    pub fn new(
        node_id: u32,
        coordinator_id: u32,
        lock_in_rx: broadcast::Receiver<LockMessage>,
        lock_out_tx: broadcast::Sender<LockMessage>,
    ) -> Arc<Self> {
        Arc::new(Self {
            node_id,
            coordinator_id,
            lock_in_rx: tokio::sync::Mutex::new(lock_in_rx),
            lock_out_tx,
            pending_reqs: DashMap::new(),
            granted_locks: DashMap::new(),
            next_fencing_token: AtomicU64::new(1),
            next_req_id: AtomicU64::new(1),
        })
    }

    /// [클라이언트 파트] 락을 비동기로 요청하고 획득할 때까지 대기합니다.
    pub async fn acquire(
        &self,
        table: &str,
        key: &[u8],
        lease_ms: u64,
        timeout: Duration,
    ) -> Result<u64, DlmError> {
        let req_id = self.next_req_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending_reqs.insert(req_id, tx);

        let msg = LockMessage::Acquire {
            table: table.to_string(),
            key: key.to_vec(),
            lease_ms,
            node_id: self.node_id,
            req_id,
        };

        if self.lock_out_tx.send(msg).is_err() {
            self.pending_reqs.remove(&req_id);
            return Err(DlmError::NetworkError);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok((true, fencing_token))) => Ok(fencing_token),
            Ok(Ok((false, _))) => Err(DlmError::Denied),
            _ => {
                self.pending_reqs.remove(&req_id);
                Err(DlmError::Timeout)
            }
        }
    }

    /// [클라이언트 파트] 획득한 락을 자발적으로 반환합니다.
    pub async fn release(&self, table: &str, key: &[u8], fencing_token: u64) {
        let msg = LockMessage::Release {
            table: table.to_string(),
            key: key.to_vec(),
            fencing_token,
            node_id: self.node_id,
        };
        let _ = self.lock_out_tx.send(msg);
    }

    /// [코디네이터 용] 만료된 락을 정리하고 탈취 여부를 판단하는 헬퍼 (Passive Eviction)
    fn try_grant_lock(
        &self,
        table: String,
        key: Vec<u8>,
        node_id: u32,
        lease_ms: u64,
    ) -> (bool, u64) {
        let lock_key = (table, key);
        let now = Instant::now();
        let expiration = now + Duration::from_millis(lease_ms);

        let mut granted = false;
        let mut f_token = 0;

        // 기존 락 확인
        if let Some(mut existing) = self.granted_locks.get_mut(&lock_key) {
            // 이미 사용 중인 락인데 만료되었는지 체크 (Passive Eviction)
            if now > existing.2 {
                // Lease 시간이 초과되었으므로 락을 빼앗아 새 노드에게 부여
                f_token = self.next_fencing_token.fetch_add(1, Ordering::SeqCst);
                existing.0 = node_id;
                existing.1 = f_token;
                existing.2 = expiration;
                granted = true;
            }
        } else {
            // 락이 비어있음 -> 즉시 획득
            f_token = self.next_fencing_token.fetch_add(1, Ordering::SeqCst);
            self.granted_locks
                .insert(lock_key, (node_id, f_token, expiration));
            granted = true;
        }

        (granted, f_token)
    }

    /// [백그라운드 루프] Inbound로 들어오는 `LockMessage` 지속 처리
    pub async fn run_receiver_loop(self: Arc<Self>) {
        let mut rx = self.lock_in_rx.lock().await;

        while let Ok(msg) = rx.recv().await {
            match msg {
                LockMessage::Acquire {
                    table,
                    key,
                    lease_ms,
                    node_id,
                    req_id,
                } => {
                    if self.node_id == self.coordinator_id {
                        let (granted, fencing_token) =
                            self.try_grant_lock(table, key, node_id, lease_ms);
                        let _ = self.lock_out_tx.send(LockMessage::AcquireAck {
                            req_id,
                            granted,
                            fencing_token,
                        });
                    }
                }

                LockMessage::AcquireAck {
                    req_id,
                    granted,
                    fencing_token,
                } => {
                    if let Some((_, sender)) = self.pending_reqs.remove(&req_id) {
                        let _ = sender.send((granted, fencing_token));
                    }
                }

                LockMessage::Release {
                    table,
                    key,
                    fencing_token,
                    node_id,
                } => {
                    if self.node_id == self.coordinator_id {
                        let lock_key = (table.clone(), key.clone());
                        if let Some(entry) = self.granted_locks.get(&lock_key)
                            && entry.0 == node_id
                            && entry.1 == fencing_token
                        {
                            drop(entry);
                            self.granted_locks.remove(&lock_key);
                        }
                    }
                }

                LockMessage::Heartbeat {
                    node_id,
                    fencing_tokens,
                } => {
                    if self.node_id == self.coordinator_id {
                        let now = Instant::now();
                        // Heartbeat가 들어온 락에 대해 5000ms 유지 연장
                        let extension = Duration::from_millis(5000);

                        // 현재 코디네이터가 쥐고 있는 모든 락을 순회하며 Token 매칭 갱신
                        for mut entry in self.granted_locks.iter_mut() {
                            let (owner, token, exp) = entry.value_mut();
                            if *owner == node_id && fencing_tokens.contains(token) {
                                // 현재 시간으로부터 5초 추가 연장
                                *exp = now + extension;
                            }
                        }
                    }
                }
            }
        }
    }
}
