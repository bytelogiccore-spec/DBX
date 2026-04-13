# Changelog

This document follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

---

## [0.2.2] - 2026-04-13

Critical fixes for data durability and partition routing.

### Fixed
- **Delete Durability** — Added missing WAL logging for delete operations, ensuring deleted data remains deleted after a system crash or restart.
- **Partition Routing** — Fixed a critical bug where `delete`, `get`, and `get_snapshot` operations did not correctly route to sub-partitions, causing operations to fail or return inconsistent data on partitioned tables.

---

## [0.2.1-beta] - 2026-04-10

MVCC VersionedKey identification hardening and DeltaStore hot-path retrieval optimization.

### Improvements & Fixes
- **MVCC Magic Suffix** — Added a deterministic 2-byte magic suffix (`[0xDB, 0x58]`) to `VersionedKey` byte-encoding. This safely prevents subtle data corruption edge-cases by guaranteeing positive identification of MVCC-encoded keys over arbitrary raw bytes.
- **DeltaStore Zero-Overhead Keys** — Refactored the core BTreeMap inside `DeltaStore` to use bare `Vec<u8>` bytes natively instead of struct-wrapped `VersionedKey`s. This fully eliminates intermediate allocating iterations, redundant decoding overheads, and properly scales range bounded queries.
- **Multi-language Pre-Documentation** — Added initial roadmap documentation structures under `docs/Version History` in preparation for comprehensive i18n support.

---

## [0.2.0-beta] - 2026-04-03

Introduced Native SSTable-based WOS, Fast-Path ultra-low latency optimization, and Workspace refactoring.

### New Features

#### 🏗️ Native WOS (Write-Optimized Store)
- **Sled Removal** — Completely removed external KV store dependencies and introduced a native SSTable-based WOS engine.
- **Ultra-fast Flush** — Optimized WAL sequential writes and SSTable merging (Compaction) to reduce write latency.

#### 🚀 Fast-Path (Local Bypass) Optimization
- **Local Execution Bypass** — Introduced Fast-Path to bypass distributed DAG scheduling overhead in single-node environments.
- **Synchronous Data Stream** — Achieved **51µs** ultra-low latency by eliminating mpsc channel overhead with the `sync_batches` synchronous data return path.

#### 📦 Workspace Refactoring
- **Crate Separation** — Refactored tests (`dbx-tests`), benchmarks (`dbx-benchmarks`), and examples (`dbx-examples`) into separate crates to keep the core library lightweight.
- **Dependency Cleansing** — Removed unnecessary dev-dependencies from the core engine to improve build speed and maintainability.

### Improvements
- **Grid Engine** — Stabilized `s2n-quic` transport and DAG scheduling logic.
- **Version Unification** — Updated all workspace members to `0.2.0-beta`.

---

## [0.1.2-beta] - 2026-03-21

Phase 1 & Ecosystem Compatibility Update: Atomic CAS Operations, Row-level Striped Locks, Native Serde, Async First Driver, and Network-Aware Distributed Lock Manager.

### New Features

#### 🛡️ Atomic CAS & Concurrency (Phase 1)
- **Atomic CAS API** — Added `insert_if_not_exists`, `compare_and_swap`, `update_if_exists`, and `delete_if_equals` methods to `DatabaseCore`.
- **Row-level Latch Manager (Lock Striping)** — Replaced table-level mutexes with a high-performance, 1024-striped `RowLockManager` ensuring zero contention for concurrent CAS operations on different keys.

#### 🌐 Grid Engine & Distributed Locks
- **Network-Aware Distributed Lock Manager (DLM)** — Added `DistributedLockManager` with Fencing Tokens, Adaptive Leases, Heartbeat renewals, and Passive Eviction for massive grid concurrency.
- **Connection Multiplexing (`GridRouter`)** — Replaced `ReplicationMessage` with a generic `GridMessage` to route replication and lock traffic through a single QUIC connection without loopbacks.
- **`GridDatabaseAsync` Wrapper** — Introduced the "Separated Explicit Mode" wrapper avoiding DLM overhead for purely local node ops, completely preserving raw HTAP performance.

