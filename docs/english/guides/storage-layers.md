---
layout: default
title: Storage Layers
parent: English
nav_order: 23
description: "Understanding DBX's 5-Tier Hybrid Storage architecture"
---

# Storage Layers
{: .no_toc }

Deep dive into DBX's 5-Tier Hybrid Storage architecture.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX uses a sophisticated **5-Tier Hybrid Storage** architecture designed to optimize both OLTP (transactional) and OLAP (analytical) workloads.

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← In-memory write buffer
│     - Lock-free concurrency             │
│     - Hot data caching                  │
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← OLAP optimization
│     - RecordBatch caching               │
│     - Projection Pushdown               │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (sled)                     │  ← Persistent storage
│     - Write-Optimized Store             │
│     - MVCC with Snapshot Isolation      │
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← Fast existence check
│     - Minimize false positives          │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Parquet)                  │  ← Columnar compression
│     - Read-Optimized Store              │
│     - Apache Arrow/Parquet              │
└─────────────────────────────────────────┘
```

---

## Tier 1: Delta Store

### Purpose

The Delta Store is an **in-memory write buffer** that provides:
- Ultra-fast writes
- Hot data caching
- Lock-free concurrent reads
- Automatic flushing to persistent storage

### Implementation

```rust
// Internal structure (conceptual)
struct DeltaStore {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
    flush_threshold: usize,
}
```

### Characteristics

| Feature | Details |
|---------|---------|
| **Data Structure** | `BTreeMap<Vec<u8>, Vec<u8>>` |
| **Concurrency** | Lock-free reads, synchronized writes |
| **Capacity** | Configurable (default: 10,000 records) |
| **Flush Trigger** | Size threshold or manual flush |
| **Persistence** | Volatile (lost on crash without WAL) |

### Usage

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Writes go to Delta Store first
    db.insert("users", b"user:1", b"Alice")?;
    
    // Reads check Delta Store first (fastest path)
    let value = db.get("users", b"user:1")?;
    
    // Manual flush to WOS
    db.flush_delta("users")?;
    
    Ok(())
}
```

### Performance

- **Write**: O(log n) - BTreeMap insertion
- **Read**: O(1) - Direct map lookup
- **Memory**: ~100 bytes per record (overhead)

---

## Tier 2: Columnar Cache

### Purpose

The Columnar Cache provides **OLAP optimization** through:
- Apache Arrow RecordBatch caching
- Columnar data layout
- Vectorized execution (SIMD)
- Zero-copy operations

### Implementation

```rust
// Internal structure (conceptual)
struct ColumnarCache {
    batches: Vec<RecordBatch>,
    schema: Arc<Schema>,
    last_sync: Instant,
}
```

### Characteristics

| Feature | Details |
|---------|---------|
| **Format** | Apache Arrow RecordBatch |
| **Layout** | Columnar (column-oriented) |
| **Operations** | Vectorized SIMD |
| **Sync** | Delta Store → Columnar Cache |
| **Use Case** | Analytical queries (SQL) |

### Usage

```rust
use dbx_core::Database;
use arrow::array::{Int32Array, StringArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
    ]));
    
    // Create RecordBatch
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie"])),
        ],
    ).unwrap();
    
    // Register table (goes to Columnar Cache)
    db.register_table("users", vec![batch]);
    
    // SQL queries use Columnar Cache
    let results = db.execute_sql("SELECT * FROM users WHERE id > 1")?;
    
    Ok(())
}
```

### Optimizations

#### Projection Pushdown

Only read required columns:

```rust
// Only reads 'name' column from cache
let results = db.execute_sql("SELECT name FROM users")?;
```

#### Predicate Pushdown

Filter during scan:

```rust
// Filter applied at cache level
let results = db.execute_sql("SELECT * FROM users WHERE id > 100")?;
```

#### Vectorized Execution

Process multiple rows simultaneously using SIMD:

```rust
// Vectorized aggregation
let results = db.execute_sql("SELECT SUM(amount) FROM orders")?;
```

---

## Tier 3: WOS (Write-Optimized Store)

### Purpose

WOS provides **persistent transactional storage**:
- ACID guarantees
- MVCC with Snapshot Isolation
- Crash recovery
- Compaction

### Implementation

Built on `sled` embedded database:

```rust
// Internal structure (conceptual)
struct WOS {
    db: sled::Db,
    mvcc: MvccManager,
}
```

### Characteristics

| Feature | Details |
|---------|---------|
| **Backend** | sled (embedded KV store) |
| **Persistence** | Disk-based |
| **Transactions** | MVCC Snapshot Isolation |
| **Durability** | fsync on commit |
| **Compaction** | Automatic background |

### MVCC Versioning

Each record stores multiple versions:

```rust
// Key format: table:key:version
// Value: serialized data + metadata
struct VersionedValue {
    value: Vec<u8>,
    version: u64,
    deleted: bool,
}
```

### Usage

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Transaction writes go to WOS
    let tx = db.begin_transaction()?;
    tx.insert("users", b"user:1", b"Alice")?;
    tx.commit()?; // Persisted to WOS
    
    Ok(())
}
```

---

## Tier 4: Index (Bloom Filter)

### Purpose

Bloom Filter provides **fast existence checks**:
- Minimize false positives
- Reduce unnecessary ROS reads
- Space-efficient

### Implementation

```rust
// Internal structure (conceptual)
struct BloomIndex {
    filter: BloomFilter,
    false_positive_rate: f64,
}
```

### Characteristics

| Feature | Details |
|---------|---------|
| **Type** | Probabilistic data structure |
| **False Positives** | Possible (configurable rate) |
| **False Negatives** | Never |
| **Space** | ~10 bits per element |
| **Lookup** | O(1) |

### Usage

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Bloom filter checked before ROS lookup
    let value = db.get("users", b"user:1")?;
    
    // Rebuild index after bulk operations
    db.rebuild_index("users")?;
    
    Ok(())
}
```

