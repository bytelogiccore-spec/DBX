---
layout: default
title: DB Configuration
parent: English
nav_order: 35
description: "DbConfig, DirtyBufferMode, ParallelismConfig, DurabilityLevel, EncryptionConfig — complete DBX configuration reference"
---

# DB Configuration Guide
{: .no_toc }

Source-code-level reference for all DBX configuration options. Covers both `Database::open()` and `Database::open_with_config()`.
{: .fs-6 .fw-300 }

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Opening a Database

### Method 1: `Database::open()` — Default Settings

```rust
use dbx_core::Database;
use std::path::Path;

// File-backed — default settings (all cores, 1,000-row threshold, DurabilityLevel::Full)
let db = Database::open(Path::new("./data"))?;

// In-memory — for testing/caching
let db = Database::open_in_memory()?;
```

### Method 2: `Database::open_with_config()` — Custom Settings

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig};
use std::path::Path;

let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig {
            cpu_cap: 0.75,
            min_rows_for_parallel: 2_000,
        },
    },
)?;
```

### Method 2b: `Database::open_encrypted()` — Encrypted DB

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// Password-based (derives 256-bit key via HKDF-SHA256)
let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted(Path::new("./data"), enc)?;

// Specify algorithm
let enc = EncryptionConfig::from_password("pw")
    .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);
let db = Database::open_encrypted(Path::new("./data"), enc)?;
```

---

## `DbConfig` Struct

> **File**: `src/engine/parallel_engine.rs`

```rust
pub struct DbConfig {
    pub parallelism: ParallelismConfig,
    pub sync: RealtimeSyncConfig,
    pub replication: ReplicationConfig,
    pub dirty_buffer_mode: DirtyBufferMode,  // added in v0.1.1-beta
}
```

---

## `DirtyBufferMode` — WOS Dirty Buffer Data Structure

> **File**: `src/engine/parallel_engine.rs`

Selects the data structure for the WOS Tier 3 `dirty` buffer (temporary in-memory store before flush).

| Value | Default | Characteristics | Recommended Workload |
|-------|:-------:|-----------------|----------------------|
| `BTreeMap` | ✅ | Maintains key order → efficient `scan(range)` | Range-heavy OLAP / general OLTP |
| `DashMap` | | Shard-locked → efficient concurrent multi-thread access | Simple get/insert, write-heavy bursts |

### Switching Safety

`dirty` is a pure in-memory buffer, never persisted to disk.  
It always starts empty on restart, so you can **freely switch between modes without data corruption**.

```
Program start
  └─ dirty = empty state (always, regardless of BTreeMap/DashMap)
       ↓
  Replay wal_entries from WAL file
       ↓
  Load SSTable index
```

### Usage Example

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, DirtyBufferMode};

// DashMap mode (high-throughput simple get/insert workloads)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        dirty_buffer_mode: DirtyBufferMode::DashMap,
        ..Default::default()
    },
)?;

// Default (BTreeMap — range query optimized)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig::default(),
)?;
```

### DashMap Trade-offs

| Operation | BTreeMap | DashMap |
|-----------|:--------:|:-------:|
| `get(key)` simple lookup | normal | faster |
| `scan(range)` range query | **fast** | slower (full iteration + sort) |
| Concurrent multi-thread access | sequential | **concurrent** |
| Sort cost during compaction | none | one-time internal sort |

> **Note**: `dirty` is a temporary buffer before flush, so data moves to WAL quickly.  
> SSTable `scan` is unaffected — SSTables are always sorted.

---

## `ParallelismConfig` — Parallelism Control

> **File**: `src/engine/parallel_engine.rs`

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cpu_cap` | `f64` | `1.0` | CPU core usage ratio (0.01 ~ 1.0) |
| `min_rows_for_parallel` | `usize` | `1_000` | Minimum row count to activate parallelism |

### `cpu_cap` Details

```
actual thread count = ceil(base_threads × cpu_cap).max(1)
base_threads        = min(num_cpus, 16)   // Auto policy
```

| `cpu_cap` | Threads on 12-core CPU |
|-----------|------------------------|
| `1.0` (default) | 12 |
| `0.75` | 9 |
| `0.5` | 6 |
| `0.25` | 3 |

### `min_rows_for_parallel` Details

Data below this threshold is processed single-threaded, avoiding thread pool overhead.

```
500   → aggressive, parallelizes small batches
1,000 → default, balanced
5,000 → conservative, parallelizes large batches only
```

### Presets

```rust
// 50% CPU, 5,000-row threshold — minimal load on dev machines
ParallelismConfig::conservative()

// 100% CPU, 500-row threshold — maximum performance
ParallelismConfig::aggressive()

// 100% CPU, 1,000-row threshold — default
ParallelismConfig::default()
```

---

## `ParallelizationPolicy` — Thread Pool Policy

> **File**: `src/engine/parallel_engine.rs`

`Database::open_with_config()` internally uses `ParallelizationPolicy::Auto`.  
For low-level control, create a `ParallelExecutionEngine` directly.

| Policy | Description | Thread Count |
|--------|-------------|--------------|
| `Auto` | Automatic based on system cores (default) | `min(num_cpus, 16)` |
| `Fixed(n)` | Fixed thread count | `n` (error if 0) |
| `Adaptive` | Starts at half, adjusts to workload | `(num_cpus / 2).max(1)` |

```rust
use dbx_core::engine::parallel_engine::{
    ParallelExecutionEngine, ParallelizationPolicy
};

// Exactly 4 threads
let engine = ParallelExecutionEngine::new_fixed(4)?;

// Auto (system-optimal)
let engine = ParallelExecutionEngine::new_auto()?;
```

