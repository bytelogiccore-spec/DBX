---
layout: default
title: English
nav_order: 2
has_children: true
description: "DBX — High-Performance Embedded Database"
---

# DBX
{: .fs-9 }

High-performance embedded database built on a 5-Tier Hybrid Storage architecture. Designed for HTAP (Hybrid Transactional/Analytical Processing) workloads, implemented in pure Rust.
{: .fs-6 .fw-300 }

[Get Started](getting-started){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/bytelogiccore-spec/DBX){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## Key Features

### 🏗️ Architecture
- **5-Tier Hybrid Storage** — Delta → Cache → Native WOS → Index → ROS
- **Native WOS** — High-performance SSTable-based engine (removed Sled dependency)
- **HTAP Power** — Seamless coexistence of OLTP and OLAP workloads
- **Distributed Grid** — QUIC-based real-time replication and distributed execution

### 🚀 Performance
- **29x faster** file GET than SQLite
- **Fast-Path** — 51µs latency for point lookups
- **GPU Acceleration** — CUDA-based aggregation, filtering, joins
- **SIMD Vectorization** — Optimized numerical operations
- **Parallel Query** — Rayon-based parallel JOIN, Sort, Columnar Build

### 🔐 Security
- **AES-256-GCM-SIV** — Industry-standard encryption
- **ChaCha20-Poly1305** — High-speed mobile encryption
- **Key Rotation** — Zero-downtime key updates

### 🌐 Multi-Language
- **Rust** — Native API
- **Python** — PyO3-based bindings
- **C#/.NET** — P/Invoke FFI
- **C/C++** — Standard C API
- **Node.js** — Native N-API bindings

---

## Quick Example

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// CRUD
db.insert("users", b"user:1", b"Alice")?;
let val = db.get("users", b"user:1")?;

// SQL
let results = db.execute_sql("SELECT * FROM users WHERE age > 25")?;

// Transactions
let tx = db.begin_transaction()?;
tx.insert("users", b"user:2", b"Bob")?;
tx.commit()?;
```
