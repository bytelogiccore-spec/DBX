---
layout: default
title: Architecture
nav_order: 2
parent: English
description: "DBX 5-Tier Hybrid Storage architecture"
---

# Architecture
{: .no_toc }

Deep dive into DBX's 5-Tier Hybrid Storage architecture and MVCC transaction system.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 5-Tier Hybrid Storage

DBX uses a sophisticated 5-tier architecture optimized for both OLTP and OLAP workloads:

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← In-memory write buffer
│     - Lock-free concurrency             │
│     - Hot data caching                  │
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← OLAP optimization
│     - RecordBatch caching               │
│     - Projection Pushdown               │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (sled)                     │  ← Persistent storage
│     - Write-Optimized Store             │
│     - MVCC with Snapshot Isolation      │
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← Fast existence check
│     - Minimize false positives          │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Parquet)                  │  ← Columnar compression
│     - Read-Optimized Store              │
│     - Apache Arrow/Parquet              │
└─────────────────────────────────────────┘

                  Optional: GPU Acceleration
┌─────────────────────────────────────────┐
│  GPU Manager (CUDA)                     │  ← Analytical query acceleration
│     - GROUP BY, Hash Join               │
│     - Filtering, Aggregation            │
└─────────────────────────────────────────┘
```

### Tier 1: Delta Store

**Purpose**: In-memory write buffer for hot data

**Implementation**: `BTreeMap<Vec<u8>, Vec<u8>>`

**Features**:
- Lock-free concurrent reads
- Fast writes (O(log n))
- Automatic flush on threshold
- Shadows lower tiers

### Tier 2: Columnar Cache

**Purpose**: OLAP query optimization

**Implementation**: Apache Arrow `RecordBatch`

**Features**:
- Columnar storage format
- Projection pushdown
- Predicate pushdown
- Zero-copy operations
- Vectorized execution (SIMD)

### Tier 3: WOS (Write-Optimized Store)

**Purpose**: Persistent transactional storage

**Implementation**: `sled` embedded database

**Features**:
- MVCC with Snapshot Isolation
- ACID transactions
- Crash recovery
- Compaction


### Tier 4: Index

**Purpose**: Fast existence checks

**Implementation**: Bloom Filter

**Features**:
- Minimize false positives
- Fast lookups (O(1))
- Space-efficient

### Tier 5: ROS (Read-Optimized Store)

**Purpose**: Long-term columnar storage

**Implementation**: Apache Parquet

**Features**:
- Columnar compression
- Efficient scans
- Predicate pushdown
- Schema evolution

---

## MVCC Transaction System

DBX implements Multi-Version Concurrency Control (MVCC) with Snapshot Isolation.

### Transaction Flow

```
Transaction Begin
    ↓
Snapshot Isolation (read_ts)
    ↓
Read/Write Operations
    ↓
Commit (commit_ts)
    ↓
Garbage Collection (async)
```

### Versioning

Each record is versioned with timestamps:

```rust
struct VersionedValue {
    value: Vec<u8>,
    version: u64,      // Transaction timestamp
    deleted: bool,     // Tombstone marker
}
```

### Snapshot Isolation

- Each transaction sees a consistent snapshot
- Read timestamp (`read_ts`) assigned at transaction start
- Write timestamp (`commit_ts`) assigned at commit
- Reads see versions where `version <= read_ts`

### Garbage Collection

- Async background process
- Removes old versions no longer visible
- Configurable retention policy

---

## GPU Acceleration

DBX optionally supports CUDA-based GPU acceleration for analytical queries.

### Supported Operations

- **Aggregations**: SUM, COUNT, MIN, MAX, AVG
- **Filtering**: Predicate evaluation
- **GROUP BY**: Hash-based grouping
- **Hash Join**: Equi-joins

### Hash Strategies

DBX supports multiple GPU hash strategies:

| Strategy | Performance | Use Case |
|----------|-------------|----------|
| **Linear** | Stable | Small groups (default) |
| **Cuckoo** | Aggressive | SUM +73%, Filtering +32% |
| **Robin Hood** | Balanced | SUM +7%, Filtering +10% |

### Performance

GPU acceleration shows significant gains on large datasets:

- **1M rows**: 3.06x faster (filtering)
- **10M+ rows**: Up to 4.57x faster

---

## Query Optimization

### Projection Pushdown

Only read required columns from storage:

```sql
SELECT id, name FROM users;  -- Only reads 'id' and 'name' columns
```

### Predicate Pushdown

Filter data at storage layer:

```sql
SELECT * FROM users WHERE age > 30;  -- Filter applied during scan
```

### Vectorized Execution

SIMD operations on Arrow RecordBatch:

- Process multiple rows simultaneously
- CPU cache-friendly
- Zero-copy data access

---

## Data Flow

### Write Path

```
Application
    ↓
Delta Store (Tier 1)
    ↓ (auto-flush on threshold)
WOS (Tier 3)
    ↓ (compaction)
ROS (Tier 5)
```

### Read Path (OLTP)

```
Application
    ↓
Delta Store (Tier 1) → if found, return
    ↓
WOS (Tier 3) → if found, return
    ↓
Index (Tier 4) → check existence
    ↓
ROS (Tier 5) → read from Parquet
```

### Read Path (OLAP)

```
Application (SQL query)
    ↓
Query Optimizer
    ↓
Columnar Cache (Tier 2) → if cached, use
    ↓
Delta Store sync to Cache
    ↓
Vectorized Execution (SIMD)
    ↓
Optional: GPU Acceleration
    ↓
Results
```

---

## Next Steps

- [Benchmarks](benchmarks) — See performance comparisons
- [Examples](examples/quick-start) — Explore code examples
- [API Documentation](https://docs.rs/dbx-core) — Full Rust API reference
