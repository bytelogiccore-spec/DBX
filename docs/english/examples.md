---
layout: default
title: Examples
nav_order: 10
parent: English
description: "Code examples for DBX"
---

# Examples

Practical code examples demonstrating DBX features.

## 🚀 Getting Started

Start here if you're new to DBX:

- [**Quick Start**](./examples/quick-start.md) - 5분 시작 가이드 (CRUD 기본)
- [**SQL Quick Start**](./examples/sql-quick-start.md) - SQL 기본 사용법

## 🔒 Security & Data Protection

Protect your data with encryption and compression:

- [**Encryption**](./examples/encryption.md) - AES-256-GCM-SIV and ChaCha20-Poly1305 encryption
- [**Compression**](./examples/compression.md) - ZSTD compression for space savings

## ⚡ Performance Optimization

Maximize performance with these features:

- [**Indexing**](./examples/indexing.md) - Bloom Filter indexes for fast lookups

## 🔄 Reliability

Ensure data durability:

- [**WAL Recovery**](./examples/wal-recovery.md) - Write-Ahead Log for crash recovery

## 📚 Example Categories

### By Complexity

| Level | Examples |
|-------|----------|
| **Beginner** | [Quick Start](./examples/quick-start.md), [SQL Quick Start](./examples/sql-quick-start.md) |
| **Intermediate** | [Indexing](./examples/indexing.md), [Encryption](./examples/encryption.md) |
| **Advanced** | [Compression](./examples/compression.md), [WAL Recovery](./examples/wal-recovery.md) |

### By Feature

| Feature | Examples |
|---------|----------|
| **Storage** | [Quick Start](./examples/quick-start.md), [Compression](./examples/compression.md) |
| **Query** | [SQL Quick Start](./examples/sql-quick-start.md), [Indexing](./examples/indexing.md) |
| **Reliability** | [WAL Recovery](./examples/wal-recovery.md) |
| **Security** | [Encryption](./examples/encryption.md) |

## 🎯 Quick Navigation

**I want to...**

- **Store and retrieve data** → [Quick Start](./examples/quick-start.md)
- **Run SQL queries** → [SQL Quick Start](./examples/sql-quick-start.md)
- **Protect sensitive data** → [Encryption](./examples/encryption.md)
- **Speed up lookups** → [Indexing](./examples/indexing.md)
- **Reduce disk usage** → [Compression](./examples/compression.md)
- **Ensure durability** → [WAL Recovery](./examples/wal-recovery.md)

## 💻 Running Examples

All examples are located in `core/dbx-core/examples/` and can be run with:

```bash
# List all examples
cargo run --example

# Run a specific example
cargo run --example encryption
cargo run --example transactions
cargo run --example gpu_acceleration
```

## 📖 Documentation Structure

Each example includes:

- **Overview**: What the feature does
- **Quick Start**: Minimal code to get started
- **Step-by-Step Guide**: Detailed walkthrough
- **Complete Example**: Full working code
- **Performance Tips**: Optimization recommendations
- **Next Steps**: Related examples and features

## 🔗 Related Resources

- [Architecture](../architecture.md) - Understand DBX's 5-Tier Hybrid Storage
- [Benchmarks](../benchmarks.md) - Performance comparisons
- [API Reference](../api/) - Detailed API documentation

---

**Need help?** [Open an issue](https://github.com/bytelogiccore-spec/DBX/issues).
