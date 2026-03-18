# DBX 미구현 기능 구현 계획

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Phase 0(HTAP 실시간 동기화, 적응형 워크로드), Phase 3(파티셔닝), Phase 4.3~4.4(Job 의존성/재시도), Phase 5.1(Views), Phase 5.3(Replication), Phase 5.4(Sharding) 구현

**Architecture:** 의존성이 없는 독립 기능부터 구현. Job 의존성→Views→파티셔닝→HTAP 동기화→Replication→Sharding 순. 각 기능은 TDD로 진행하며 기존 `automation/`, `storage/`, `sql/`, `engine/` 구조 내에 확장.

**Tech Stack:** Rust stable, Rayon, Apache Arrow, Tokio(async sync), DashMap

---

## 구현 순서 (의존성 기반)

```
Phase 4.3 Job 의존성  ←──────────────── 기존 scheduler 확장
Phase 4.4 재시도/모니터링 ←──────────── 기존 scheduler_thread 확장
Phase 5.1 Views ←───────────────────── 기존 sql/parser, sql/executor 확장
Phase 3   파티셔닝 ←─────────────────── 기존 storage/native_wos, sql/executor 확장
Phase 0   HTAP 실시간 동기화 ←────────── delta_store + columnar_cache 연결
Phase 0   적응형 워크로드 조정 ←──────── 위 동기화 구현 이후
Phase 5.3 Replication ←─────────────── WAL 기반 스트리밍 (가장 복잡)
Phase 5.4 Sharding ←────────────────── Replication 이후 (분산 계층 필요)
```

---

## Task 1: Phase 4.3 — Job 의존성 (DAG 스케줄러)

**목표**: 작업 A → 작업 B 순서 실행, A 완료 후 B 트리거

**Files:**
- Modify: `core/dbx-core/src/automation/schedule.rs`
- Create: `core/dbx-core/src/automation/job_dag.rs`
- Modify: `core/dbx-core/src/automation/schedule_executor.rs`

**Step 1: 실패 테스트 작성**

```rust
// core/dbx-core/src/automation/job_dag.rs (tests)
#[test]
fn test_job_dag_dependency_ordering() {
    let mut dag = JobDag::new();
    dag.add_job("backup", Schedule::Interval(Duration::from_secs(3600)));
    dag.add_job("cleanup", Schedule::After("backup".to_string()));
    dag.add_dependency("cleanup", "backup");

    let order = dag.resolve_execution_order().unwrap();
    assert_eq!(order[0], "backup");
    assert_eq!(order[1], "cleanup");
}

#[test]
fn test_job_dag_cycle_detection() {
    let mut dag = JobDag::new();
    dag.add_dependency("a", "b");
    dag.add_dependency("b", "a");
    assert!(dag.resolve_execution_order().is_err()); // 순환 감지
}
```

**Step 2: 테스트 실행 → FAIL 확인**

```bash
cargo test -p dbx-core -- job_dag --nocapture
```

**Step 3: 구현**

```rust
// core/dbx-core/src/automation/job_dag.rs
use std::collections::{HashMap, HashSet, VecDeque};
use crate::error::{DbxError, DbxResult};
use super::schedule::Schedule;

pub struct JobDag {
    jobs: HashMap<String, Schedule>,
    /// job → 이 job이 완료되어야 실행되는 job들
    dependents: HashMap<String, Vec<String>>,
    /// job → 이 job이 실행되려면 완료되어야 하는 job들
    dependencies: HashMap<String, Vec<String>>,
}

impl JobDag {
    pub fn new() -> Self { ... }
    pub fn add_job(&mut self, id: &str, schedule: Schedule) { ... }
    pub fn add_dependency(&mut self, job: &str, depends_on: &str) { ... }

    /// Kahn's algorithm으로 위상 정렬. 순환 시 Err 반환.
    pub fn resolve_execution_order(&self) -> DbxResult<Vec<String>> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for job in self.jobs.keys() { in_degree.insert(job, 0); }
        for deps in self.dependencies.values() {
            for d in deps { *in_degree.entry(d).or_insert(0) += 1; }
        }
        let mut queue: VecDeque<&str> = in_degree.iter()
            .filter(|(_, &v)| v == 0).map(|(&k, _)| k).collect();
        let mut order = Vec::new();
        while let Some(job) = queue.pop_front() {
            order.push(job.to_string());
            if let Some(deps) = self.dependents.get(job) {
                for d in deps {
                    let cnt = in_degree.get_mut(d.as_str()).unwrap();
                    *cnt -= 1;
                    if *cnt == 0 { queue.push_back(d); }
                }
            }
        }
        if order.len() != self.jobs.len() {
            return Err(DbxError::InvalidArguments("순환 의존성 감지".into()));
        }
        Ok(order)
    }
}
```