#### 🦀 Rust Ecosystem Compatibility
- **Native Serde Support** — Introduced `DatabaseSerde` trait with `insert_struct` and `get_struct` for seamless serialization of Rust structs (via `bincode`).
- **Async First Driver** — Added `DatabaseAsync` tokio-compatible non-blocking wrapper, offloading heavy I/O to `spawn_blocking` for massive async/await concurrency.

---

## [0.1.1-beta] - 2026-03-19

WAL sequential append, multi-core parallelization, Multi-Master Failover, cross-node sharding, distributed transactions, and Phase 3 partitioning synergy (auto-stats, differential compression, fully automatic lifecycle scheduler, Hot/Cold tiering).

### New Features

#### 📊 Partitioning Synergy (Phase 3)

- **INSERT auto-increments `row_count`** — per-partition `PartitionStats.row_count` updated automatically on every insert
- **`set_partition_compression`** — per-partition ZSTD level (1–9), independent of global compression
- **`enable_auto_archive(table, lifecycle)`** — single call spawns a background `dbx-lifecycle-scheduler` thread (1-hour interval, CAS-guaranteed single instance)
  - `archive_after_days` → ZSTD level 9 + Cold tier hint auto-applied
  - `delete_after_days` → metadata auto-deleted
- **`run_partition_lifecycle` / `run_all_partition_lifecycles`** — on-demand immediate execution
- **`set_partition_tier` / `get_partition_tier` / `list_partitions_by_tier`** — Hot / Warm / Cold tiering API

#### 📦 WAL / Parallelization

- **WAL sequential append** — sequential `.wal` appends; `compact()` only on `wal_entries >= 5,000`
- **`DirtyBufferMode`** — runtime-select `BTreeMap` (default) or `DashMap` for WOS dirty buffer
- **`Database::open_with_config()`** — `DbConfig` constructor with `conservative()` / `aggressive()` presets
- **Parallel improvements** — `par_iter()` for insert_batch, GROUP BY, JOIN, scan, compact, WAL encode

#### 🔄 Multi-Master Failover

- **Quorum-based leader election** — Raft-like `term` + majority vote; Split-Brain prevention via auto-demotion
- **Vector Clock** — causality-based conflict detection replacing LWW

#### 🗂️ Cross-Node Sharding

- **Weight-based vnode distribution** — `ShardNode::weight` for non-uniform allocation
- **Data rebalancing** — automatic key migration on node add/remove
- **2PC distributed transactions** — Prepare → Commit/Abort atomicity across shards

#### 🌐 QUIC Transport

- **s2n-quic QuicTransport** — TLS 1.3, HoL-blocking-free multi-stream inter-process replication
- **Runtime config** — `ReplicationConfig::in_memory()` / `ReplicationConfig::quic(...)` switch without code changes

### Internal Changes

- `Database` struct: 7 new fields for partition management (`partition_stats`, `partition_compression`, `partition_lifecycle`, `partition_tier_hints`, `partition_creation_times`, `lifecycle_stop_flag`, `lifecycle_running`)
- `crud.rs` `insert()`: partition auto-stats/timestamp hook (zero overhead for non-partitioned tables)

### Dependencies Added

- `wide = "0.7"` — stable SIMD
- `s2n-quic = "1"` — AWS QUIC
- `tokio` — `net`, `io-util` features

---

## [0.0.6-beta] - 2026-02-17

### Added

**DDL API**:
- `drop_table(table_name)` - Drop an existing table
- `table_exists(table_name)` - Check if a table exists
- `list_tables()` - List all tables in the database

**Multi-Language Support**:

All DDL APIs are now available in C/C++, C#, Node.js, and Python:

```csharp
// C#
db.DropTable("users");
bool exists = db.TableExists("users");
var tables = db.ListTables();
```

```javascript
// Node.js
db.dropTable('users');
const exists = db.tableExists('users');
const tables = db.listTables();
```

```python
# Python
db.drop_table('users')
exists = db.table_exists('users')
tables = db.list_tables()
```

**FFI Architecture**:
- **dbx-ffi**: C/C++ FFI layer
- **dbx-csharp**: C# native bindings (csbindgen)
- **dbx-node**: Node.js native bindings (N-API)
- **dbx-py**: Python native bindings (PyO3)

