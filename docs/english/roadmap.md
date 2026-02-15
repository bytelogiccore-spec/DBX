---
layout: default
title: Roadmap
nav_order: 50
parent: English
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

## ⚡ Phase 0: HTAP Optimization (Q1 2026)

**Goal**: Enhance DBX's HTAP (Hybrid Transactional/Analytical Processing) maturity

While DBX currently supports both OLTP and OLAP workloads through its 5-Tier architecture, it lacks real-time analytics and adaptive workload optimization. This phase transforms DBX into a true HTAP system.

### 0.1 Real-time Synchronization (4 weeks)

**Problem**:
- Delta Store → Columnar Cache synchronization is threshold-based, causing delays
- Latest data not immediately reflected in analytical queries
- Fails to meet HTAP's core requirement of "real-time analytics"

**Implementation**:
```rust
pub struct RealtimeSyncConfig {
    /// Synchronization mode
    mode: SyncMode,
    /// Batch size (in rows)
    batch_size: usize,
    /// Maximum latency (milliseconds)
    max_latency_ms: u64,
}

pub enum SyncMode {
    /// Immediate sync (after every write)
    Immediate,
    /// Async batch sync (default)
    AsyncBatch,
    /// Threshold-based (legacy)
    Threshold(usize),
}

impl DeltaStore {
    /// Real-time propagation to Columnar Cache on Delta changes
    pub async fn sync_to_cache(&self) -> DbxResult<()> {
        let changes = self.drain_pending_changes();
        self.columnar_cache.append_batch_async(changes).await?;
        Ok(())
    }
}
```

**Features**:
- **Async real-time sync**: Immediate Columnar Cache updates on Delta Store changes
- **Batch optimization**: Small changes batched to minimize overhead
- **Latency guarantee**: Sync completes within 100ms
- **Backpressure control**: Automatic throttling under Cache load

**Example Usage**:
```rust
// Enable real-time synchronization
db.enable_realtime_sync(RealtimeSyncConfig {
    mode: SyncMode::AsyncBatch,
    batch_size: 1000,
    max_latency_ms: 100,
})?;

// Now analytical queries see latest data immediately after INSERT
db.insert("users", user_data)?;
// Sync completes within 100ms
let result = db.execute_sql("SELECT COUNT(*) FROM users WHERE status = 'active'")?;
```

**Performance Targets**:
- Sync latency: < 100ms (99th percentile)
- Write overhead: < 5%
- Query freshness: Real-time (vs. seconds/minutes previously)

**Success Criteria**:
- TPC-H benchmark support for real-time analytical queries
- Maintain 95%+ write throughput
- Provide sync latency monitoring dashboard

---

### 0.2 Adaptive Workload Tuning (5 weeks)

**Problem**:
- Flush/Compaction thresholds are statically configured
- Same strategy applied to OLTP-heavy and OLAP-heavy workloads
- Resource waste and performance degradation

**Implementation**:
```rust
pub struct WorkloadAnalyzer {
    /// OLTP vs OLAP ratio (0.0 = pure OLAP, 1.0 = pure OLTP)
    oltp_ratio: f64,
    /// Hot key tracking
    hot_keys: LruCache<Vec<u8>, u64>,
    /// Query pattern history
    query_patterns: VecDeque<QueryPattern>,
    /// Analysis window (seconds)
    window_size: u64,
}

pub struct AdaptiveConfig {
    /// Delta Store size (dynamically adjusted)
    delta_threshold: usize,
    /// Columnar Cache size (dynamically adjusted)
    cache_size: usize,
    /// Compaction frequency (dynamically adjusted)
    compaction_interval: Duration,
    /// GPU usage (dynamically determined)
    enable_gpu: bool,
}

impl WorkloadAnalyzer {
    /// Analyze workload and auto-tune
    pub fn analyze_and_tune(&mut self, stats: &WorkloadStats) -> AdaptiveConfig {
        self.update_oltp_ratio(stats);
        
        if self.is_oltp_heavy() {
            // OLTP optimization: Expand Delta Store, shrink Cache
            AdaptiveConfig {
                delta_threshold: 100_000,  // 2x default
                cache_size: 50_000,        // 0.5x default
                compaction_interval: Duration::from_secs(300),
                enable_gpu: false,
            }
        } else if self.is_olap_heavy() {
            // OLAP optimization: Expand Cache, enable GPU
            AdaptiveConfig {
                delta_threshold: 10_000,   // Fast flush
                cache_size: 500_000,       // 5x default
                compaction_interval: Duration::from_secs(60),
                enable_gpu: true,
            }
        } else {
            // Balanced mode (default)
            AdaptiveConfig::default()
        }
    }
    
    fn is_oltp_heavy(&self) -> bool {
        self.oltp_ratio > 0.7
    }
    
    fn is_olap_heavy(&self) -> bool {
        self.oltp_ratio < 0.3
    }
}

pub enum QueryPattern {
    PointQuery,      // SELECT WHERE id = ?
    RangeScan,       // SELECT WHERE date BETWEEN ? AND ?
    Aggregation,     // SELECT SUM(amount) GROUP BY ...
    Join,            // SELECT ... FROM a JOIN b ...
}
```

