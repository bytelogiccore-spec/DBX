---
layout: default
title: Roadmap
nav_order: 10
---

# DBX Roadmap

Future direction and planned features for DBX.

---

## 🎯 Vision

**Evolve DBX into a full-scale enterprise-grade embedded database.**

While DBX already provides high-performance CRUD, SQL, transactions, and GPU acceleration, it lacks advanced features required for enterprise environments. This roadmap outlines our plan to elevate DBX to a level comparable with established databases like PostgreSQL or MySQL.

---

## 📊 Current Status (v0.1.0)

### ✅ Completed Features

- **Core Functionality**
  - 5-Tier Hybrid Storage (Delta → Cache → WOS → Index → ROS)
  - CRUD Operations (Insert, Get, Delete, Count)
  - MVCC Transactions (Snapshot Isolation)
  - SQL Support (SELECT, WHERE, JOIN, GROUP BY, ORDER BY)
  
- **Performance Optimization**
  - GPU Acceleration (CUDA-based aggregation, filtering, joins)
  - Bloom Filter Indexing
  - LRU Cache
  - SIMD Vectorization
  - **Parallel Query** (Rayon-based)
    - Parallelized JOIN operations (Build/Probe Phase)
    - Parallelized Sort operations
    - Parallelized Columnar Store building
    - Threshold: Automatic parallelization for 1,000+ rows
  
- **Data Protection**
  - Encryption (AES-256-GCM-SIV, ChaCha20-Poly1305)
  - Compression (ZSTD)
  - WAL (Write-Ahead Logging)
  
- **Language Bindings**
  - Python, C#/.NET, C/C++, Node.js

### ❌ Planned (Unimplemented) Features

- Partitioning
- User-Defined Functions (UDF)
- Job Scheduler
- Triggers
- Views
- Stored Procedures
- Replication
- Sharding

---

## 🚀 Phase 1: Trigger System (Q2 2026)

**Goal**: Establish an automated reaction system for data changes.

### 1.1 Basic Triggers (4 weeks)

**Implementation Details**:
```rust
pub enum TriggerEvent {
    BeforeInsert(String),
    AfterInsert(String),
    BeforeUpdate(String),
    AfterUpdate(String),
    BeforeDelete(String),
    AfterDelete(String),
}

pub struct Trigger {
    name: String,
    event: TriggerEvent,
    action: Box<dyn Fn(&Row, &Row) -> DbxResult<()>>,
    enabled: bool,
}
```

**Features**:
- BEFORE/AFTER INSERT/UPDATE/DELETE triggers
- Single table triggers
- Trigger activation/deactivation

**Example Usage**:
```rust
db.create_trigger("audit_log", TriggerEvent::AfterInsert("users"), |_, new| {
    db.execute_sql("INSERT INTO audit_log VALUES (?, NOW())", &[new])?;
    Ok(())
})?;
```

**Success Criteria**:
- Maintain 100,000+ TPS (with triggers active)
- Trigger overhead < 10%
- Reliable operation in all CRUD tasks

---

### 1.2 Conditional Triggers (2 weeks)

**Implementation Details**:
```rust
pub struct Trigger {
    condition: Option<Box<dyn Fn(&Row) -> bool>>,
    // ...
}
```

**Features**:
- WHERE condition support
- Complex logical expressions (AND, OR, NOT)

**Example Usage**:
```rust
db.create_trigger_with_condition("log_vip",
    TriggerEvent::AfterInsert("users"),
    |row| row.get("tier")? == "VIP",
    |_, new| { /* ... */ }
)?;
```

---

### 1.3 Advanced Triggers (4 weeks)

**Implementation Details**:
- INSTEAD OF triggers (view updates)
- Trigger chaining (multi-stage triggers)
- Trigger priority settings
- Recursion prevention

**Example Usage**:
```rust
db.create_trigger("cascade_update",
    TriggerEvent::AfterUpdate("orders"),
    |old, new| {
        if old.get("status")? != new.get("status")? {
            db.execute_sql("UPDATE order_items SET status = ? WHERE order_id = ?",
                &[new.get("status")?, new.get("id")?])?;
        }
        Ok(())
    }
)?;
```

**Success Criteria**:
- Support trigger chain depth of 10+
- Recursive call detection and prevention
- Guaranteed execution order

---

## 🔧 Phase 2: User-Defined Functions (Q3 2026)

**Goal**: Provide SQL extensibility.

### 2.1 Scalar UDF (4 weeks)

**Implementation Details**:
```rust
pub trait ScalarUDF: Send + Sync {
    fn call(&self, args: &[Value]) -> DbxResult<Value>;
    fn return_type(&self) -> DataType;
    fn arg_types(&self) -> Vec<DataType>;
}
```

**Features**:
- Single-value return functions
- Type validation
- Inline optimization