---

## `DurabilityLevel` — WAL Durability

> **File**: `src/engine/types.rs`

Controls WAL write sync policy. **`Database::open()` defaults to `Full`.**

| Value | Description | fsync | Performance | Safety |
|-------|-------------|-------|-------------|--------|
| `Full` | Immediate fsync on every write | every write | low | maximum |
| `Lazy` | Background deferred fsync | async | medium | medium |
| `None` | No WAL (memory-only) | none | maximum | data loss on crash |

> **Warning**: Use `DurabilityLevel::None` only for benchmarks or cache-only databases.

---

## WAL Internal Constants

> **File**: `src/storage/native_wos/table_store.rs`

These compile-time constants affect behavior but cannot be directly modified.

| Constant | Value | Meaning |
|----------|-------|---------|
| `WAL_COMPACT_THRESHOLD` | `5_000` | Triggers `compact()` when WAL entries exceed 5,000 |
| `PAGE_TARGET_BYTES` | `4_096` | SSTable page target size (4KB) |
| `FOOTER_SIZE` | `16` bytes | SSTable footer size |

**Compact behavior**:
```
On flush():
  WAL entries < 5,000  → sequential append to .wal file (fast)
  WAL entries >= 5,000 → compact() triggered: merge WAL + SSTable, rewrite
```

---

## `EncryptionConfig` — Encryption Settings

> **File**: `src/storage/encryption/config.rs`

### Supported Algorithms

| Algorithm | Default | Characteristics | Recommended Environment |
|-----------|:-------:|-----------------|-------------------------|
| `Aes256GcmSiv` | ✅ | AES-NI hardware acceleration, nonce-misuse resistant | Server · Desktop · Modern mobile |
| `ChaCha20Poly1305` | | High-performance software, constant-time implementation | Embedded · No AES-NI |

### Creation Methods

```rust
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// 1. Password-based (HKDF-SHA256 → 256-bit key)
let config = EncryptionConfig::from_password("my-password");

// 2. Raw 256-bit key (32 bytes)
let raw_key: [u8; 32] = [0x42u8; 32];
let config = EncryptionConfig::from_key(raw_key);

// 3. Specify algorithm
let config = EncryptionConfig::from_password("pw")
    .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);

// 4. Key + algorithm together
let config = EncryptionConfig::from_key_with_algorithm(
    raw_key,
    EncryptionAlgorithm::ChaCha20Poly1305,
);
```

### Wire Format

```
encrypted output = [nonce 12 bytes] || [ciphertext + auth_tag]
                 = self-contained (no external nonce storage needed)
```

---

## `WosVariant` — WOS Backend Selection

> **File**: `src/engine/wos_variant.rs`

Selected automatically at database creation. No manual selection required.

| Variant | When Used | Characteristics |
|---------|-----------|-----------------|
| `Native` | `Database::open()` | Native SSTable + WAL, persistent file storage |
| `InMemory` | `open_in_memory()` | Memory-only, fast, resets on restart |
| `Encrypted` | `open_encrypted()` | Native + AES/ChaCha20 encryption |

---

## `DeltaVariant` — Delta Store Implementation

> **File**: `src/engine/delta_variant.rs`

Internal implementation for Tier 1 (Delta Store).

| Variant | Description |
|---------|-------------|
| `RowBased(DeltaStore)` | BTreeMap-based row store (default) |
| `Columnar(ColumnarDelta)` | Apache Arrow RecordBatch-based columnar store |

---

## `TablePersistence` — Table Persistence

> **File**: `src/engine/types.rs`

Specifies the storage type per table.

| Value | Description |
|-------|-------------|
| `Memory` | Dropped on process exit (cache, temporary data) |
| `File` | Permanently stored to filesystem |

---

## Complete Configuration Examples

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig, DirtyBufferMode};
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// High-performance server (all cores, aggressive parallelism)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig::aggressive(),
        ..Default::default()
    },
)?;

// Conservative (half CPU, large batches only)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig::conservative(),
        ..Default::default()
    },
)?;

// High-throughput simple get/insert (DashMap mode)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        dirty_buffer_mode: DirtyBufferMode::DashMap,
        parallelism: ParallelismConfig::aggressive(),
        ..Default::default()
    },
)?;

// Encrypted DB
let enc = EncryptionConfig::from_password("secret");
let db = Database::open_encrypted(Path::new("./data"), enc)?;

// In-memory (benchmark / test)
let db = Database::open_in_memory()?;
```

---

## Configuration Scenario Summary

| Scenario | Recommended Settings |
|----------|---------------------|
| Dedicated server analytics | `open_with_config` + `aggressive()` |
| Range-heavy OLAP | `open_with_config` + `DirtyBufferMode::BTreeMap` (default) |
| High-throughput simple get/insert | `open_with_config` + `DirtyBufferMode::DashMap` |
| Development PC / laptop | `open_with_config` + `conservative()` |
| General OLTP server | `open()` (default) |
| Data protection required | `open_encrypted()` |
| Testing / caching | `open_in_memory()` |
| Benchmarking (max performance) | `open_in_memory()` or `DurabilityLevel::None` |

---

## Next Steps

- [Parallel Query Guide](parallel-query) — SQL parallel aggregation details
- [Encryption Guide](encryption) — EncryptionConfig deep dive
- [WAL Recovery Guide](wal-recovery) — How WAL works
- [Storage Layers](storage-layers) — 5-Tier architecture