**Features**:
- **Workload detection**: Real-time OLTP/OLAP ratio tracking
- **Auto-tuning**: Dynamic Tier sizing based on workload
- **Hot data tracking**: Keep frequently accessed keys in Delta Store
- **Predictive optimization**: Proactive adjustments based on historical patterns

**Example Usage**:
```rust
// Enable adaptive optimization
db.enable_adaptive_tuning(AdaptiveTuningConfig {
    analysis_window: Duration::from_secs(300),  // 5-minute window
    tuning_interval: Duration::from_secs(60),   // Re-tune every minute
    auto_gpu: true,  // Auto-enable GPU based on workload
})?;

// System automatically analyzes and optimizes
// OLTP-heavy → Expand Delta Store
// OLAP-heavy → Expand Columnar Cache, enable GPU
```

**Optimization Strategy**:

| Workload | OLTP Ratio | Delta Size | Cache Size | GPU | Compaction Interval |
|---------|----------|-----------|-----------|-----|----------------|
| **Pure OLTP** | > 90% | 200K | 10K | ❌ | 10 min |
| **OLTP-heavy** | 70-90% | 100K | 50K | ❌ | 5 min |
| **Balanced** | 30-70% | 50K | 100K | ⚠️ | 2 min |
| **OLAP-heavy** | 10-30% | 10K | 500K | ✅ | 1 min |
| **Pure OLAP** | < 10% | 5K | 1M | ✅ | 30 sec |

**Performance Targets**:
- OLTP workload: 20% write throughput improvement
- OLAP workload: 30% query response time reduction
- Mixed workload: 15% overall throughput improvement

**Success Criteria**:
- Auto-readjustment on workload shift (< 1 minute)
- Optimized resource utilization (< 10% memory waste)
- Average 20% performance gain vs. static configuration in benchmarks

---

### 0.3 HTAP Benchmark Suite (3 weeks)

**Goal**: Validate HTAP performance and prevent regressions

**Implementation**:
```rust
pub struct HtapBenchmark {
    /// Concurrent OLTP transactions
    oltp_threads: usize,
    /// Concurrent OLAP queries
    olap_threads: usize,
    /// Benchmark duration
    duration: Duration,
}
```

**Benchmark Scenarios**:
1. **CH-benCHmark**: TPC-C (OLTP) + TPC-H (OLAP) mixed
2. **Real-time analytics**: Measure aggregation query latency after INSERT
3. **Workload transition**: Measure adaptation time on OLTP → OLAP shift
4. **Isolation test**: Impact of OLAP queries on OLTP throughput

**Performance Criteria**:
- OLTP throughput: > 50,000 TPS (with concurrent OLAP)
- OLAP query latency: < 500ms (TPC-H Q1)
- Real-time analytics latency: < 100ms (99th percentile)
- Isolation overhead: < 10%

---

### 0.4 Monitoring and Observability (2 weeks)

**Implementation**:
```rust
pub struct HtapMetrics {
    /// Real-time sync latency
    sync_latency: Histogram,
    /// OLTP/OLAP ratio
    workload_ratio: Gauge,
    /// Tier hit rates
    tier_hit_rates: HashMap<String, f64>,
    /// Adaptive tuning events
    tuning_events: Vec<TuningEvent>,
}

// Export metrics
db.export_metrics("/metrics")?;  // Prometheus format
```

**Dashboard**:
- Real-time workload distribution (OLTP vs OLAP)
- Tier-level data distribution and hit rates
- Sync latency histogram
- Adaptive tuning history

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