### Performance Impact

```
Without Bloom Filter:
  Read miss: Check Delta → WOS → ROS (slow)

With Bloom Filter:
  Read miss: Check Delta → WOS → Bloom → (skip ROS if not present)
  Speedup: ~3-5x for non-existent keys
```

---

## Tier 5: ROS (Read-Optimized Store)

### Purpose

ROS provides **long-term columnar storage**:
- Apache Parquet format
- High compression ratios
- Efficient analytical scans
- Schema evolution

### Implementation

```rust
// Internal structure (conceptual)
struct ROS {
    parquet_files: Vec<PathBuf>,
    metadata: ParquetMetadata,
}
```

### Characteristics

| Feature | Details |
|---------|---------|
| **Format** | Apache Parquet |
| **Compression** | ZSTD, Snappy, Brotli |
| **Layout** | Columnar |
| **Encoding** | Dictionary, RLE, Delta |
| **Statistics** | Min/Max, Null count |

### Compression Ratios

| Data Type | Typical Compression |
|-----------|---------------------|
| Integers | 5-10x |
| Strings | 3-5x |
| Timestamps | 10-20x |
| Floats | 2-4x |

### Usage

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Compact WOS to ROS
    db.compact("users")?;
    
    // Reads from ROS (if not in Delta/WOS)
    let value = db.get("users", b"user:1")?;
    
    Ok(())
}
```

### Parquet Features

#### Column Pruning

```rust
// Only reads 'name' column from Parquet
let results = db.execute_sql("SELECT name FROM users")?;
```

#### Predicate Pushdown

```rust
// Uses Parquet statistics to skip row groups
let results = db.execute_sql("SELECT * FROM users WHERE age > 30")?;
```

---

## Data Flow

### Write Path

```
Application
    ↓
1. Delta Store (in-memory)
    ↓ (auto-flush on threshold)
2. WOS (persistent, transactional)
    ↓ (manual/auto compaction)
3. ROS (compressed, columnar)
```

Example:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 1. Write to Delta Store
    db.insert("users", b"user:1", b"Alice")?;
    
    // 2. Flush to WOS
    db.flush_delta("users")?;
    
    // 3. Compact to ROS
    db.compact("users")?;
    
    Ok(())
}
```

### Read Path (OLTP)

```
Application
    ↓
1. Check Delta Store → if found, return
    ↓
2. Check WOS → if found, return
    ↓
3. Check Bloom Filter → if not present, return None
    ↓
4. Read from ROS → return
```

### Read Path (OLAP)

```
Application (SQL query)
    ↓
1. Query Optimizer
    ↓
2. Columnar Cache (if cached)
    ↓
3. Sync Delta Store to Cache
    ↓
4. Vectorized Execution (SIMD)
    ↓
5. Optional: GPU Acceleration
    ↓
Results
```

---

## Configuration

### Delta Store Settings

```rust
use dbx_core::{Database, Config};

fn main() -> dbx_core::DbxResult<()> {
    let config = Config::default()
        .delta_flush_threshold(10000)  // Flush after 10k records
        .delta_flush_interval(60);     // Or after 60 seconds
    
    let db = Database::open_with_config("./data", config)?;
    
    Ok(())
}
```

### Compaction Settings

```rust
let config = Config::default()
    .compaction_threshold(100_000)  // Compact after 100k records
    .compaction_interval(3600);     // Or after 1 hour

let db = Database::open_with_config("./data", config)?;
```

---

## Performance Tuning

### Hot Data in Delta Store

Keep frequently accessed data in memory:

```rust
// Hot data stays in Delta Store
for i in 0..1000 {
    let key = format!("hot:key:{}", i);
    db.insert("cache", key.as_bytes(), b"value")?;
}
// Don't flush immediately
```

### Batch Compaction

Compact multiple tables together:

```rust
db.compact("users")?;
db.compact("orders")?;
db.compact("products")?;
```

### Columnar Cache Warming

Pre-load cache for analytical queries:

```rust
// Register frequently queried tables
db.register_table("users", batches)?;
db.register_table("orders", order_batches)?;
```

---

## Monitoring

### Storage Statistics

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Get storage stats
    let stats = db.storage_stats("users")?;
    
    println!("Delta Store: {} records", stats.delta_count);
    println!("WOS: {} records", stats.wos_count);
    println!("ROS: {} files, {} MB", stats.ros_files, stats.ros_size_mb);
    
    Ok(())
}
```

---

## Best Practices

### 1. Flush Strategically

```rust
// Good: Flush after batch operations
for i in 0..10000 {
    db.insert("users", format!("user:{}", i).as_bytes(), b"data")?;
}
db.flush_delta("users")?;

// Avoid: Flush after every write
db.insert("users", b"user:1", b"data")?;
db.flush_delta("users")?; // Too frequent
```

### 2. Compact Regularly

```rust
// Schedule compaction during low-traffic periods
if is_low_traffic_time() {
    db.compact("users")?;
}
```

### 3. Use Columnar Cache for Analytics

```rust
// Register tables for SQL queries
db.register_table("users", user_batches)?;
db.register_table("orders", order_batches)?;

// Now SQL queries are fast
let results = db.execute_sql(
    "SELECT u.name, COUNT(o.id) 
     FROM users u 
     JOIN orders o ON u.id = o.user_id 
     GROUP BY u.name"
)?;
```

---

## Next Steps

- [CRUD Operations](crud-operations) — Basic database operations
- [Transactions](transactions) — MVCC transactions
- [SQL Reference](sql-reference) — Analytical queries
- [Performance Tuning](../operations/performance-tuning) — Optimize storage
