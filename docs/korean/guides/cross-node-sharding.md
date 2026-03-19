---
layout: default
title: 크로스-노드 샤딩
nav_order: 24
parent: 가이드
grand_parent: 한국어
---

# 크로스-노드 샤딩

DBX의 크로스-노드 샤딩은 **Consistent Hashing** 기반으로 데이터를 여러 노드에 분산 저장합니다.

---

## 아키텍처 개요

```
                  해시 링 (Hash Ring)
              node:0        node:1
           vnode×50      vnode×50
         ┌────────────────────────┐
    key  │  → fnv1a_hash → 링 탐색 → 담당 노드 결정
         └────────────────────────┘
              node:2
           vnode×50
```

---

## 노드 가중치 기반 vnode 분배

성능이 다른 서버에 데이터를 비균등하게 분배할 수 있습니다.

```rust
use dbx_core::sharding::{ShardNode, ShardRouter};

// 고성능 서버에 2배 더 많은 데이터 할당
let nodes = vec![
    ShardNode { id: 0, address: "서버A:7878".into(), weight: 2.0 }, // vnode × 2배
    ShardNode { id: 1, address: "서버B:7878".into(), weight: 1.0 }, // 기본
    ShardNode { id: 2, address: "서버C:7878".into(), weight: 0.5 }, // vnode × 절반
];

let router = ShardRouter::new(nodes, 100); // 100 vnodes_per_node 기준
```

### weight 계산

```
실제 vnode 수 = vnodes_per_node × weight
```

| weight | vnode 수 (기본 100 기준) | 데이터 할당 비율 |
|--------|--------------------------|-----------------|
| 2.0    | 200                      | ~50%            |
| 1.0    | 100                      | ~25%            |
| 0.5    | 50                       | ~12.5%          |

---

## 데이터 리밸런싱

노드 추가/제거 시 영향받는 키만 선택적으로 이관합니다 (전체 데이터 복사 없음).

```rust
use dbx_core::sharding::rebalancer::Rebalancer;
use dbx_core::sharding::node_ring::NodeRing;

let old_ring = NodeRing::new(50); // 노드 추가 전
let new_ring = NodeRing::new(50); // 노드 추가 후

let rebalancer = Rebalancer::new(&old_ring, &new_ring);

// 이관 필요한 키 목록 계산
let tasks = rebalancer.compute_tasks(&all_keys);

// 실제 이관 실행
rebalancer.execute(
    &tasks,
    |node_id, key| db.get(node_id, key),           // 읽기
    |node_id, key, value| db.put(node_id, key, value), // 쓰기
    |node_id, key| db.delete(node_id, key),         // 삭제
);
```

---

## 2PC 분산 트랜잭션

여러 샤드에 걸친 쓰기 작업의 원자성을 보장합니다.

```rust
use dbx_core::sharding::two_phase::{TwoPhaseCoordinator, PrepareResult};

let mut coord = TwoPhaseCoordinator::new();
let txn = coord.begin();
let nodes = vec![0, 1, 2]; // 참여할 노드 목록

// Phase 1: Prepare
coord.prepare(txn, &nodes, |node_id, txn_id| {
    // 각 노드에서 커밋 가능 여부 확인
    if can_commit(node_id, txn_id) {
        PrepareResult::Ready
    } else {
        PrepareResult::Abort("리소스 부족".to_string())
    }
});

// Phase 2: Commit 또는 Abort
let outcome = coord.commit_or_abort(
    txn,
    &nodes,
    |node_id, txn_id| commit(node_id, txn_id),   // 모든 노드 Ready 시
    |node_id, txn_id| rollback(node_id, txn_id),  // 하나라도 Abort 시
);
```

### 2PC 결과

| 결과 | 조건 |
|------|------|
| `Committed` | 모든 Participant가 `Ready` 응답 |
| `Aborted` | 하나 이상의 Participant가 `Abort` 응답 |

---

## 관련 모듈

| 파일 | 역할 |
|------|------|
| `sharding/node_ring.rs` | Consistent Hashing 링, vnode 관리 |
| `sharding/router.rs` | ShardNode, ShardRouter |
| `sharding/rebalancer.rs` | 데이터 이관 태스크 계산/실행 |
| `sharding/two_phase.rs` | 2PC Coordinator |
