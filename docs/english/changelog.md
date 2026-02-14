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
