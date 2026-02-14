# DBX — High-Performance Embedded Database

[![Version](https://img.shields.io/badge/version-0.0.2--beta-blue.svg)](https://github.com/ByteLogicCore/DBX)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Commercial-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://bytelogiccore-spec.github.io/DBX/)

> **29x faster file GET** than SQLite • Pure Rust • GPU-Accelerated • MVCC Transactions

**DBX** is a next-generation embedded database built on a **5-Tier Hybrid Storage** architecture, designed for modern HTAP (Hybrid Transactional/Analytical Processing) workloads.

---

## 💖 Support This Project

If you find DBX useful, please consider supporting its development!

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/Q5Q41TDHWG)

Your support helps with:
- 🚀 New features and performance optimizations
- 🐛 Bug fixes and stability improvements
- 📚 Documentation and tutorials
- 💻 Test infrastructure and CI/CD maintenance

---

## ⚡ Why DBX?

### 🏆 Blazing Fast Performance

**Latest Benchmark Results (10,000 records):**

| Benchmark | DBX | SQLite | Speedup |
|-----------|-----|--------|---------|
| **Memory INSERT** | 25.37 ms | 29.50 ms | **1.16x faster** ✅ |
| **File GET** | 17.28 ms | 497.64 ms | **28.8x faster** 🔥🔥🔥 |

### 🎯 Key Advantages

- **🚀 5-Tier Hybrid Storage** — Optimized for both OLTP and OLAP workloads
- **🎮 GPU Acceleration** — CUDA-powered analytics (up to 4.5x faster filtering)
- **🔒 MVCC Transactions** — Snapshot Isolation with zero-lock reads
- **💾 Columnar Cache** — Apache Arrow-based query optimization
- **🔐 Enterprise Security** — AES-256-GCM-SIV encryption, ZSTD compression
- **🦀 Pure Rust** — Memory safety guaranteed, zero-cost abstractions

📊 **[Full Benchmark Report](https://bytelogiccore-spec.github.io/DBX/english/benchmarks)** — Detailed comparison vs SQLite, Sled, Redb

## 📦 5-Tier Hybrid Storage Architecture

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← In-memory write buffer (528K rec/sec)
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← OLAP optimization (Projection Pushdown)
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (sled)                     │  ← MVCC Snapshot Isolation
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← Fast existence check
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Parquet)                  │  ← Columnar compression
└─────────────────────────────────────────┘

                  Optional: GPU Acceleration (CUDA)
```

🏗️ **[Architecture Deep Dive](https://bytelogiccore-spec.github.io/DBX/english/architecture)** — How DBX achieves 6.7x performance

---

## 🌐 Language Bindings

DBX provides official bindings for multiple languages:

- **Python** - Pythonic API with context managers
- **C#/.NET** - High-performance .NET bindings
- **C/C++** - Low-level C API and modern C++17 wrapper
- **Node.js** - Native N-API bindings

**[View Language Bindings Guide →](https://bytelogiccore-spec.github.io/DBX/english/guides/language-bindings)**

---

## 📚 Documentation

### 🎓 Getting Started
- **[Quick Start Guide](https://bytelogiccore-spec.github.io/DBX/english/getting-started)** — Install and run your first query
- **[Beginner Tutorial](https://bytelogiccore-spec.github.io/DBX/english/tutorials/beginner)** — Step-by-step learning path

### 📖 Feature Guides
- **[CRUD Operations](https://bytelogiccore-spec.github.io/DBX/english/guides/crud-operations)** — Insert, read, delete, batch operations
- **[Transactions](https://bytelogiccore-spec.github.io/DBX/english/guides/transactions)** — MVCC, Snapshot Isolation, concurrency
- **[SQL Reference](https://bytelogiccore-spec.github.io/DBX/english/guides/sql-reference)** — Supported syntax and query optimization
- **[Storage Layers](https://bytelogiccore-spec.github.io/DBX/english/guides/storage-layers)** — 5-Tier architecture explained
- **[GPU Acceleration](https://bytelogiccore-spec.github.io/DBX/english/guides/gpu-acceleration)** — CUDA setup and performance tuning

### 🔬 Advanced Topics
- **[Architecture Guide](https://bytelogiccore-spec.github.io/DBX/english/architecture)** — Design principles and internals
- **[Performance Benchmarks](https://bytelogiccore-spec.github.io/DBX/english/benchmarks)** — DBX vs SQLite/Sled/Redb comparison
- **[Examples](https://bytelogiccore-spec.github.io/DBX/english/examples)** — Code examples and use cases

---

## ✨ Features

### Core Features ✅
- ✅ **5-Tier Hybrid Storage** — Delta → Cache → WOS → Index → ROS
- ✅ **MVCC Transactions** — Snapshot Isolation, Garbage Collection
- ✅ **SQL Support** — SELECT, WHERE, JOIN, GROUP BY, ORDER BY
- ✅ **GPU Acceleration** — CUDA-based aggregation and filtering
- ✅ **Encryption** — AES-256-GCM-SIV, ChaCha20-Poly1305
- ✅ **Compression** — ZSTD, Brotli
- ✅ **WAL 2.0** — Write-Ahead Logging with async fsync
- ✅ **100+ Tests** — Comprehensive integration test suite

### Roadmap 🚧
- **Phase 1: Trigger System** — BEFORE/AFTER triggers, conditional logic
- **Phase 2: User-Defined Functions** — Scalar, Aggregate, and Table UDFs
- **Phase 3: Partitioning** — Range, Hash, and List partitioning with pruning
- **Phase 4: Job Scheduler** — Automated maintenance and periodic tasks
- **Phase 5: Advanced Features** — Materialized Views, Replication, Sharding

---

## 📄 License

DBX is available under a **dual-license model**:

- **🆓 MIT License** — Free for individuals, startups, and small organizations
- **💼 Commercial License** — Required for large organizations (100+ employees or $5M+ revenue)

📚 **[License Policy Guide](./legal/english/LICENSE-POLICY.md)** — Which license do I need?

📧 **Commercial licensing:** license@bytelogic.studio

---

## 🤝 Contributing

Issues and PRs are always welcome!

Please read our [Contributing Guide](./legal/english/CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

---

## 🙏 Acknowledgments

- [Apache Arrow](https://arrow.apache.org/) - Columnar data processing
- [sled](https://github.com/spacejam/sled) - Embedded KV store (WOS backend)
- [cudarc](https://github.com/coreylowman/cudarc) - Rust CUDA bindings

---

**Made with ❤️ in Rust**