**Example Usage**:
```rust
db.register_udf("calculate_discount", |price: f64, tier: &str| -> f64 {
    match tier {
        "gold" => price * 0.8,
        "silver" => price * 0.9,
        _ => price,
    }
})?;

db.execute_sql("SELECT calculate_discount(price, tier) FROM products")?;
```

**Success Criteria**:
- UDF call overhead < 5%
- Guaranteed type safety
- Support for 1,000+ registered UDFs

---

### 2.2 Aggregate UDF (4 weeks)

**Implementation Details**:
```rust
pub trait AggregateUDF: Send + Sync {
    fn init(&mut self);
    fn update(&mut self, value: &Value);
    fn merge(&mut self, other: &Self);
    fn finalize(&self) -> Value;
}
```

**Features**:
- Aggregate functions (SUM, AVG, COUNT, etc.)
- Parallel aggregation support (via merge)
- Window function integration

**Example Usage**:
```rust
db.register_aggregate_udf("median", MedianAggregator::new())?;
db.execute_sql("SELECT median(price) FROM products GROUP BY category")?;
```

---

### 2.3 Table UDF (3 weeks)

**Implementation Details**:
```rust
pub trait TableUDF: Send + Sync {
    fn call(&self, args: &[Value]) -> DbxResult<RecordBatch>;
}
```

**Features**:
- Table-valued functions
- Usage in FROM clauses
- Dynamic table generation

**Example Usage**:
```rust
db.register_table_udf("generate_series", |start: i64, end: i64| {
    // Generate numbers from start to end
})?;

db.execute_sql("SELECT * FROM generate_series(1, 100)")?;
```

---

### 2.4 Vectorized UDF (3 weeks)

**Implementation Details**:
- Batch processing (multiple rows at once)
- SIMD optimization
- GPU UDF (CUDA kernels)

**Performance Targets**:
- Vectorized UDF: 10x faster
- GPU UDF: 100x faster (on large datasets)

---

## 📦 Phase 3: Partitioning (Q4 2026)

**Goal**: Improve handling of large datasets and query performance.

### 3.1 Range Partitioning (6 weeks)

**Implementation Details**:
```rust
pub enum PartitionType {
    Range {
        column: String,
        ranges: Vec<(Value, Value)>,
    },
}

pub struct PartitionedTable {
    partitions: Vec<Partition>,
    partition_key: String,
    partition_type: PartitionType,
}
```

**Features**:
- Date/Time range partitioning
- Numeric range partitioning
- Automatic Partition Pruning

**Example Usage**:
```rust
db.create_partition("logs", PartitionType::Range {
    column: "created_at",
    ranges: vec![
        ("2024-01-01", "2024-02-01"),
        ("2024-02-01", "2024-03-01"),
        ("2024-03-01", "2024-04-01"),
    ]
})?;

// Automatically scans only required partitions
db.execute_sql("SELECT * FROM logs WHERE created_at >= '2024-02-15'")?;
// → Scans only Feb and March partitions (10x faster!)
```

**Performance Targets**:
- 10-100x reduction in query time via pruning
- Support for per-partition parallel queries

---

### 3.2 Hash Partitioning (4 weeks)

**Implementation Details**:
```rust
pub enum PartitionType {
    Hash {
        column: String,
        num_partitions: usize,
    },
}
```

**Features**:
- Uniform distribution
- Load balancing
- Parallel processing optimization

**Example Usage**:
```rust
db.create_partition("users", PartitionType::Hash {
    column: "user_id",
    num_partitions: 10,
})?;

// Evenly distributed across 10 partitions
// 10x faster via parallel queries
```

---

### 3.3 List Partitioning (3 weeks)

**Implementation Details**:
```rust
pub enum PartitionType {
    List {
        column: String,
        values: Vec<Vec<Value>>,
    },
}
```

**Features**:
- Category-based partitioning
- Regional partitioning

**Example Usage**:
```rust
db.create_partition("users", PartitionType::List {
    column: "region",
    values: vec![
        vec!["KR", "JP"],  // Asia
        vec!["US", "CA"],  // America
        vec!["UK", "DE"],  // Europe
    ]
})?;
```

---

### 3.4 Automatic Partition Management (4 weeks)

**Implementation Details**:
- Auto-creation for time-series data
- Auto-dropping of old data
- Partition rebalancing
- Partition merging and splitting

**Example Usage**:
```rust
db.enable_auto_partition("logs", AutoPartitionConfig {
    type: PartitionType::Range { column: "created_at", interval: "1 month" },
    retention: Duration::from_days(180),  // Keep 6 months
    auto_create: true,
    auto_drop: true,
})?;

// New partition created automatically every month
// Partitions older than 6 months dropped automatically
```

---

## ⏰ Phase 4: Job Scheduler (Q1 2027)

**Goal**: Automate routine database tasks.

### 4.1 Basic Scheduler (4 weeks)

