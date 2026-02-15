# Changelog

This document follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format.

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
- Language bindings: Python, Node.js, C#, C/C++, WASM