### Changed

**GitHub Actions**:
- Updated CI to build all FFI layers (dbx-ffi, dbx-csharp, dbx-node, dbx-py)
- Updated publish workflows to use native bindings instead of shared FFI

### Performance Improvements

Achieved **1st place in all major operations** (INSERT, GET, SCAN) through algorithmic optimizations.

#### Benchmark Results (10,000 records, Default features)

| Operation | DBX | SQLite | Sled | Redb | Rank |
|-----------|-----|--------|------|------|------|
| **INSERT** | **44.92ms** 🥇 | 53.06ms | 60.56ms | 54.05ms | **1st** |
| **GET** | **2.84ms** 🥇 | 37.39ms | 5.88ms | 3.25ms | **1st** |
| **SCAN** | **1.60ms** 🥇 | 2.98ms | 4.64ms | 2.15ms | **1st** |

#### Performance vs Competitors

**vs SQLite**:
- INSERT: 18% faster
- GET: 1,217% faster (13x)
- SCAN: 86% faster

**vs Redb**:
- INSERT: 20% faster
- GET: 14% faster
- SCAN: 34% faster

**vs Sled**:
- INSERT: 35% faster
- GET: 107% faster (2x)
- SCAN: 190% faster (2.9x)

#### Optimization Details

1. **Phase 1: GET Optimization (+70% improvement)**
   - Added `#[inline(always)]` attribute to hot path functions
   - Removed MVCC overhead for maximum performance
   - Simplified code path (removed unnecessary conditionals)
   - Result: 9.63ms → 2.84ms (3.4x faster)

2. **Phase 2: SCAN Optimization (+57% improvement)**
   - Implemented fast-path for empty Delta Store
   - Skip 2-way merge when Delta is empty
   - Direct WOS scan for better cache locality
   - Result: 3.70ms → 1.60ms (2.3x faster)

### Technical Details

#### Test Configuration
- **Platform**: Windows 11 Pro (Build 26200), x64
- **Compiler**: rustc 1.92.0 (release profile)
- **Framework**: Criterion.rs v0.5 (100 samples, 95% CI)
- **Features**: Default (wal, mvcc, index enabled)
- **Durability**: None (fair comparison)

#### Architecture
- **Delta Store**: DashMap + SkipMap (lock-free)
- **WOS**: BTreeMap (sorted storage)
- **MVCC**: Disabled in hot path for performance

### Changed
- Optimized `Database::get()` with inline attribute and simplified logic
- Optimized `Database::scan()` with Delta empty check fast-path
- Optimized `DeltaStore::scan()` with early return for empty tables

---

## [0.0.5-beta] - 2026-02-16

Full API synchronization across all language bindings. ● = existing, 🆕 = added in this release.

### Binding API Matrix

| API | Node.js | Python | FFI/C | C# | C++ |
|-----|:-------:|:------:|:-----:|:--:|:---:|
| `open` / `open_in_memory` | ● | ● | ● | ● | ● |
| `insert` / `get` / `delete` | ● | ● | ● | ● | ● |
| `count` | 🆕 | 🆕 | ● | 🆕 | ● |
| `flush` | 🆕 | 🆕 | ● | 🆕 | ● |
| `insert_batch` | ● | 🆕 | 🆕 | 🆕 | 🆕 |
| `scan` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `range` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `table_names` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `gc` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `is_encrypted` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `execute_sql` | ● | 🆕 | 🆕 | 🆕 | 🆕 |
| `create_index` / `drop_index` / `has_index` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `save_to_file` / `load_from_file` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `insert_versioned` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `get_snapshot` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| `current_timestamp` / `allocate_commit_ts` | 🆕 | 🆕 | 🆕 | 🆕 | 🆕 |
| Transaction (`begin` / `commit` / `rollback`) | ● | ● | ● | ● | 🆕 |

> **FFI Note**: Collection returns use opaque handle pattern (`DbxScanResult`, `DbxStringList`) with accessor + free functions.

### Fixed

