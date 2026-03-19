---
layout: default
title: Multi-Master Failover
parent: English
nav_order: 23
---

# Multi-Master Failover

DBX automatically elects a new master node when the current master fails, ensuring high availability through quorum-based leader election.

---

## Architecture Overview

```
┌───────────────────────────────────────┐
│           DBX Cluster                 │
│  ┌─────────┐  ┌─────────┐  ┌───────┐ │
│  │ Master  │  │ Slave 1 │  │Slave 2│ │
│  │ (term=3)│  │(Follower│  │       │ │
│  └────┬────┘  └────┬────┘  └───┬───┘ │
│       │  Heartbeat  │          │     │
│       └─────────────┴──────────┘     │
└───────────────────────────────────────┘
```

When the master stops sending heartbeats, the remaining nodes initiate a **quorum election**.

---

## Quorum-Based Leader Election

### Key Concepts

| Concept | Description |
|---------|-------------|
| `term` | Election epoch number, incremented on each election |
| `voted_for` | The candidate voted for in the current term (prevents duplicate votes) |
| `Quorum` | Only candidates with majority votes (⌈N/2⌉ + 1) are promoted to master |

### Election Flow

```
1. Heartbeat timeout detected
        ↓
2. Transition to Candidate + increment term
        ↓
3. Broadcast VoteRequest to all nodes
        ↓
4. Receive majority VoteResponse
        ↓
5. Promote to Master + propagate Promotion message
```

---

## Split-Brain Prevention

**Split-Brain** (two nodes simultaneously claiming to be master) is automatically resolved via `term` numbers.

- A node receiving a `Promotion` message with a higher `term` immediately **demotes to Slave**
- After network partition recovery, the cluster converges to a single master automatically

---

## Configuration

```rust
use dbx_core::replication::transport::ReplicationConfig;
use dbx_core::engine::parallel_engine::DbConfig;

let config = DbConfig {
    replication: ReplicationConfig::in_memory(3), // 3-node cluster
    ..Default::default()
};
```

---

## Vector Clock Conflict Resolution

Instead of simple LWW (Last Write Wins), DBX uses **vector clocks** to precisely detect concurrent events.

```rust
use dbx_core::replication::VectorClock;

let mut vc_a = VectorClock::new();
vc_a.tick(1); // event at node 1

let mut vc_b = VectorClock::new();
vc_b.merge_and_tick(&vc_a, 2); // node 2 processes message from node 1

// a → b: a happened before b
assert!(vc_a.happens_before(&vc_b));
```

### Comparison Results

| Result | Meaning |
|--------|---------|
| `HappensBefore` | A occurred before B → B is latest |
| `HappensAfter` | B occurred before A → A is latest |
| `Concurrent` | Simultaneous → application must resolve conflict |
| `Equal` | Identical clock state |

---

## Related Modules

| File | Role |
|------|------|
| `replication/node.rs` | Quorum election, heartbeat, term management |
| `replication/protocol.rs` | VoteRequest, VoteResponse, Promotion messages |
| `replication/vector_clock.rs` | Vector clock implementation |
