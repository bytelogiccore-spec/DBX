---
layout: default
title: Benchmarks
nav_order: 3
parent: English
description: "DBX performance benchmarks"
---

# Benchmarks
{: .no_toc }

Performance benchmarks comparing DBX against other embedded databases.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Executive Summary

DBX is a high-performance embedded database engine written in pure Rust. **v0.2.0-beta achieved 1st place in all major operations** (INSERT, GET, SCAN).

### Latest Benchmarks (v0.2.0-beta, 10,000 Records)

| Operation | DBX (Fast-Path) | SQLite | Sled | Redb | Rank |
|-----------|-----------------|--------|------|------|------|
| **SCAN (Count)** | **51µs** 🥇 | 340µs | 4.64ms | 2.08ms | **#1** |
| **INSERT** | **25.21ms** 🥇 | 29.4ms | 56.6ms | 54.05ms | **#1** |
| **GET** | **2.84ms** 🥇 | 33.8ms | 5.88ms | 2.96ms | **#1** |

> **SCAN Note**: With the **Fast-Path** technology introduced in v0.2.0, single-node query latency has been reduced to the microsecond (µs) level.

### Performance vs Competitors

**vs SQLite**:
- INSERT: **18% faster**
- GET: **1,217% faster (13x)**
- SCAN: **86% faster**

**vs Redb**:
- INSERT: **20% faster**
- GET: **14% faster**
- SCAN: **34% faster**

**vs Sled**:
- INSERT: **35% faster**
- GET: **107% faster (2x)**
- SCAN: **190% faster (2.9x)**

**Version**: DBX v0.2.0-beta
**Test Date**: April 3, 2026
**Report Type**: Official Performance Analysis

---

## Test Environment

### Hardware Specifications

| Item | Specification |
|------|---------------|
| **Operating System** | Microsoft Windows 11 Pro (Build 26200) |
| **System Type** | x64-based PC |
| **Processor** | 1 Processor (Multiprocessor Free) |
| **Memory** | 16,273 MB (approx. 16GB) |

### Software Environment

| Component | Version |
|-----------|---------|
| **Rust Compiler** | rustc 1.92.0 (ded5c06cf 2025-12-08) |
| **Cargo** | 1.92.0 (344c4567c 2025-10-21) |
| **Build Profile** | `release` (optimizations enabled) |
| **Benchmark Framework** | Criterion.rs v0.5 |

---

## Tested Databases

| Database | Version | Language | Features |
|----------|---------|----------|----------|
| **DBX** | 0.2.0-beta | Pure Rust | 5-Tier Hybrid Storage, MVCC |
| **SQLite** | 0.32 (rusqlite) | C (bundled) | Industry-standard embedded DB |
| **Sled** | 0.34 | Pure Rust | Lock-free B+ tree |
| **Redb** | 2.1 | Pure Rust | LMDB-inspired, file-only |

---

## Benchmark Methodology

### Measurement Framework

- **Tool**: Criterion.rs v0.5 (Rust standard benchmarking library)
- **Sample Count**: 100 iterations per test
- **Warmup**: 3-second warmup before each test
- **Statistical Analysis**: Mean, standard deviation, 95% confidence interval
- **Outlier Detection**: Automatic outlier removal and reporting

### Fair Comparison Conditions

#### DBX Configuration

```rust
// Default features enabled
features = ["wal", "mvcc", "index"]

// Durability disabled for fair comparison
durability = DurabilityLevel::None
```

#### Common Settings for All Databases

1. **Transaction/Batch Mode**
   - Fair comparison using batch commits instead of individual INSERTs
   - DBX: `begin()` → `insert()` × N → `commit()`
   - SQLite: `unchecked_transaction()` → `execute()` × N → `commit()`
   - Sled: `insert()` × N → `flush()`
   - Redb: `begin_write()` → `insert()` × N → `commit()`

2. **WAL (Write-Ahead Logging) Disabled**
   - DBX: `durability = DurabilityLevel::None`
   - SQLite: `PRAGMA synchronous = OFF`
   - Sled: Default settings (flush-based)
   - Redb: Default settings (transaction-based)

3. **Identical Data Size**
   - Key: String format `"key_{i}"`
   - Value: String format `"value_data_{i}"`
   - Test size: 10,000 records

---

## Detailed Benchmark Results

### INSERT Performance (10,000 records)

| Database | Average Time | Std Dev | Throughput (rec/sec) | vs DBX |
|----------|--------------|---------|----------------------|--------|
| **DBX** | **25.21ms** | ±0.20ms | **396,668** | **1.0× (baseline)** |
| SQLite | 29.4ms | ±0.38ms | 340,136 | **0.85× (14% slower)** |
| Redb | 54.05ms | ±0.72ms | 185,015 | **0.46× (114% slower)** |
| Sled | 56.6ms | ±1.55ms | 176,678 | **0.44× (124% slower)** |

**DBX Advantages**:
- ✅ **Faster than all competitors**
- ✅ 14% faster than SQLite
- ✅ Stable performance (low std dev)

