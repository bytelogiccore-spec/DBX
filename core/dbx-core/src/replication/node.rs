//! Replication Node — Quorum 기반 리더 선출 + Multi-Master Failover
//!
//! ## 개선 사항 (MVP → P0)
//! - `current_term`: Raft-like 임기(term) 번호 도입으로 중복 선출 방지
//! - `voted_for`: 현 임기에 이미 투표한 경우 재투표 방지
//! - Quorum 투표 집계: `cluster_size / 2 + 1` 과반수를 얻어야만 Master 승격
//! - `Promotion` 메시지에 `term` 포함 → 구버전 Master를 Slave로 강등
//! - Split-Brain 방어: 자신보다 높은 term의 Promotion 수신 시 Slave 강등

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio::sync::{Mutex, RwLock};

use crate::replication::protocol::ReplicationMessage;

/// 노드 역할 (Master / Slave / Candidate)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Master,
    Slave,
    /// 투표 요청 후 과반 대기 중
    Candidate,
}

/// 노드 상태 에러
#[derive(Debug)]
pub enum NodeError {
    ChannelError(String),
    ApplyError(String),
}

/// 클러스터 노드 (Quorum 기반 리더 선출)
pub struct ReplicationNode {
    /// 자기 자신의 노드 ID
    pub node_id: u32,
    /// 클러스터 전체 노드 수 (Quorum 계산용)
    pub cluster_size: usize,
    /// 현재 노드 역할
    role: Arc<RwLock<NodeRole>>,
    /// 송수신용 공용 브로드캐스트 채널
    tx: broadcast::Sender<ReplicationMessage>,
    /// 마지막으로 수신/적용된 LSN
    last_lsn: Arc<AtomicU64>,
    /// 현재 임기(term) 번호
    current_term: Arc<AtomicU64>,
    /// 현 임기에 투표한 노드 ID (None = 아직 미투표)
    voted_for: Arc<Mutex<Option<u32>>>,
    /// 현 임기에 자신이 받은 투표 수 (Candidate 중에만 의미 있음)
    votes_received: Arc<Mutex<u32>>,
    /// 마지막으로 마스터 Heartbeat를 수신한 시간
    last_heartbeat: Arc<RwLock<Instant>>,
}