**Step 4: 테스트 PASS 확인**

```bash
cargo test -p dbx-core -- job_dag
```

**Step 5: Commit**

```bash
git add core/dbx-core/src/automation/job_dag.rs
git commit -m "feat(scheduler): add DAG-based job dependency resolution"
```

---

## Task 2: Phase 4.4 — 재시도 정책 + Job 히스토리

**목표**: 실패한 Job을 max_retries, exponential backoff으로 재시도. JobHistory로 이력 저장.

**Files:**
- Create: `core/dbx-core/src/automation/retry_policy.rs`
- Modify: `core/dbx-core/src/automation/schedule_executor.rs`

**Step 1: 실패 테스트**

```rust
#[test]
fn test_retry_executes_until_success() {
    let policy = RetryPolicy { max_retries: 3, backoff_ms: 10, exponential: false };
    let attempts = Arc::new(AtomicUsize::new(0));
    let a = attempts.clone();
    let result = policy.execute(move || {
        a.fetch_add(1, Ordering::SeqCst);
        if a.load(Ordering::SeqCst) < 3 { Err(DbxError::IoError("fail".into())) }
        else { Ok(()) }
    });
    assert!(result.is_ok());
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[test]
fn test_retry_fails_after_max() {
    let policy = RetryPolicy { max_retries: 2, backoff_ms: 0, exponential: false };
    let result = policy.execute::<_, ()>(|| Err(DbxError::IoError("always fail".into())));
    assert!(result.is_err());
}
```

**Step 2: 구현**

```rust
// core/dbx-core/src/automation/retry_policy.rs
pub struct RetryPolicy {
    pub max_retries: u32,
    pub backoff_ms: u64,
    pub exponential: bool,
}

pub struct JobHistoryEntry {
    pub job_id: String,
    pub started_at: std::time::SystemTime,
    pub completed_at: Option<std::time::SystemTime>,
    pub success: bool,
    pub error: Option<String>,
    pub attempts: u32,
}

impl RetryPolicy {
    pub fn execute<F, T>(&self, mut f: F) -> DbxResult<T>
    where F: FnMut() -> DbxResult<T>
    {
        let mut last_err = None;
        let mut delay = self.backoff_ms;
        for attempt in 0..=self.max_retries {
            match f() {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < self.max_retries {
                        std::thread::sleep(Duration::from_millis(delay));
                        if self.exponential { delay *= 2; }
                    }
                }
            }
        }
        Err(last_err.unwrap())
    }
}
```

**Step 3: 테스트 PASS, Commit**

```bash
cargo test -p dbx-core -- retry_policy
git commit -m "feat(scheduler): add RetryPolicy and JobHistory"
```

---

## Task 3: Phase 5.1 — SQL Views (뷰)

**목표**: `CREATE VIEW v AS SELECT ...` 등록, `SELECT * FROM v` 시 내부 SQL로 치환하여 실행

**Files:**
- Create: `core/dbx-core/src/sql/view.rs`
- Modify: `core/dbx-core/src/sql/parser/mod.rs` (CREATE/DROP VIEW 파싱)
- Modify: `core/dbx-core/src/sql/executor/mod.rs` (뷰 → 서브쿼리 확장)
- Modify: `core/dbx-core/src/engine/ddl_api.rs` (create_view, drop_view API)

**Step 1: 실패 테스트**

```rust
#[test]
fn test_create_and_query_view() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE users (id INT, name TEXT, active BOOL)").unwrap();
    db.insert("users", b"1", b"Alice:true").unwrap();
    
    // 뷰 생성
    db.execute_sql("CREATE VIEW active_users AS SELECT id, name FROM users WHERE active = true").unwrap();
    
    // 뷰 조회
    let result = db.execute_sql("SELECT * FROM active_users").unwrap();
    assert_eq!(result.num_rows(), 1);
}

#[test]
fn test_drop_view() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE VIEW v AS SELECT 1 AS x").unwrap();
    db.execute_sql("DROP VIEW v").unwrap();
    assert!(db.execute_sql("SELECT * FROM v").is_err());
}
```

**Step 2: 구현 설계**

뷰는 이름 → SQL 문자열 매핑을 `DashMap<String, String>`으로 저장.
SQL 실행 전 FROM 절에서 뷰 이름 발견 시 해당 SQL을 서브쿼리로 인라인 치환.