### GET Performance (10,000 records)

| Database | Average Time | Std Dev | Throughput (rec/sec) | vs DBX |
|----------|--------------|---------|----------------------|--------|
| **DBX** | **2.84ms** | ±0.01ms | **3,521,127** | **1.0× (baseline)** |
| Redb | 2.96ms | ±0.17ms | 3,378,378 | **0.95× (4% slower)** |
| Sled | 5.88ms | ±0.03ms | 1,700,680 | **0.48× (107% slower)** |
| SQLite | 33.8ms | ±0.48ms | 295,857 | **0.08× (1,090% slower)** |

**DBX Advantages**:
- ✅ **11x faster than SQLite**
- ✅ 4% faster than Redb
- ✅ 2x faster than Sled

### SCAN Performance (10,000 records)

| Database | Average Time | Std Dev | Throughput (rec/sec) | vs DBX |
|----------|--------------|---------|----------------------|--------|
| **DBX** | **51µs** | ±0.01µs | **19,607,843** | **1.0× (baseline)** |
| Redb | 2.08ms | ±0.02ms | 480,769 | **0.02× (3,978% slower)** |
| SQLite | 340µs | ±0.16ms | 2,941,176 | **0.15× (566% slower)** |
| Sled | 4.64ms | ±0.10ms | 215,517 | **0.01× (8,998% slower)** |

**DBX Advantages**:
- ✅ **Faster than all competitors**
- ✅ 40x faster than Redb
- ✅ 6x faster than SQLite

---

## Performance Optimization Techniques

### Phase 1: GET Optimization (+70% improvement)

1. **Inline Attributes**
   - Added `#[inline(always)]` to eliminate function call overhead
   - Improved compiler optimization

2. **MVCC Overhead Removal**
   - Removed MVCC checks from hot path
   - Eliminated unnecessary timestamp acquisition cost

3. **Code Path Simplification**
   - Removed conditionals for better branch prediction
   - Improved CPU pipeline efficiency

**Result**: scan -29.2%, get -6.8% further improvement

### Phase 9: Fast-Path Technology (v0.2.0)

1. **Local Bypass**: Bypasses distributed DAG scheduling overhead, routing directly to the LocalExecutor.
2. **Sync Data Stream**: Returns memory data synchronously via `sync_batches`, eliminating mpsc channel overhead.
3. **Lazy Setup**: Removed redundant I/O (directory scans, etc.) during query initialization.

**Result**: Scan latency reduced from 340µs to **51µs** (6.6x improvement).

### Phase 2: SCAN Optimization (+57% improvement)

1. **Delta Store Fast-path**
   - Early return when Delta is empty
   - Eliminated unnecessary scan operations

2. **2-way Merge Optimization**
   - Direct WOS scan when Delta is empty
   - Completely removed merge overhead

3. **Cache Locality Improvement**
   - Optimized memory access pattern with single scan
   - Improved CPU cache efficiency

**Result**: 3.70ms → 1.60ms (2.3x faster)

---

## Architecture Strengths

### 5-Tier Hybrid Storage

1. **Delta Store (Tier 1)**
   - DashMap + SkipMap (Lock-free)
   - Ultra-fast INSERT performance

2. **WOS (Tier 2)**
   - BTreeMap (sorted storage)
   - Efficient range scans

3. **Optimized Data Flow**
   - Automatic Delta → WOS flush
   - Efficient utilization of memory and disk

---

## Reproducing Benchmarks

```bash
# Clone project
git clone https://github.com/ByteLogicCore/DBX.git
cd DBX

# Run full comparison benchmark
cargo bench -p dbx-benchmarks --bench official_db_comparison

# Run individual database benchmarks
cargo bench -p dbx-benchmarks --bench official_db_comparison -- dbx_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- sqlite_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- sled_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- redb_
```

---

## Conclusion

DBX v0.0.6-beta **achieved 1st place in all major operations** (INSERT, GET, SCAN).

### Key Achievements

- ✅ **INSERT 1st**: 44.92ms (18-35% faster than competitors)
- ✅ **GET 1st**: 2.84ms (13x faster than SQLite)
- ✅ **SCAN 1st**: 1.60ms (34-190% faster than competitors)

### Technical Differentiators

1. **5-Tier Hybrid Storage**: Efficient utilization of memory and disk
2. **Lock-Free Architecture**: DashMap + SkipMap
3. **Pure Rust**: Memory safety and zero-cost abstractions
4. **Optimized Algorithms**: Fast-path and inline optimizations

DBX is the **optimal choice for applications requiring high-performance write workloads and balanced read/write performance**.

---

## Next Steps

- [Architecture](architecture) — Understand the 5-Tier Hybrid Storage
- [Getting Started](getting-started) — Try DBX yourself
- [GPU Acceleration](guides/gpu-acceleration) — Accelerate analytical queries
- [Examples](examples/quick-start) — Explore code examples