**Implementation Details**:
```rust
pub enum Schedule {
    Once(DateTime<Utc>),
    Interval(Duration),
    Hourly,
    Daily(u8, u8),
    Weekly(Weekday, u8, u8),
    Monthly(u8, u8, u8),
}

pub struct Job {
    id: String,
    schedule: Schedule,
    task: Box<dyn Fn() -> DbxResult<()> + Send + Sync>,
    enabled: bool,
    last_run: Option<DateTime<Utc>>,
    next_run: DateTime<Utc>,
}
```

**Features**:
- Time-based scheduling
- Job registration, deletion, and manual execution
- Job activation/deactivation

**Example Usage**:
```rust
db.schedule_job("cleanup", Schedule::Daily(2, 0), || {
    db.execute_sql("DELETE FROM temp WHERE created_at < NOW() - 7 DAYS")?;
    Ok(())
})?;
```

---

### 4.2 Cron Support (2 weeks)

**Implementation Details**:
```rust
pub enum Schedule {
    Cron(String),  // e.g., "0 3 * * *"
}
```

**Features**:
- Cron expression parsing
- Support for complex recurring schedules

**Example Usage**:
```rust
db.schedule_job("backup", Schedule::Cron("0 3 * * *"), || {
    db.backup("./backups/daily.tar.gz")?;
    Ok(())
})?;
```

---

### 4.3 Job Dependencies (3 weeks)

**Implementation Details**:
```rust
pub struct JobDependency {
    depends_on: Vec<String>,
    wait_for_completion: bool,
}
```

**Features**:
- Inter-job dependencies
- Sequential execution
- Parallel execution where appropriate

**Example Usage**:
```rust
db.schedule_job_with_deps("cleanup",
    Schedule::After("backup"),
    vec!["backup"],
    || { /* ... */ }
)?;
```

---

### 4.4 Retries and Monitoring (3 weeks)

**Implementation Details**:
```rust
pub struct RetryPolicy {
    max_retries: u32,
    backoff: Duration,
    exponential: bool,
}

pub struct JobHistory {
    job_id: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    status: JobStatus,
    error: Option<String>,
}
```

**Features**:
- Automatic retries on failure
- Job history logging
- Notifications (Email, Slack, etc.)

---

## 🔮 Phase 5: Advanced Features (Q2-Q4 2027)

### 5.1 Views

**Implementation Details**:
```rust
db.create_view("active_users", "
    SELECT * FROM users WHERE status = 'active'
")?;

db.execute_sql("SELECT * FROM active_users")?;
```

**Features**:
- Materialized Views
- Automatic view updates
- Integration with INSTEAD OF triggers

---

### 5.2 Stored Procedures

**Implementation Details**:
```rust
db.create_procedure("calculate_total", |order_id: i64| -> f64 {
    let items = db.execute_sql("SELECT price FROM order_items WHERE order_id = ?", &[order_id])?;
    items.iter().map(|r| r.get("price")).sum()
})?;

db.call_procedure("calculate_total", &[Value::Int(123)])?;
```

---

### 5.3 Replication

**Implementation Details**:
- Master-Slave replication
- Multi-Master replication
- Automatic Failover

**Example Usage**:
```rust
db.enable_replication(ReplicationConfig {
    mode: ReplicationMode::MasterSlave,
    replicas: vec!["replica1:5432", "replica2:5432"],
    sync: true,
})?;
```

---

### 5.4 Sharding

**Implementation Details**:
- Horizontal sharding
- Shard key based routing
- Cross-shard query support

**Example Usage**:
```rust
db.enable_sharding(ShardingConfig {
    shard_key: "user_id",
    num_shards: 10,
    shards: vec![
        "shard1:5432",
        "shard2:5432",
        // ...
    ],
})?;
```

---

## 📈 Performance Goals

| Feature | Current | Goal (Post Phase 5) |
|------|------|----------------------|
| **Single Query TPS** | 100,000 | 100,000 (Maintain) |
| **Range Query (Partition)** | O(n) | O(n/p) (10-100x faster) |
| **UDF Overhead** | - | < 5% |
| **Trigger Overhead** | - | < 10% |
| **Parallel Query** | Partial (JOIN, Sort, Columnar) | Full (All operations) |
| **Max Data Size** | 100GB | 10TB+ |

---

## 🎯 Milestones

```
2026 Q2: Phase 1 (Triggers) Completed
2026 Q3: Phase 2 (UDF) Completed
2026 Q4: Phase 3 (Partitioning) Completed
2027 Q1: Phase 4 (Scheduler) Completed
2027 Q2-Q4: Phase 5 (Advanced Features) Completed

→ DBX v1.0 Release (2027 Q4)
```

---

## 🤝 Contribution Guidelines

DBX is an open-source project. Contributions are welcome!

### High-Priority Tasks

1. **Trigger System Implementation**
2. **UDF Framework Design**
3. **Partitioning Algorithm Optimization**
4. **Cron Scheduler Parser**

### How to Contribute

1. Select a task from GitHub Issues
2. Fork & Submit a Pull Request
3. Write tests (maintain 80%+ coverage)
4. Update documentation

---

## 📝 License

MIT OR Apache-2.0