- Fixed `clippy::manual-c-str-literals` warning in `dbx-ffi` (`b"No error\0"` → `c"No error"`)

---

## [0.0.4-beta] - 2026-02-15

First feature release. Full query execution pipeline optimization.

### New Features

- **Query Plan Cache** — Two-tier (memory + disk) cache that skips parsing and optimization for repeated SQL queries
- **Parallel Query Execution** — Rayon thread pool-based parallel filtering, aggregation, and projection for large datasets
- **WAL Partitioning** — Per-table WAL partitions to eliminate write bottlenecks
- **Schema Versioning** — Zero-downtime DDL support with schema change history and per-version rollback
- **Index Versioning** — Index rebuild history tracking with performance metrics
- **Feature Flags** — Runtime toggle system for individual features (supports environment variables and file persistence)
- **UDF Framework** — User-defined functions (scalar, aggregate, table), triggers, and schedulers
- **Benchmark Framework** — Criterion-based performance measurement with before/after comparison tools
- **PTX Persistent Kernel** — NVRTC-based runtime CUDA kernel compilation for persistent GPU processing (optional, behind `gpu` feature)
- **Hash/Range Sharding** — GPU shard strategies: hash-based (ahash) and range-based row distribution
- **CUDA Stream Management** — Separate stream creation via `fork_default_stream()`
- **Schema-based INSERT Serialization** — Column-named JSON object serialization when table schema is available
- **JOIN Optimization** — Size-based build/probe table swap for INNER JOIN (smaller table as build)
- **Tombstone Deletion** — Versioned tombstone support in columnar delta storage
- **Table-specific Cache Invalidation** — Selective eviction by table name instead of full cache clear

### Performance Improvements

| Metric | Before | After | Improvement |
|--------|:------:|:-----:|:-----------:|
| Repeated SQL parsing (10x) | 146 µs | 20 µs | 7.3x |
| WAL append (100 entries) | 1,016 µs | 71 µs | 14.2x |
| Schema lookup (single-thread) | 86 ns | 46 ns | 47% |
| Schema lookup (8 threads) | 7.4M ops/s | 18.1M ops/s | 2.44x |
| Small aggregation (150 rows) | 32.5 µs | 991 ns | 33x |

### Refactored

- **SQL Optimizer** — Split 874-line monolithic `optimizer.rs` into modular directory structure (6 files: trait, 4 rules, tests)
- **CREATE FUNCTION** — Actual parameter parsing from parenthesized arguments
- **ORDER BY** — Activated test for `sqlparser` 0.52 `OrderBy.exprs` API

### Internal Changes

- Migrated `SchemaVersionManager` storage from `RwLock<HashMap>` to `DashMap` for improved concurrent read performance
- Changed `ParallelQueryExecutor` parallelization criteria from batch count to **total row count** (defaults to sequential execution below 1,000 rows)
- Applied dynamic threading and automatic batch size tuning to the SQL parser
- Documented `cudarc` 0.19.2 limitations for Unified Memory, P2P detection, and persistent kernels

### Dependencies

- Added `dashmap` 6.x (lock-free concurrent hashmap)
- Added `rayon` 1.x (parallel processing)
- Added `criterion` 0.5 (benchmarking)

---

## [0.0.3-beta] - 2026-02-14

### Changed

- Restricted crates.io publishing to `dbx-core` only
- Unified license badges to `MIT OR Commercial`
- Added per-language API guides (Python, Node.js, .NET)
- Added API reference section to GitHub Pages

---

## [0.0.2-beta] - 2026-02-13

### Changed

- Built bilingual documentation (Korean/English) for Python, Node.js, .NET, C/C++
- Eliminated all build errors and warnings
- Removed `dbx-derive` macro crate
- Switched CI workflows to manual-trigger only

---

## [0.0.1-beta] - 2026-02-12

Initial release.

### Features

- SQL parser (SELECT, INSERT, CREATE TABLE, DROP TABLE)
- Arrow RecordBatch-based columnar storage
- MVCC transactions (Snapshot Isolation)
- Write-Ahead Logging (WAL)
- B-Tree indexing
- Language bindings: Python, Node.js, C#, C/C++