```rust
// core/dbx-core/src/sql/view.rs
use dashmap::DashMap;
use crate::error::DbxResult;

pub struct ViewRegistry {
    views: DashMap<String, String>, // view_name → sql_text
}

impl ViewRegistry {
    pub fn new() -> Self { Self { views: DashMap::new() } }

    pub fn create(&self, name: &str, sql: &str) -> DbxResult<()> {
        self.views.insert(name.to_lowercase(), sql.to_string());
        Ok(())
    }

    pub fn drop(&self, name: &str) -> DbxResult<()> {
        self.views.remove(&name.to_lowercase())
            .map(|_| ())
            .ok_or_else(|| crate::error::DbxError::InvalidArguments(
                format!("뷰 '{name}' 없음")
            ))
    }

    /// SQL의 FROM 절에서 뷰 이름을 서브쿼리로 치환
    pub fn expand(&self, sql: &str) -> String {
        let mut result = sql.to_string();
        for entry in self.views.iter() {
            let pattern = format!("FROM {}", entry.key());
            let replacement = format!("FROM ({}) AS {}", entry.value(), entry.key());
            result = result.replace(&pattern, &replacement);
        }
        result
    }

    pub fn exists(&self, name: &str) -> bool {
        self.views.contains_key(&name.to_lowercase())
    }
}
```

**Step 3: Database에 ViewRegistry 통합**

`src/engine/database.rs`에 `view_registry: Arc<ViewRegistry>` 필드 추가.
`execute_sql()` 호출 전 `view_registry.expand(sql)` 적용.

**Step 4: 테스트 PASS, Commit**

```bash
cargo test -p dbx-core -- view
git commit -m "feat(sql): add CREATE/DROP VIEW with SQL expansion"
```

---

## Task 4: Phase 3 — 파티셔닝

**목표**: Range/Hash/List 파티션 키 기반 물리적 분할. 쿼리 시 필요한 파티션만 스캔.

**Files:**
- Create: `core/dbx-core/src/storage/partition.rs`
- Modify: `core/dbx-core/src/engine/ddl_api.rs` (`create_partition`, `drop_partition`)
- Modify: `core/dbx-core/src/engine/crud.rs` (파티션 라우팅)
- Modify: `core/dbx-core/src/sql/executor/mod.rs` (Partition Pruning)

**핵심 설계**:

파티션된 테이블은 사실 N개의 sub-테이블 (`table__part_0`, `table__part_1`, ...)로 저장.
라우팅 함수가 key → sub-table 이름을 결정.

```rust
// core/dbx-core/src/storage/partition.rs
#[derive(Debug, Clone)]
pub enum PartitionType {
    Range {
        column: String,
        bounds: Vec<(i64, i64)>, // [low, high) 범위들
    },
    Hash {
        column: String,
        num_partitions: usize,
    },
    List {
        column: String,
        values: Vec<Vec<String>>, // 각 파티션의 값 목록
    },
}

pub struct PartitionMap {
    pub table: String,
    pub partition_type: PartitionType,
    pub num_partitions: usize,
}

impl PartitionMap {
    /// key 값을 받아 sub-table 이름 반환
    pub fn route_key(&self, key_value: &Value) -> String {
        let idx = match &self.partition_type {
            PartitionType::Hash { num_partitions, .. } => {
                let h = fnv1a_hash(key_value);
                h % num_partitions
            }
            PartitionType::Range { bounds, .. } => {
                let v = key_value.as_i64().unwrap_or(0);
                bounds.iter().position(|(lo, hi)| v >= *lo && v < *hi)
                    .unwrap_or(bounds.len() - 1)
            }
            PartitionType::List { values, .. } => {
                let s = key_value.to_string();
                values.iter().position(|group| group.contains(&s))
                    .unwrap_or(0)
            }
        };
        format!("{}__{}_part_{}", self.table, "p", idx)
    }

    /// WHERE 조건값으로 스캔할 파티션 목록 반환 (Pruning)
    pub fn pruned_partitions(&self, filter_value: Option<&Value>) -> Vec<String> {
        match filter_value {
            None => (0..self.num_partitions)
                .map(|i| format!("{}__{}_part_{}", self.table, "p", i))
                .collect(),
            Some(v) => vec![self.route_key(v)],
        }
    }
}
```

**Step 1: 실패 테스트**

