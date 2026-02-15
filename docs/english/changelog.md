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
