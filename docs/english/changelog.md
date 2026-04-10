---
layout: default
title: Changelog
nav_order: 8
parent: English
---

# Changelog

All notable changes to DBX will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

WAL sequential append, multi-core parallelization, Multi-Master Failover, cross-node sharding enhancements, distributed transactions, and Phase 3 partitioning synergy.

### New Features

#### 📊 Partitioning Synergy (Phase 3)

- **INSERT auto-increments row_count** — Every INSERT into a partitioned table automatically increments `row_count` for the target partition. No manual call needed
- **`update_partition_stats(table, partition, stats)`** — Manual precision stats for the query optimizer (min/max/null/distinct)
- **`get_partition_stats` / `all_partition_stats`** — Per-partition statistics queries
- **`set_partition_compression(table, partition, config)`** — Per-partition independent compression level (ZSTD 1–9)
- **`get_partition_compression`** — Query current setting (returns Snappy default if unset)
- **`enable_auto_archive(table, lifecycle)`** — Single call activates full automation
  - Immediately spawns `dbx-lifecycle-scheduler` background thread (1-hour interval)
  - Only one thread regardless of how many tables are registered (CAS `compare_exchange` guarantee)
  - `archive_after_days` elapsed → ZSTD level 9 + Cold tier auto-applied
  - `delete_after_days` elapsed → partition metadata auto-deleted
- **`run_partition_lifecycle(table)`** — On-demand immediate execution, returns `(archived, deleted)`
- **`run_all_partition_lifecycles()`** — Batch immediate execution for all registered tables
- **`get_partition_creation_time(partition)`** — Auto-recorded first-write timestamp (set on INSERT)
- **`partition_needs_archive` / `partition_needs_delete`** — Manual condition checks
- **`set_partition_tier(table, partition, hint)`** — Set `Hot` / `Warm` / `Cold` tier hint
- **`get_partition_tier`** — Query current tier (returns `Hot` default if unset)
- **`list_partitions_by_tier(table, hint)`** — List partitions of a given tier

#### 📦 WAL / Parallelization

- **WAL sequential append** — Sequential appends to WAL file instead of full rewrite on WOS flush. `compact()` triggered only when `wal_entries >= WAL_COMPACT_THRESHOLD`
- **`ParallelismConfig` / `DbConfig`** — Control CPU core usage ratio (`cpu_cap`) and parallelization threshold (`min_rows_for_parallel`)
- **`DirtyBufferMode`** — Runtime-selectable data structure for WOS `dirty` buffer: `BTreeMap` (default, range query optimal) or `DashMap` (concurrent optimal). Freely switchable between restarts
- **`Database::open_with_config()`** — New constructor accepting `DbConfig`. Provides `conservative()` / `aggressive()` presets
- **`Compactor::bypass_flush_tables()`** — New API to bypass_flush multiple tables simultaneously

#### 🔄 Multi-Master Failover

- **Quorum-based leader election** — Stable master election via `term` numbers and majority vote counting (Raft-like). Lower-term masters auto-demoted to Slave to prevent Split-Brain (`replication/node.rs`, `replication/protocol.rs`)
- **Vector Clock** — Causality-based conflict detection replacing LWW. `HappensBefore` / `Concurrent` determination for lossless conflict resolution (`replication/vector_clock.rs`)

#### 🗂️ Cross-Node Sharding Enhancements

- **Weight-based vnode distribution** — `ShardNode::weight` field for non-uniform data allocation per node (`sharding/node_ring.rs`, `sharding/router.rs`)
- **Data rebalancing** — Automatic key migration for affected hash ranges when adding/removing nodes. `compute_tasks()` + `execute()` pattern (`sharding/rebalancer.rs`)
- **2PC distributed transactions** — Two-phase commit (Prepare → Commit/Abort) for cross-node atomicity. Full rollback if any participant fails (`sharding/two_phase.rs`)

#### 🌐 QUIC-based Transport Layer

- **s2n-quic QuicTransport** — Real inter-process communication using AWS `s2n-quic` (v1.76). Built-in TLS 1.3, Head-of-Line Blocking-free multi-stream (`replication/transport.rs`)
- **Runtime transport config** — Switch between single-process ↔ distributed without code changes via `ReplicationConfig::in_memory()` / `ReplicationConfig::quic(...)`
- **QuicNode server/client mode** — Async `QuicNode::server()` / `QuicNode::client()` initialization with bincode serialization, 4-byte length-prefix framing, and self-signed certificate helper

### Performance Improvements