```rust
#[test]
fn test_hash_partition_routing() {
    let map = PartitionMap {
        table: "users".into(),
        partition_type: PartitionType::Hash { column: "id".into(), num_partitions: 4 },
        num_partitions: 4,
    };
    let t0 = map.route_key(&Value::Int(0));
    let t3 = map.route_key(&Value::Int(3));
    assert!(t0.contains("part_"));
    assert_ne!(t0, t3); // 다른 파티션
}

#[test]
fn test_range_partition_pruning() {
    let map = PartitionMap {
        table: "logs".into(),
        partition_type: PartitionType::Range {
            column: "ts".into(),
            bounds: vec![(0, 1000), (1000, 2000), (2000, 3000)],
        },
        num_partitions: 3,
    };
    let partitions = map.pruned_partitions(Some(&Value::Int(1500)));
    assert_eq!(partitions.len(), 1); // 하나의 파티션만
}
```

**Step 2: Commit**

```bash
git commit -m "feat(storage): add PartitionMap with Range/Hash/List routing and Pruning"
```

---

## Task 5: Phase 0 — HTAP 실시간 동기화

**목표**: Delta Store에 insert 후 Columnar Cache로 즉시(또는 비동기 배치) 전파

**Files:**
- Create: `core/dbx-core/src/storage/realtime_sync.rs`
- Modify: `core/dbx-core/src/engine/crud.rs` (`insert()` 후 sync 호출)
- Modify: `core/dbx-core/src/engine/parallel_engine.rs` (`DbConfig`에 `sync: RealtimeSyncConfig` 추가)

**핵심 설계**:

```rust
// core/dbx-core/src/storage/realtime_sync.rs
#[derive(Debug, Clone)]
pub enum SyncMode {
    Immediate,                  // 모든 insert 후 즉시
    Threshold(usize),           // N행 쌓이면 배치 (기존 방식)
    AsyncBatch { max_latency_ms: u64 }, // 백그라운드 스레드
}

#[derive(Debug, Clone)]
pub struct RealtimeSyncConfig {
    pub mode: SyncMode,
    pub batch_size: usize,
}

impl Default for RealtimeSyncConfig {
    fn default() -> Self {
        Self { mode: SyncMode::Threshold(10_000), batch_size: 1000 }
    }
}
```

`insert()` 시:
- `Immediate` → insert 후 바로 `columnar_cache.append_batch()`
- `Threshold(n)` → delta 항목 수 >= n 이면 자동 flush (기존)
- `AsyncBatch` → 백그라운드 채널로 전송, 별도 스레드가 max_latency_ms 주기로 flush

**Step 1: 실패 테스트**

```rust
#[test]
fn test_immediate_sync_reflects_in_cache() {
    let db = Database::open_with_config(path, DbConfig {
        parallelism: ParallelismConfig::default(),
        sync: RealtimeSyncConfig { mode: SyncMode::Immediate, batch_size: 1 },
    }).unwrap();
    
    db.insert("users", b"k1", b"v1").unwrap();
    
    // insert 직후 columnar cache에서 조회 가능해야 함
    let cache_count = db.columnar_cache().row_count("users");
    assert!(cache_count >= 1);
}
```

**Step 2: Commit**

```bash
git commit -m "feat(htap): add RealtimeSyncConfig with Immediate/Threshold/AsyncBatch modes"
```

---

## Task 6: Phase 0 — 적응형 워크로드 조정

**목표**: OLTP/OLAP 비율 감지 → Delta 크기/Cache 크기/Compaction 주기 자동 조정

**Files:**
- Create: `core/dbx-core/src/engine/workload_analyzer.rs`
- Modify: `core/dbx-core/src/engine/database.rs` (백그라운드 분석 루프)

**핵심**: 최근 쿼리 N개 중 포인트 쿼리(OLTP) vs 풀 스캔/집계(OLAP) 비율 추적.

```rust
pub enum QueryPattern { PointQuery, RangeScan, Aggregation, Join }

pub struct WorkloadAnalyzer {
    window: VecDeque<QueryPattern>,
    window_size: usize,
}

impl WorkloadAnalyzer {
    pub fn record(&mut self, pattern: QueryPattern) {
        if self.window.len() >= self.window_size {
            self.window.pop_front();
        }
        self.window.push_back(pattern);
    }

    /// 0.0 = 순수 OLAP, 1.0 = 순수 OLTP
    pub fn oltp_ratio(&self) -> f64 {
        if self.window.is_empty() { return 0.5; }
        let oltp = self.window.iter()
            .filter(|p| matches!(p, QueryPattern::PointQuery)).count();
        oltp as f64 / self.window.len() as f64
    }

    pub fn recommended_config(&self) -> AdaptiveConfig {
        let ratio = self.oltp_ratio();
        match ratio {
            r if r > 0.7 => AdaptiveConfig::oltp_heavy(),
            r if r < 0.3 => AdaptiveConfig::olap_heavy(),
            _ => AdaptiveConfig::balanced(),
        }
    }
}
```

