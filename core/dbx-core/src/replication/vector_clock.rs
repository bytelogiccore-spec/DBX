//! 벡터 클록(Vector Clock) — 분산 이벤트 인과관계 추적
//!
//! ## 용도
//! - LWW(Timestamp)는 clock skew 시 데이터 손실 위험이 있음
//! - 벡터 클록은 각 노드의 논리 시계를 독립적으로 관리하여
//!   **동시 발생 이벤트(concurrent)**를 감지할 수 있습니다.
//!
//! ## 비교 결과
//! - `HappensBefore(A → B)`: A가 B 이전에 발생
//! - `HappensAfter(B → A)`: B가 A 이전에 발생
//! - `Concurrent`: 인과관계 없음 → 충돌 → 애플리케이션 레벨 처리 필요

use std::collections::HashMap;

/// 벡터 클록 비교 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorClockOrder {
    /// self 가 other 이전 (self → other)
    HappensBefore,
    /// self 가 other 이후 (other → self)
    HappensAfter,
    /// 인과관계 없음 — 동시 발생 (충돌)
    Concurrent,
    /// 완전히 동일
    Equal,
}

/// 벡터 클록
///
/// `node_id → logical_clock` 의 맵으로 구성됩니다.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VectorClock {
    clocks: HashMap<u32, u64>,
}

impl VectorClock {
    /// 새 빈 벡터 클록 생성
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    /// 특정 노드의 논리 클록 값 조회
    pub fn get(&self, node_id: u32) -> u64 {
        *self.clocks.get(&node_id).unwrap_or(&0)
    }

    /// 특정 노드의 클록을 1 증가 (이벤트 발생 시 호출)
    pub fn tick(&mut self, node_id: u32) {
        let entry = self.clocks.entry(node_id).or_insert(0);
        *entry += 1;
    }

    /// 메시지 수신 시 병합: `self[node] = max(self[node], other[node])` + tick
    pub fn merge_and_tick(&mut self, other: &VectorClock, self_node_id: u32) {
        for (&node_id, &clock) in &other.clocks {
            let entry = self.clocks.entry(node_id).or_insert(0);
            *entry = (*entry).max(clock);
        }
        self.tick(self_node_id);
    }

    /// 단순 병합 (tick 없이)
    pub fn merge(&mut self, other: &VectorClock) {
        for (&node_id, &clock) in &other.clocks {
            let entry = self.clocks.entry(node_id).or_insert(0);
            *entry = (*entry).max(clock);
        }
    }

    /// 두 벡터 클록을 비교하여 인과관계 판단
    pub fn compare(&self, other: &VectorClock) -> VectorClockOrder {
        let mut self_gt = false; // self가 더 큰 항목 존재?
        let mut other_gt = false; // other가 더 큰 항목 존재?

        // 양쪽 노드 집합 합집합
        let all_nodes: std::collections::HashSet<u32> = self
            .clocks
            .keys()
            .chain(other.clocks.keys())
            .copied()
            .collect();

        for node in all_nodes {
            let lhs = self.get(node);
            let rhs = other.get(node);
            if lhs > rhs {
                self_gt = true;
            }
            if rhs > lhs {
                other_gt = true;
            }
        }

        match (self_gt, other_gt) {
            (false, false) => VectorClockOrder::Equal,
            (true, false) => VectorClockOrder::HappensAfter,
            (false, true) => VectorClockOrder::HappensBefore,
            (true, true) => VectorClockOrder::Concurrent,
        }
    }

    /// self가 other보다 먼저 발생했는지 여부
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        self.compare(other) == VectorClockOrder::HappensBefore
    }

    /// 두 클록이 동시 발생(충돌)인지 여부
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        self.compare(other) == VectorClockOrder::Concurrent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_and_get() {
        let mut vc = VectorClock::new();
        vc.tick(1);
        vc.tick(1);
        vc.tick(2);
        assert_eq!(vc.get(1), 2);
        assert_eq!(vc.get(2), 1);
        assert_eq!(vc.get(99), 0);
    }

    #[test]
    fn test_happens_before() {
        let mut a = VectorClock::new();
        a.tick(1);
        // b는 a를 받은 뒤 b 자신의 이벤트 발생
        let mut b = VectorClock::new();
        b.merge_and_tick(&a, 2);
        // a → b
        assert_eq!(a.compare(&b), VectorClockOrder::HappensBefore);
        assert_eq!(b.compare(&a), VectorClockOrder::HappensAfter);
    }

    #[test]
    fn test_concurrent() {
        let mut a = VectorClock::new();
        a.tick(1);
        let mut b = VectorClock::new();
        b.tick(2);
        // 두 이벤트은 서로 독립적 → 동시 발생
        assert_eq!(a.compare(&b), VectorClockOrder::Concurrent);
        assert!(a.is_concurrent(&b));
    }

    #[test]
    fn test_merge() {
        let mut a = VectorClock::new();
        a.tick(1);
        a.tick(1); // [1:2]
        let mut b = VectorClock::new();
        b.tick(2); // [2:1]
        b.merge(&a);
        // b = [1:2, 2:1]
        assert_eq!(b.get(1), 2);
        assert_eq!(b.get(2), 1);
    }

    #[test]
    fn test_equal_clocks() {
        let mut a = VectorClock::new();
        let mut b = VectorClock::new();
        a.tick(1);
        b.tick(1);
        assert_eq!(a.compare(&b), VectorClockOrder::Equal);
    }
}