| Item | Detail | File |
|------|---------|------|
| P1 | `insert_batch()` — parallel insertion via `par_iter()` for 1,000+ rows | `crud.rs` |
| P2 | `get_batches()` projection — parallel column selection via `par_iter()` | `columnar_cache.rs` |
| P3 | GROUP BY aggregation — parallel aggregation via `par_iter()` for 1,000+ groups | `hash_aggregate.rs` |
| P4 | JOIN build/probe — `into_par_iter()` (1,000-row threshold) | `join.rs` |
| P5 | `scan()` — concurrent Delta+WOS scan via `rayon::join()` | `crud.rs` |
| P6 | `compact()` — page deserialization parallelized via `par_iter()` | `table_store.rs` |
| P7 | SIMD — switched to `wide` crate stable (nightly removed, always active) | `simd.rs` |
| P8 | WAL encode — serialization via `par_iter()`, file writes sequential | `table_store.rs` |
| P9 | Compaction — batch `Arc::clone` collected in parallel via `par_iter()` | `compaction.rs` |

### Internal Changes

- `Database` struct: added `partition_stats`, `partition_compression`, `partition_lifecycle`, `partition_tier_hints`, `partition_creation_times`, `lifecycle_stop_flag`, `lifecycle_running` fields
- `crud.rs` `insert()`: added partition auto-stats/timestamp hook (zero overhead for non-partitioned tables)

### Dependencies Added

- `wide = "0.7"` — stable SIMD abstraction crate
- `s2n-quic = "1"` — AWS QUIC implementation (inter-process replication)
- `tokio` — added `net`, `io-util` features

### Tests

23 new integration tests added (all passing). Existing regression: 78 integration, 509 unit tests — no regressions.

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

## [0.0.3-beta] - 2026-02-15

### Added
- Comprehensive usage guides for Python, Node.js, and .NET packages
  - JSON data handling examples
  - Batch operations and error handling
  - Real-world examples (KV Store, Session Manager, Cache Wrapper)
  - TypeScript support for Node.js
  - ASP.NET Core integration examples
- Bilingual documentation (English + Korean) for all language bindings

### Changed
- **Platform Support**: Corrected to **Windows x64 only** (Linux/macOS planned)
- **Cargo.toml**: `homepage` now points to GitHub Pages instead of bytelogic.studio
- **crates.io**: Only `dbx-core` is published (removed `dbx-derive` and `dbx-ffi`)
- **Documentation**: Removed Derive Macro section (not used in production)
- **Doc Comments**: Converted Rust doc comments to English for docs.rs consistency

### Fixed
- Over-claimed platform support (was: all platforms, now: Windows x64 only)
- Version inconsistencies across packages

---

## [0.0.2-beta] - 2026-02-15

### Added
- Package documentation for all language bindings (Rust, .NET, Python, Node.js, C/C++)
- GitHub Pages bilingual docs (English + Korean) for each package
- CHANGELOG.md
- NuGet package metadata (version, license, readme)
- `readme` field in all Rust crate Cargo.toml files
- `permissions: contents: write` for GitHub Release workflow

### Changed
- **CI/CD**: Split monolithic release workflow into independent per-registry workflows
  - `publish-crates.yml` — crates.io (dbx-derive → dbx-core → dbx-ffi)
  - `publish-nuget.yml` — NuGet
  - `publish-pypi.yml` — PyPI
  - `publish-npm.yml` — npm
  - `release.yml` — Build + Test + GitHub Release only
- **Versions**: Unified all packages to `0.0.2-beta`
- **License**: Simplified to `MIT` for crates.io compatibility
- **Workspace metadata**: Added `repository`, `homepage`, `documentation` inheritance
- **crates.io**: Removed `|| true` from publish commands, added `--no-verify`, increased index wait to 60s

### Fixed
- NuGet 403 error: API key permission guidance
- PyPI 400 error: Version format corrected to PEP 440 (`0.0.2b0`)
- npm EOTP error: Granular Access Token guidance for 2FA bypass
- crates.io circular dependency: Removed `version` from `dbx-derive` dev-dependency
- GitHub Release 403: Added `contents: write` permission
- `edition = "2024"` preserved for `let chains` syntax support

---

## [0.0.1-beta] - 2026-02-12

### Added
- Initial release
- 5-Tier Hybrid Storage engine (WOS → L0 → L1 → L2 → Cold)
- MVCC transaction support with snapshot isolation
- SQL engine (CREATE TABLE, INSERT, SELECT, UPDATE, DELETE)
- Write-Ahead Logging (WAL) for crash recovery
- Language bindings: Rust, C#/.NET, Python, Node.js, C/C++
- Encryption support (AES-GCM-SIV, ChaCha20-Poly1305)
- Arrow/Parquet native columnar format
- GitHub Pages documentation site
- CI/CD pipeline with GitHub Actions
- Comparison benchmarks vs SQLite, Sled, Redb