**Step 1: 실패 테스트 → 구현 → Commit**

```bash
git commit -m "feat(htap): add WorkloadAnalyzer with OLTP/OLAP ratio detection"
```

---

## Task 7: Phase 5.3 — Replication (Master-Slave)

**주의**: 가장 복잡한 기능. 별도 네트워크 레이어 필요. **MVP 범위**로 제한.

**MVP 범위**:
- WAL을 기반으로 변경사항을 직렬화
- Slave는 Master WAL을 소비하여 로컬 DB에 재생

**Files:**
- Create: `core/dbx-core/src/replication/mod.rs`
- Create: `core/dbx-core/src/replication/master.rs`
- Create: `core/dbx-core/src/replication/slave.rs`
- Create: `core/dbx-core/src/replication/protocol.rs`

**핵심 설계**:

```rust
// Replication 프로토콜
pub enum ReplicationMessage {
    WalEntry { lsn: u64, data: Vec<u8> },
    Heartbeat { lsn: u64 },
    RequestFrom { lsn: u64 },
}

// Master: WAL append 시 채널로 전송
pub struct ReplicationMaster {
    tx: broadcast::Sender<ReplicationMessage>,
    current_lsn: AtomicU64,
}

// Slave: TCP/채널에서 수신 → 로컬 WAL replay
pub struct ReplicationSlave {
    local_db: Arc<Database>,
    master_addr: String,
    last_applied_lsn: AtomicU64,
}
```

**단계**: TCP 구현 전 `tokio::sync::broadcast` 인메모리 채널로 먼저 테스트.

```bash
git commit -m "feat(replication): add WAL-based master-slave replication MVP"
```

---

## Task 8: Phase 5.4 — Sharding

**주의**: Replication 완료 후 구현. Sharding은 Task 4(파티셔닝)를 분산 환경으로 확장.

**MVP 범위**:
- 샤드 키 기반 라우팅 (Hash)
- 로컬 샤드 + 원격 샤드(gRPC/HTTP)
- 크로스 샤드 쿼리는 Scatter-Gather

**Files:**
- Create: `core/dbx-core/src/sharding/mod.rs`
- Create: `core/dbx-core/src/sharding/router.rs`
- Create: `core/dbx-core/src/sharding/scatter_gather.rs`

```rust
pub struct ShardRouter {
    shard_key: String,
    shards: Vec<ShardInfo>,
}

pub struct ShardInfo {
    id: usize,
    endpoint: String, // "localhost:7001" 등
    local: bool,      // 로컬 DB면 직접 호출
}

impl ShardRouter {
    pub fn route(&self, key_value: &[u8]) -> &ShardInfo {
        let idx = fnv1a_hash(key_value) % self.shards.len();
        &self.shards[idx]
    }
}
```

```bash
git commit -m "feat(sharding): add hash-based shard router"
```

---

## 전체 구현 순서 요약

| 순서 | Task | 예상 시간 | 난이도 | 의존성 |
|------|------|---------|--------|--------|
| 1 | Job 의존성 (DAG) | 1일 | ⭐⭐ | 없음 |
| 2 | 재시도 + Job 히스토리 | 0.5일 | ⭐ | 없음 |
| 3 | SQL Views | 2일 | ⭐⭐⭐ | SQL 파서 이해 |
| 4 | 파티셔닝 | 3일 | ⭐⭐⭐ | storage 이해 |
| 5 | HTAP 실시간 동기화 | 2일 | ⭐⭐⭐ | delta+cache 이해 |
| 6 | 적응형 워크로드 | 1일 | ⭐⭐ | Task 5 완료 후 |
| 7 | Replication | 5일 | ⭐⭐⭐⭐⭐ | WAL 이해 |
| 8 | Sharding | 5일 | ⭐⭐⭐⭐⭐ | Task 4, 7 완료 후 |

**총 예상 시간**: 약 3~4주 (병렬 작업 시 단축 가능)

---

## 빌드/테스트 명령어

```bash
# 전체 테스트
cargo test -p dbx-core

# 특정 모듈
cargo test -p dbx-core -- job_dag
cargo test -p dbx-core -- retry_policy
cargo test -p dbx-core -- view
cargo test -p dbx-core -- partition
cargo test -p dbx-core -- realtime_sync
cargo test -p dbx-core -- workload_analyzer
cargo test -p dbx-core -- replication
cargo test -p dbx-core -- sharding

# 빌드 확인
cargo build -p dbx-core
```
