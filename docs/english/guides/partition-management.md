---
layout: default
title: Partition Management
parent: English
nav_order: 27
---

# Partition Management
{: .no_toc }

Efficiently manage large tables with per-partition statistics, differential compression, automatic archiving, and Hot/Cold data tiering.
{: .fs-6 .fw-300 }

---

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX provides four synergy features alongside table partitioning:

| Feature | Description | Automation |
|---------|-------------|------------|
| **PartitionStats** | Track row_count, min/max/null/distinct per partition | Auto-updated on INSERT |
| **Differential Compression** | Apply different compression levels by partition age | Manual or auto via Lifecycle |
| **PartitionLifecycle** | Auto-archive and delete partitions after set periods | ✅ Fully automatic background scheduler |
| **PartitionTierHint** | Classify partitions as Hot/Warm/Cold storage tier | Manual or auto via Lifecycle |

---

## Prerequisite: Create a Partition

All partition management features require a prior `create_partition()` call.

```rust
use dbx_core::{Database, storage::partition::{PartitionMap, PartitionType}};

let db = Database::open("./db")?;
db.execute_sql("CREATE TABLE orders (id INT, amount FLOAT, created_at INT)")?;

db.create_partition(PartitionMap {
    table: "orders".into(),
    partition_type: PartitionType::Hash {
        column: "id".into(),
        num_partitions: 4,
    },
    num_partitions: 4,
})?;
```

---

## 1. PartitionStats — Per-Partition Statistics

Per-partition statistics used by the query optimizer for execution plan optimization.

### Auto-Update (on INSERT)

`row_count` is automatically incremented with every INSERT — no manual call needed.

```rust
// INSERT into a partitioned table → row_count automatically +1
for i in 0..1000 {
    db.insert("orders", format!("{}", i).as_bytes(), b"data")?;
}

// Check per-partition row_count
let all_stats = db.all_partition_stats("orders")?;
for (partition, stats) in &all_stats {
    println!("{}: {} rows", partition, stats.row_count);
}
// orders__p_part_0: 245 rows
// orders__p_part_1: 258 rows
// orders__p_part_2: 251 rows
// orders__p_part_3: 246 rows
```

### Manual Update (precision hints)

Provide min/max/null/distinct values directly to improve optimizer accuracy.

```rust
use dbx_core::storage::partition::PartitionStats;

db.update_partition_stats("orders", "orders__p_part_0", PartitionStats {
    row_count: 245,
    min_value: 0,
    max_value: 999,
    null_count: 0,
    distinct_count: 245,
})?;

let stats = db.get_partition_stats("orders", "orders__p_part_0")?;
println!("Row count: {}", stats.row_count);
```

---

## 2. Per-Partition Differential Compression

Apply different compression levels based on partition age and access patterns.

```rust
use dbx_core::storage::compression::CompressionConfig;

// Recent partition (Hot) — low compression, fast read/write
db.set_partition_compression("orders", "orders__p_part_3",
    CompressionConfig::zstd_level(3))?;

// Old partition (Cold) — high compression, saves disk space
db.set_partition_compression("orders", "orders__p_part_0",
    CompressionConfig::zstd_level(9))?;

// Query setting (returns default Snappy if not configured)
let config = db.get_partition_compression("orders", "orders__p_part_0")?;
```

### Compression Level Guide

| Level | Use Case | Ratio | Speed |
|-------|----------|-------|-------|
| 1 ~ 3 | Real-time data (Hot) | Low | Very fast |
| 4 ~ 6 | General data (Warm) | Medium | Fast |
| 7 ~ 9 | Archives (Cold) | High | Moderate |

---

## 3. PartitionLifecycle — Fully Automatic Archiving

Call `enable_auto_archive()` **once** — everything else is automatic.

```rust
use dbx_core::storage::partition::PartitionLifecycle;

db.enable_auto_archive("orders", PartitionLifecycle {
    archive_after_days: 90,   // after 90 days → ZSTD level 9 + Cold tier auto-applied
    delete_after_days: 365,   // after 365 days → metadata auto-deleted
})?;

// Nothing else needed.
// Runs automatically in the background every 1 hour.
```

### How Automation Works

```
enable_auto_archive() called
    │
    └─ [dbx-lifecycle-scheduler] background thread auto-started
            ↓ every 1 hour
        Check age of all partitions
        ├─ archive_after_days elapsed → ZSTD 9 compression + Cold tier auto-applied
        └─ delete_after_days elapsed → all metadata auto-deleted
```

> **Thread guarantee**: Even if `enable_auto_archive()` is called for multiple tables, only one scheduler thread is created (guaranteed by CAS `compare_exchange`).

### On-demand Immediate Execution

Trigger execution outside the automatic schedule.

```rust
// Single table
let (archived, deleted) = db.run_partition_lifecycle("orders")?;
println!("Archived: {}, Deleted: {}", archived, deleted);

// All tables at once
let (total_a, total_d) = db.run_all_partition_lifecycles()?;
```

### Partition Creation Time

Automatically recorded on first INSERT.

```rust
// Query (returns None if not yet written)
let created_at: Option<u64> = db.get_partition_creation_time("orders__p_part_0");

// Manual condition check
let needs_archive = db.partition_needs_archive("orders", created_at.unwrap_or(0))?;
let needs_delete  = db.partition_needs_delete("orders", created_at.unwrap_or(0))?;
```

---

## 4. PartitionTierHint — Hot/Cold Data Separation

Assign storage tier hints to partitions for query routing and cost optimization.

```rust
use dbx_core::storage::partition::PartitionTierHint;

// Recent partition → Hot (memory/SSD priority)
db.set_partition_tier("orders", "orders__p_part_3", PartitionTierHint::Hot)?;
// Middle partition → Warm (SSD)
db.set_partition_tier("orders", "orders__p_part_2", PartitionTierHint::Warm)?;
// Old partition → Cold (HDD + high compression)
db.set_partition_tier("orders", "orders__p_part_0", PartitionTierHint::Cold)?;

// Query tier (returns default Hot if not configured)
let tier = db.get_partition_tier("orders", "orders__p_part_3")?;

// List partitions by tier
let hot_list  = db.list_partitions_by_tier("orders", PartitionTierHint::Hot)?;
let cold_list = db.list_partitions_by_tier("orders", PartitionTierHint::Cold)?;
```

---

## Recommended Pattern — Full Combination

```rust
use dbx_core::{Database, storage::partition::*};

let db = Database::open("./db")?;
db.execute_sql("CREATE TABLE logs (id INT, msg TEXT)")?;

// 1. Create partitions
db.create_partition(PartitionMap {
    table: "logs".into(),
    partition_type: PartitionType::Hash {
        column: "id".into(),
        num_partitions: 8,
    },
    num_partitions: 8,
})?;

// 2. Enable fully automatic archiving — one line does it all
db.enable_auto_archive("logs", PartitionLifecycle {
    archive_after_days: 90,
    delete_after_days: 365,
})?;

// 3. Insert data — row_count and creation_time auto-recorded
for i in 0..10000 {
    db.insert("logs", format!("{}", i).as_bytes(), b"log data")?;
}

// 4. No further management needed — background handles everything
```

**Expected benefits**:
- 📊 Query speed: 10-50× faster (partition pruning)
- 💾 Disk usage: 50-70% reduction (high-compression archives)
- 🤖 Operational overhead: fully automated

---

## Next Steps

- [Compression Guide](compression) — Global compression settings
- [Storage Layers](storage-layers) — 5-Tier architecture and partitioning relationship
- [Scheduler Guide](scheduler) — Register custom scheduled jobs
