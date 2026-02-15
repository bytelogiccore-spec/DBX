---
layout: default
title: Query Plan Cache
parent: English
nav_order: 33
---

# Query Plan Cache
{: .no_toc }

Skip parsing and optimization for repeated SQL queries to maximize performance.
{: .fs-6 .fw-300 }

---

## Overview

SQL execution goes through **parsing → logical plan → physical plan → execution**. Repeating the same SQL triggers redundant parsing and optimization each time.

The plan cache eliminates this overhead:

```
First run:  SQL → [Parse → Optimize → Build Plan] → Execute    (slow)
Re-run:     SQL → [Cache Hit!]                     → Execute    (fast)
```

---

## Two-Tier Architecture

| Tier | Storage | Speed | Purpose |
|------|---------|-------|---------|
| **L1** | Memory (DashMap) | ~1.6 µs | Frequently used queries |
| **L2** | Disk | ~ms | Reuse evicted plans |

---

## Usage

```rust
use dbx_core::engine::plan::PlanCache;

// Create cache (max 1000 entries)
let cache = PlanCache::new(1000);

// Plans are cached automatically on execution
let plan = cache.get_or_insert("SELECT * FROM users WHERE id = 1", || {
    // Parse + optimize (runs only on cache miss)
    planner.plan(sql)?
})?;
```

---

## Cache Statistics

```rust
let stats = cache.stats();
println!("Hit rate: {:.1}%", stats.hit_rate() * 100.0);
println!("Hits: {} / Misses: {} / Evictions: {}", stats.hits, stats.misses, stats.evictions);
```

---

## Performance

| Scenario | No Cache | Cached | Improvement |
|----------|:--------:|:------:|:-----------:|
| Same SQL 10x | 146 µs | 20 µs | **7.3x** |
| L1 hit | - | 1.6 µs | - |

---

## Next Steps

- [Parallel Query Guide](parallel-query) — Parallel processing for large data
- [SQL Reference](sql-reference) — Supported SQL syntax
