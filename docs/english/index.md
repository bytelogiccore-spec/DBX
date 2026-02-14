---
layout: default
title: Home
nav_order: 1
description: "DBX — High-Performance Embedded Database"
permalink: /
---

# DBX
{: .fs-9 }

High-performance embedded database built on a 5-Tier Hybrid Storage architecture. Designed for HTAP (Hybrid Transactional/Analytical Processing) workloads, implemented in pure Rust.
{: .fs-6 .fw-300 }

[Get Started](getting-started){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/ByteLogicCore/DBX){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Key Features

### 🏗️ Architecture
- **5-Tier Hybrid Storage** — Delta → Cache → WOS → Index → ROS
- **HTAP Support** — Concurrent OLTP and OLAP workloads
- **MVCC Transactions** — Snapshot Isolation with Garbage Collection
- **Columnar Cache** — Apache Arrow-based analytical query optimization

### ⚡ Performance
- **GPU Acceleration** — CUDA-based aggregation and filtering (up to 4.57x faster)
- **Query Optimization** — Projection Pushdown, Predicate Pushdown
- **Zero-copy Operations** — Direct Arrow RecordBatch utilization
- **Vectorized Execution** — SIMD vectorized operations

### 🔒 Security & Reliability
- **Encryption** — AES-256-GCM-SIV, ChaCha20-Poly1305
- **Compression** — ZSTD, Brotli
- **WAL 2.0** — Bincode binary serialization with async fsync
- **ACID** — Full transaction guarantees and crash recovery

### 🎯 Developer Experience
- **Pure Rust** — Memory safety guaranteed
- **SQL Support** — SELECT, WHERE, JOIN, GROUP BY, ORDER BY
- **Embedded** — No separate server required
- **Well-tested** — 100+ integration tests

---

## Quick Example

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // Open database
    let db = Database::open("./data")?;
    
    // Insert data
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    
    // Query data
    let value = db.get("users", b"user:1")?;
    assert_eq!(value, Some(b"Alice".to_vec()));
    
    Ok(())
}
```

---

## Performance Highlights

| Operation | CPU | GPU | Speedup |
|-----------|-----|-----|---------|
| SUM | 456.66µs | 783.36µs | 0.58x |
| Filter (>500K) | 2.06ms | 673.38µs | **3.06x** |

*Benchmarked on 1,000,000 rows. GPU shows greater gains on larger datasets (>10M rows).*

---

## Documentation

### 📚 Guides

Comprehensive feature guides:

- **[CRUD Operations](guides/crud-operations)** — Complete CRUD guide
- **[SQL Reference](guides/sql-reference)** — Full SQL syntax reference
- **[Transactions](guides/transactions)** — MVCC and snapshot isolation
- **[GPU Acceleration](guides/gpu-acceleration)** — CUDA-based query acceleration
- **[Storage Layers](guides/storage-layers)** — 5-Tier architecture deep dive
- **[Language Bindings](guides/language-bindings)** — Python, C#, C/C++, Node.js
- **[Encryption](guides/encryption)** — AES-256 and ChaCha20 encryption
- **[Compression](guides/compression)** — ZSTD compression
- **[Indexing](guides/indexing)** — Bloom Filter indexes
- **[WAL Recovery](guides/wal-recovery)** — Write-Ahead Logging and crash recovery

### 🎓 Tutorials

Step-by-step tutorials for learning DBX:

- **[Beginner Tutorial](tutorials/beginner)** — Your first DBX database

### 📖 Examples

Practical code examples:

- **[Quick Start](examples/quick-start)** — 5분 시작 가이드
- **[SQL Quick Start](examples/sql-quick-start)** — SQL 기본 사용법
- **[Encryption](examples/encryption)** — Data encryption
- **[Compression](examples/compression)** — Data compression
- **[Indexing](examples/indexing)** — Index creation and usage
- **[WAL Recovery](examples/wal-recovery)** — Crash recovery

### 🔧 API Reference

Complete API documentation:

- **[Database API](api/database)** — Core database operations
- **[Transaction API](api/transaction)** — Transaction management
- **[SQL API](api/sql)** — SQL execution

### 🗺️ Roadmap

- **[Roadmap](roadmap)** — Future features and development plan

---

## Getting Started

Ready to dive in? Check out our [Getting Started Guide](getting-started) to install DBX and run your first queries.

For detailed architecture information, see the [Architecture Guide](architecture).

---

## License

MIT OR Apache-2.0
