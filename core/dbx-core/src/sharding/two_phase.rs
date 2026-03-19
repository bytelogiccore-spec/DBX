//! 2단계 커밋(Two-Phase Commit, 2PC) — 크로스-노드 분산 트랜잭션
//!
//! ## 프로토콜 흐름
//! 1. **PREPARE**: Coordinator가 모든 Participant에게 prepare 요청
//! 2. **COMMIT / ABORT**: 모든 Participant가 준비 완료 시 COMMIT, 하나라도 실패 시 ABORT
//!
//! ## MVP 범위
//! - 인메모리 시뮬레이션 (실제 네트워크 없음)
//! - 비동기 콜백 패턴 사용
//! - timeout은 MVP에서 구현하지 않음 (P3에서 TCP 통신과 함께 추가)

use std::collections::HashMap;

/// 2PC 트랜잭션 ID
pub type TxnId = u64;

/// Participant(참여 노드)의 준비 결과
#[derive(Debug, Clone, PartialEq)]
pub enum PrepareResult {
    Ready,
    Abort(String),
}

/// 2PC Coordinator
///
/// - `participants`: 참여 노드 집합 (node_id → 준비 콜백)
pub struct TwoPhaseCoordinator {
    /// 다음에 발급할 트랜잭션 ID
    next_txn_id: TxnId,
    /// 진행 중인 트랜잭션 (txn_id → 각 노드의 준비 결과)
    pending: HashMap<TxnId, HashMap<usize, PrepareResult>>,
}

/// 2PC 트랜잭션 결과
#[derive(Debug, Clone, PartialEq)]
pub enum CommitOutcome {
    Committed,
    Aborted { reason: String },
}

impl TwoPhaseCoordinator {
    pub fn new() -> Self {
        Self {
            next_txn_id: 1,
            pending: HashMap::new(),
        }
    }

    /// 새 트랜잭션 ID 발급
    pub fn begin(&mut self) -> TxnId {
        let id = self.next_txn_id;
        self.next_txn_id += 1;
        self.pending.insert(id, HashMap::new());
        id
    }

    /// Phase 1: 모든 Participant에게 prepare 요청
    ///
    /// `prepare_fn(node_id, txn_id)` → `PrepareResult`
    pub fn prepare<F>(&mut self, txn_id: TxnId, node_ids: &[usize], mut prepare_fn: F)
    where
        F: FnMut(usize, TxnId) -> PrepareResult,
    {
        if let Some(results) = self.pending.get_mut(&txn_id) {
            for &node_id in node_ids {
                let result = prepare_fn(node_id, txn_id);
                results.insert(node_id, result);
            }
        }
    }

    /// Phase 2: 모든 노드가 Ready 이면 COMMIT, 하나라도 Abort 이면 전체 ABORT
    ///
    /// `commit_fn(node_id, txn_id)` → 커밋 실행
    /// `abort_fn(node_id, txn_id)`  → 롤백 실행
    pub fn commit_or_abort<C, A>(
        &mut self,
        txn_id: TxnId,
        node_ids: &[usize],
        mut commit_fn: C,
        mut abort_fn: A,
    ) -> CommitOutcome
    where
        C: FnMut(usize, TxnId),
        A: FnMut(usize, TxnId),
    {
        let results = match self.pending.get(&txn_id) {
            Some(r) => r,
            None => return CommitOutcome::Aborted { reason: "Unknown txn_id".to_string() },
        };

        // 모든 노드가 Ready인지 확인
        let abort_reason = results.values().find_map(|r| {
            if let PrepareResult::Abort(reason) = r {
                Some(reason.clone())
            } else {
                None
            }
        });

        let outcome = if let Some(reason) = abort_reason {
            for &node_id in node_ids {
                abort_fn(node_id, txn_id);
            }
            CommitOutcome::Aborted { reason }
        } else {
            for &node_id in node_ids {
                commit_fn(node_id, txn_id);
            }
            CommitOutcome::Committed
        };

        self.pending.remove(&txn_id);
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_all_ready_commits() {
        let mut coord = TwoPhaseCoordinator::new();
        let txn = coord.begin();
        let nodes = vec![0, 1, 2];

        coord.prepare(txn, &nodes, |_, _| PrepareResult::Ready);

        let committed: Arc<Mutex<HashSet<usize>>> = Arc::new(Mutex::new(HashSet::new()));
        let c = Arc::clone(&committed);
        let outcome = coord.commit_or_abort(
            txn,
            &nodes,
            move |node_id, _| { c.lock().unwrap().insert(node_id); },
            |_, _| {},
        );

        assert_eq!(outcome, CommitOutcome::Committed);
        assert_eq!(committed.lock().unwrap().len(), 3, "모든 노드가 커밋되어야 함");
    }

    #[test]
    fn test_one_abort_aborts_all() {
        let mut coord = TwoPhaseCoordinator::new();
        let txn = coord.begin();
        let nodes = vec![0, 1, 2];

        // 노드 1이 Abort
        coord.prepare(txn, &nodes, |node_id, _| {
            if node_id == 1 {
                PrepareResult::Abort("disk full".to_string())
            } else {
                PrepareResult::Ready
            }
        });

        let aborted: Arc<Mutex<HashSet<usize>>> = Arc::new(Mutex::new(HashSet::new()));
        let ab = Arc::clone(&aborted);
        let outcome = coord.commit_or_abort(
            txn,
            &nodes,
            |_, _| panic!("커밋되어서는 안 됨"),
            move |node_id, _| { ab.lock().unwrap().insert(node_id); },
        );

        match outcome {
            CommitOutcome::Aborted { reason } => assert_eq!(reason, "disk full"),
            _ => panic!("ABORT 결과 예상"),
        }
        assert_eq!(aborted.lock().unwrap().len(), 3, "모든 노드가 롤백되어야 함");
    }
}
