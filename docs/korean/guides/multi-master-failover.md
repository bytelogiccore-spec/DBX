---
layout: default
title: Multi-Master Failover
nav_order: 23
parent: 가이드
grand_parent: 한국어
---

# Multi-Master Failover

DBX의 Multi-Master Failover는 클러스터 내 마스터 노드 장애 시 자동으로 새로운 마스터를 선출하여 고가용성을 보장합니다.

---

## 아키텍처 개요

```
┌───────────────────────────────────────┐
│           DBX 클러스터                 │
│  ┌─────────┐  ┌─────────┐  ┌───────┐ │
│  │ Master  │  │ Slave 1 │  │Slave 2│ │
│  │ (term=3)│  │(Follower│  │       │ │
│  └────┬────┘  └────┬────┘  └───┬───┘ │
│       │  Heartbeat  │          │     │
│       └─────────────┴──────────┘     │
└───────────────────────────────────────┘
```

마스터가 일정 시간 동안 Heartbeat를 전송하지 않으면 나머지 노드 중 하나가 **Quorum 선거**를 시작합니다.

---

## Quorum 기반 리더 선출

### 핵심 개념

| 개념 | 설명 |
|------|------|
| `term` | 선거 임기 번호. 선거가 발생할 때마다 증가 |
| `voted_for` | 현재 term에서 투표한 후보 (중복 투표 방지) |
| `Quorum` | 과반 투표(⌈N/2⌉ + 1)를 얻은 후보만 마스터로 승격 |

### 선거 흐름

```
1. Heartbeat 타임아웃 감지
        ↓
2. Candidate 상태 전환 + term 증가
        ↓
3. 전체 노드에 VoteRequest 브로드캐스트
        ↓
4. 과반수 VoteResponse 수신
        ↓
5. Master 승격 + Promotion 메시지 전파
```

---

## Split-Brain 방지

두 노드가 동시에 마스터를 주장하는 **Split-Brain** 상황은 `term` 번호로 자동 해결됩니다.

- 더 높은 `term`의 `Promotion` 메시지를 받으면 낮은 `term`의 마스터는 즉시 **Slave로 강등**
- 네트워크 파티션 복구 후 자동으로 단일 마스터 상태로 수렴

---

## 설정

```rust
use dbx_core::replication::transport::ReplicationConfig;
use dbx_core::engine::parallel_engine::DbConfig;

let config = DbConfig {
    replication: ReplicationConfig::in_memory(3), // 3노드 클러스터
    ..Default::default()
};
```

---

## 벡터 클록 기반 충돌 해결

단순 LWW(Last Write Wins) 대신 **벡터 클록**으로 동시 발생 이벤트를 정확히 감지합니다.

```rust
use dbx_core::replication::VectorClock;

let mut vc_a = VectorClock::new();
vc_a.tick(1); // 노드 1 이벤트 발생

let mut vc_b = VectorClock::new();
vc_b.merge_and_tick(&vc_a, 2); // 노드 2가 노드 1 메시지 수신 후 처리

// a → b: a가 먼저 발생
assert!(vc_a.happens_before(&vc_b));
```

### 비교 결과

| 결과 | 의미 |
|------|------|
| `HappensBefore` | A가 B보다 먼저 발생 → B가 최신 |
| `HappensAfter` | B가 A보다 먼저 발생 → A가 최신 |
| `Concurrent` | 동시 발생 → 애플리케이션에서 충돌 처리 필요 |
| `Equal` | 동일한 클록 상태 |

---

## 관련 모듈

| 파일 | 역할 |
|------|------|
| `replication/node.rs` | Quorum 선거, Heartbeat, term 관리 |
| `replication/protocol.rs` | VoteRequest, VoteResponse, Promotion 메시지 |
| `replication/vector_clock.rs` | 벡터 클록 구현 |
