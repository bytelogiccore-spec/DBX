---
layout: default
title: Cross-Node Sharding
parent: English
nav_order: 24
---

# Cross-Node Sharding

DBX distributes data across multiple nodes using **Consistent Hashing**, providing horizontal scalability with minimal data movement.

---

## Architecture Overview

```
                  Hash Ring
              node:0        node:1
           vnode×50      vnode×50
         ┌────────────────────────┐
    key  │  → fnv1a_hash → ring lookup → assigned node
         └────────────────────────┘
              node:2
           vnode×50
```

---

## Weight-Based vnode Distribution

Assign more data to higher-capacity servers by adjusting node weights.

```rust
use dbx_core::sharding::{ShardNode, ShardRouter};

// High-performance server gets 2× more data
let nodes = vec![
    ShardNode { id: 0, address: "server-a:7878".into(), weight: 2.0 }, // 2× vnodes
    ShardNode { id: 1, address: "server-b:7878".into(), weight: 1.0 }, // baseline
    ShardNode { id: 2, address: "server-c:7878".into(), weight: 0.5 }, // ½ vnodes
];

let router = ShardRouter::new(nodes, 100); // 100 vnodes_per_node baseline
```

### Weight Calculation

```
actual vnode count = vnodes_per_node × weight
```

| weight | vnode count (base 100) | data share |
|--------|------------------------|------------|
| 2.0    | 200                    | ~50%       |
| 1.0    | 100                    | ~25%       |
| 0.5    | 50                     | ~12.5%     |

---

## Data Rebalancing

When adding or removing nodes, only the affected key ranges are migrated — no full data copy.

```rust
use dbx_core::sharding::rebalancer::Rebalancer;
use dbx_core::sharding::node_ring::NodeRing;

let old_ring = NodeRing::new(50); // before adding node
let new_ring = NodeRing::new(50); // after adding node

let rebalancer = Rebalancer::new(&old_ring, &new_ring);

// Compute which keys need migration
let tasks = rebalancer.compute_tasks(&all_keys);

// Execute migration
rebalancer.execute(
    &tasks,
    |node_id, key| db.get(node_id, key),                // read
    |node_id, key, value| db.put(node_id, key, value),  // write
    |node_id, key| db.delete(node_id, key),             // delete
);
```

---

## 2PC Distributed Transactions

Guarantees atomicity for writes that span multiple shards.

```rust
use dbx_core::sharding::two_phase::{TwoPhaseCoordinator, PrepareResult};

let mut coord = TwoPhaseCoordinator::new();
let txn = coord.begin();
let nodes = vec![0, 1, 2]; // participating nodes

// Phase 1: Prepare
coord.prepare(txn, &nodes, |node_id, txn_id| {
    if can_commit(node_id, txn_id) {
        PrepareResult::Ready
    } else {
        PrepareResult::Abort("insufficient resources".to_string())
    }
});

// Phase 2: Commit or Abort
let outcome = coord.commit_or_abort(
    txn,
    &nodes,
    |node_id, txn_id| commit(node_id, txn_id),    // all Ready
    |node_id, txn_id| rollback(node_id, txn_id),  // any Abort
);
```

### 2PC Outcomes

| Result | Condition |
|--------|-----------|
| `Committed` | All participants responded `Ready` |
| `Aborted` | At least one participant responded `Abort` |

---

## Related Modules

| File | Role |
|------|------|
| `sharding/node_ring.rs` | Consistent hashing ring, vnode management |
| `sharding/router.rs` | ShardNode, ShardRouter |
| `sharding/rebalancer.rs` | Migration task computation and execution |
| `sharding/two_phase.rs` | 2PC Coordinator |