impl ReplicationNode {
    /// 새로운 노드 생성
    pub fn new(
        node_id: u32,
        cluster_size: usize,
        initial_role: NodeRole,
        tx: broadcast::Sender<ReplicationMessage>,
    ) -> Self {
        Self {
            node_id,
            cluster_size,
            role: Arc::new(RwLock::new(initial_role)),
            tx,
            last_lsn: Arc::new(AtomicU64::new(0)),
            current_term: Arc::new(AtomicU64::new(0)),
            voted_for: Arc::new(Mutex::new(None)),
            votes_received: Arc::new(Mutex::new(0)),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Quorum 수 (과반) — `cluster_size / 2 + 1`
    fn quorum(&self) -> u32 {
        (self.cluster_size / 2 + 1) as u32
    }

    /// 현재 역할 조회
    pub async fn role(&self) -> NodeRole {
        *self.role.read().await
    }

    /// 현재 임기 번호 조회
    pub fn term(&self) -> u64 {
        self.current_term.load(Ordering::SeqCst)
    }

    /// (Master 용) WAL 레코드 발행
    pub async fn replicate(&self, data: Vec<u8>) -> Result<u64, String> {
        let role = self.role().await;
        if role != NodeRole::Master {
            return Err("Only Master can replicate".to_string());
        }

        let lsn = self.last_lsn.fetch_add(1, Ordering::SeqCst) + 1;
        let msg = ReplicationMessage::WalEntry {
            node_id: self.node_id,
            lsn,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            data,
        };
        let _ = self.tx.send(msg);
        Ok(lsn)
    }

    /// (Master 용) Heartbeat 송신
    pub async fn send_heartbeat(&self) {
        if self.role().await != NodeRole::Master {
            return;
        }
        let lsn = self.last_lsn.load(Ordering::SeqCst);
        let _ = self.tx.send(ReplicationMessage::Heartbeat {
            node_id: self.node_id,
            lsn,
        });
    }

    /// 클러스터 메시지 수신 및 적용 루프
    pub async fn run_receiver_loop<F>(
        &self,
        mut rx: broadcast::Receiver<ReplicationMessage>,
        mut apply_fn: F,
    ) -> Result<(), NodeError>
    where
        F: FnMut(u64, u64, &[u8]) -> Result<(), String>,
    {
        loop {
            match rx.recv().await {
                Ok(msg) => self.handle_message(msg, &mut apply_fn).await?,
                Err(broadcast::error::RecvError::Closed) => {
                    return Err(NodeError::ChannelError("Channel closed".into()));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    /// 개별 메시지 처리
    async fn handle_message<F>(
        &self,
        msg: ReplicationMessage,
        apply_fn: &mut F,
    ) -> Result<(), NodeError>
    where
        F: FnMut(u64, u64, &[u8]) -> Result<(), String>,
    {
        match msg {
            // ── Heartbeat: 마스터 생존 확인 ──────────────────────────────
            ReplicationMessage::Heartbeat { node_id, .. } => {
                if node_id != self.node_id {
                    *self.last_heartbeat.write().await = Instant::now();
                }
            }

            // ── WalEntry: 복제 레코드 수신 ───────────────────────────────
            ReplicationMessage::WalEntry {
                node_id,
                lsn,
                timestamp,
                data,
            } => {
                if node_id == self.node_id {
                    return Ok(());
                }
                let local_lsn = self.last_lsn.load(Ordering::SeqCst);
                if lsn > local_lsn {
                    apply_fn(lsn, timestamp, &data).map_err(NodeError::ApplyError)?;
                    self.last_lsn.store(lsn, Ordering::SeqCst);
                }
            }

            // ── VoteRequest: 다른 노드가 투표를 요청 ─────────────────────
            ReplicationMessage::VoteRequest {
                node_id: candidate_id,
                term,
                last_lsn,
            } => {
                if candidate_id == self.node_id {
                    return Ok(());
                }
                let my_term = self.current_term.load(Ordering::SeqCst);
                let my_lsn = self.last_lsn.load(Ordering::SeqCst);

                let mut voted_for = self.voted_for.lock().await;
                // 투표 조건: (1) 요청 term이 내 term 이상 (2) 같은 term에 아직 미투표
                //             (3) 후보의 LSN이 내 LSN 이상 (데이터 최신성)
                let grant = term >= my_term
                    && (voted_for.is_none() || *voted_for == Some(candidate_id))
                    && last_lsn >= my_lsn;

                if grant {
                    *voted_for = Some(candidate_id);
                    // term 갱신
                    self.current_term.store(term, Ordering::SeqCst);
                }

                let _ = self.tx.send(ReplicationMessage::VoteResponse {
                    node_id: candidate_id,
                    voter_id: self.node_id,
                    term,
                    granted: grant,
                });
            }

            // ── VoteResponse: 내가 Candidate일 때 투표 집계 ──────────────
            ReplicationMessage::VoteResponse {
                node_id,
                voter_id: _,
                term,
                granted,
            } => {
                if node_id != self.node_id {
                    return Ok(());
                }
                let my_term = self.current_term.load(Ordering::SeqCst);
                // 오래된 term의 응답은 무시
                if term != my_term {
                    return Ok(());
                }
                if granted && self.role().await == NodeRole::Candidate {
                    let mut votes = self.votes_received.lock().await;
                    *votes += 1;
                    if *votes >= self.quorum() {
                        // 과반 획득 → Master 승격
                        let mut role = self.role.write().await;
                        *role = NodeRole::Master;
                        drop(role);
                        let _ = self.tx.send(ReplicationMessage::Promotion {
                            node_id: self.node_id,
                            term: my_term,
                        });
                    }
                }
            }

            // ── Promotion: 다른 노드가 Master로 선출됨 ───────────────────
            ReplicationMessage::Promotion { node_id, term } => {
                if node_id == self.node_id {
                    return Ok(());
                }
                let mut role = self.role.write().await;
                // 더 높거나 같은 term의 Promotion → Slave 강등
                if term >= self.current_term.load(Ordering::SeqCst) {
                    self.current_term.store(term, Ordering::SeqCst);
                    *role = NodeRole::Slave;
                    *self.last_heartbeat.write().await = Instant::now();
                }
            }

            _ => {}
        }
        Ok(())
    }

    /// (Slave/Candidate용) Heartbeat 타임아웃 시 선거 시작
    ///
    /// 1. 자신을 Candidate로 승격
    /// 2. term 증가
    /// 3. 자신에게 투표 (자가 투표)
    /// 4. VoteRequest 브로드캐스트
    pub async fn start_election(&self) -> bool {
        if self.role().await == NodeRole::Master {
            return false;
        }
        let elapsed = self.last_heartbeat.read().await.elapsed();
        if elapsed < Duration::from_millis(200) {
            return false; // 타임아웃 아님
        }

        // Candidate로 전환
        *self.role.write().await = NodeRole::Candidate;

        // term 증가 + voted_for 초기화
        let new_term = self.current_term.fetch_add(1, Ordering::SeqCst) + 1;
        *self.voted_for.lock().await = Some(self.node_id); // 자가 투표
        *self.votes_received.lock().await = 1; // 자가 투표 1표

        let last_lsn = self.last_lsn.load(Ordering::SeqCst);
        let _ = self.tx.send(ReplicationMessage::VoteRequest {
            node_id: self.node_id,
            term: new_term,
            last_lsn,
        });

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_node_wins_election() {
        // 클러스터 = 1 노드 → quorum = 1 → 자가 투표만으로 Master 승격
        let (tx, mut rx) = broadcast::channel(32);
        let node = ReplicationNode::new(1, 1, NodeRole::Slave, tx.clone());

        // Heartbeat 오래 전으로 조작 (직접 타임아웃 대기)
        tokio::time::sleep(Duration::from_millis(250)).await;
        let started = node.start_election().await;
        assert!(started, "선거 시작되어야 함");

        // 자신의 VoteRequest 메시지 수신 후 VoteResponse(자가) 처리
        let msg = rx.recv().await.unwrap();
        match msg {
            ReplicationMessage::VoteRequest {
                node_id,
                term,
                last_lsn,
            } => {
                assert_eq!(node_id, 1);
                // VoteResponse 수신 처리 (자기 자신의 투표 응답 시뮬레이션)
                let _ = node
                    .handle_message(
                        ReplicationMessage::VoteResponse {
                            node_id: 1,
                            voter_id: 1,
                            term,
                            granted: true,
                        },
                        &mut |_, _, _| Ok(()),
                    )
                    .await;
                drop(last_lsn);
            }
            _ => panic!("VoteRequest 예상"),
        }

        assert_eq!(node.role().await, NodeRole::Master);
    }

    #[tokio::test]
    async fn test_quorum_requires_majority() {
        // 클러스터 = 3 노드 → quorum = 2
        let (tx, _rx) = broadcast::channel(32);
        let node = ReplicationNode::new(1, 3, NodeRole::Candidate, tx.clone());
        node.current_term.store(1, Ordering::SeqCst);
        *node.votes_received.lock().await = 1; // 자가 투표

        // 1표만으로는 Master가 되면 안 됨
        node.handle_message(
            ReplicationMessage::VoteResponse {
                node_id: 1,
                voter_id: 2,
                term: 1,
                granted: false,
            },
            &mut |_, _, _| Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(
            node.role().await,
            NodeRole::Candidate,
            "아직 Candidate여야 함"
        );

        // 2번째 투표 (granted=true) → 과반 → Master
        node.handle_message(
            ReplicationMessage::VoteResponse {
                node_id: 1,
                voter_id: 3,
                term: 1,
                granted: true,
            },
            &mut |_, _, _| Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(
            node.role().await,
            NodeRole::Master,
            "과반 획득 후 Master여야 함"
        );
    }

    #[tokio::test]
    async fn test_higher_term_promotion_demotes_master() {
        // 기존 Master가 더 높은 term의 Promotion을 수신하면 Slave로 강등
        let (tx, _rx) = broadcast::channel(32);
        let node = ReplicationNode::new(1, 3, NodeRole::Master, tx.clone());
        node.current_term.store(1, Ordering::SeqCst);

        node.handle_message(
            ReplicationMessage::Promotion {
                node_id: 2,
                term: 2,
            },
            &mut |_, _, _| Ok(()),
        )
        .await
        .unwrap();

        assert_eq!(
            node.role().await,
            NodeRole::Slave,
            "더 높은 term의 Promotion → Slave 강등"
        );
        assert_eq!(node.term(), 2);
    }

    #[tokio::test]
    async fn test_replicate_only_as_master() {
        let (tx, _rx) = broadcast::channel(16);
        let node = ReplicationNode::new(1, 1, NodeRole::Slave, tx.clone());
        let result = node.replicate(b"data".to_vec()).await;
        assert!(result.is_err(), "Slave는 복제 불가");
    }
}
