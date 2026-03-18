---
layout: default
title: Roadmap
nav_order: 50
parent: 한국어
---

# DBX Roadmap

DBX의 미래 발전 방향과 계획된 기능들입니다.

---

## 🎯 비전

**DBX를 완전한 엔터프라이즈급 임베디드 데이터베이스로 발전시킵니다.**

현재 DBX는 고성능 CRUD, SQL, 트랜잭션, GPU 가속을 제공하지만, 엔터프라이즈 환경에서 필요한 고급 기능들이 부족합니다. 이 로드맵은 DBX를 PostgreSQL, MySQL과 같은 수준의 완전한 데이터베이스로 만들기 위한 계획입니다.

---

## 📊 현재 상태 (v0.1.0)

### ✅ 구현 완료

- **핵심 기능**
  - 5-Tier Hybrid Storage (Delta → Cache → WOS → Index → ROS)
  - CRUD Operations (Insert, Get, Delete, Count)
  - MVCC Transactions (Snapshot Isolation)
  - SQL Support (SELECT, WHERE, JOIN, GROUP BY, ORDER BY)
  - WAL (Write-Ahead Log) — native SSTable + sequential append 방식

- **성능 최적화**
  - GPU Acceleration (CUDA-based aggregation, filtering, joins)
  - Bloom Filter Indexing
  - LRU Cache
  - SIMD Vectorization (`wide` crate, stable Rust, 항상 활성)
  - **멀티코어 병렬화** (Rayon 기반, v0.1.1 전면 적용)
    - `insert_batch()` par_iter (1,000행 임계값)
    - `scan()` rayon::join Delta+WOS 동시 스캔
    - GROUP BY 집계 par_iter (1,000 그룹 임계값)
    - JOIN Build/Probe Phase par_iter (1,000행 임계값)
    - `get_batches()` projection par_iter
    - WAL encode par_iter (500건 임계값)
    - `compact()` 역직렬화 par_iter (4페이지 임계값)
    - `ParallelismConfig` — cpu_cap / min_rows_for_parallel 런타임 제어
  
- **데이터 보호**
  - Encryption (AES-256-GCM-SIV, ChaCha20-Poly1305)
  - Compression (ZSTD)

- **언어 바인딩**
  - Python, C#/.NET, C/C++, Node.js

### ❌ 미구현 기능

- Partitioning (파티션)
- User-Defined Functions (UDF)
- Job Scheduler (스케줄러)
- Triggers (트리거)
- Views (뷰)
- Stored Procedures (저장 프로시저)
- Replication (복제)
- Sharding (샤딩)

> ⚠️ **위 목록은 v0.0.6 기준입니다.** 이후 v0.4, v0.1.1에서 여러 항목이 구현 완료되었습니다. 아래 각 Phase 제목의 ✅/❌를 참고하세요.

---

## ⚡ Phase 0: HTAP 최적화 (Q1 2026)

**목표**: DBX의 HTAP(Hybrid Transactional/Analytical Processing) 완성도 향상

현재 DBX는 5-Tier 아키텍처로 OLTP와 OLAP을 모두 지원하지만, 실시간 분석과 워크로드 적응형 최적화가 부족합니다. 이 단계에서는 진정한 HTAP 시스템으로 발전시킵니다.

### 0.1 실시간 동기화 (4주)

**문제점**:
- Delta Store → Columnar Cache 동기화가 임계값 기반으로 지연 발생
- 최신 데이터가 분석 쿼리에 즉시 반영되지 않음
- HTAP의 핵심 요건인 "실시간 분석" 미충족

**구현 내용**:
```rust
pub struct RealtimeSyncConfig {
    /// 동기화 모드
    mode: SyncMode,
    /// 배치 크기 (행 단위)
    batch_size: usize,
    /// 최대 지연 시간 (밀리초)
    max_latency_ms: u64,
}

pub enum SyncMode {
    /// 즉시 동기화 (모든 쓰기 후)
    Immediate,
    /// 비동기 배치 동기화 (기본값)
    AsyncBatch,
    /// 임계값 기반 (기존 방식)
    Threshold(usize),
}

impl DeltaStore {
    /// Delta 변경 시 Columnar Cache로 실시간 전파
    pub async fn sync_to_cache(&self) -> DbxResult<()> {
        let changes = self.drain_pending_changes();
        self.columnar_cache.append_batch_async(changes).await?;
        Ok(())
    }
}
```

**기능**:
- **비동기 실시간 동기화**: Delta Store 변경 시 즉시 Columnar Cache 업데이트
- **배치 최적화**: 작은 변경은 배치로 묶어 오버헤드 최소화
- **지연 시간 보장**: 최대 100ms 이내 동기화 완료
- **백프레셔 제어**: Cache 부하 시 자동 조절

**사용 예시**:
```rust
// 실시간 동기화 활성화
db.enable_realtime_sync(RealtimeSyncConfig {
    mode: SyncMode::AsyncBatch,
    batch_size: 1000,
    max_latency_ms: 100,
})?;

// 이제 INSERT 직후 분석 쿼리가 최신 데이터를 봄
db.insert("users", user_data)?;
// 100ms 이내에 동기화 완료
let result = db.execute_sql("SELECT COUNT(*) FROM users WHERE status = 'active'")?;
```

**성능 목표**:
- 동기화 지연: < 100ms (99 percentile)
- 쓰기 오버헤드: < 5%
- 분석 쿼리 신선도: 실시간 (기존: 수 초~분)

**성공 기준**:
- TPC-H 벤치마크에서 실시간 분석 쿼리 지원
- 쓰기 처리량 95% 이상 유지
- 동기화 지연 시간 모니터링 대시보드 제공

---

### 0.2 적응형 워크로드 조정 (5주)

**문제점**:
- Flush/Compaction 임계값이 정적 설정
- OLTP 중심 워크로드와 OLAP 중심 워크로드에 동일한 전략 적용
- 리소스 낭비 및 성능 저하

**구현 내용**:
```rust
pub struct WorkloadAnalyzer {
    /// OLTP vs OLAP 비율 (0.0 = 순수 OLAP, 1.0 = 순수 OLTP)
    oltp_ratio: f64,
    /// 핫 키 추적
    hot_keys: LruCache<Vec<u8>, u64>,
    /// 쿼리 패턴 히스토리
    query_patterns: VecDeque<QueryPattern>,
    /// 분석 윈도우 (초)
    window_size: u64,
}

pub struct AdaptiveConfig {
    /// Delta Store 크기 (동적 조정)
    delta_threshold: usize,
    /// Columnar Cache 크기 (동적 조정)
    cache_size: usize,
    /// Compaction 빈도 (동적 조정)
    compaction_interval: Duration,
    /// GPU 사용 여부 (동적 결정)
    enable_gpu: bool,
}

impl WorkloadAnalyzer {
    /// 워크로드 분석 및 자동 튜닝
    pub fn analyze_and_tune(&mut self, stats: &WorkloadStats) -> AdaptiveConfig {
        self.update_oltp_ratio(stats);
        
        if self.is_oltp_heavy() {
            // OLTP 최적화: Delta Store 확대, Cache 축소
            AdaptiveConfig {
                delta_threshold: 100_000,  // 기본값의 2배
                cache_size: 50_000,        // 기본값의 0.5배
                compaction_interval: Duration::from_secs(300),
                enable_gpu: false,
            }
        } else if self.is_olap_heavy() {
            // OLAP 최적화: Cache 확대, GPU 활성화
            AdaptiveConfig {
                delta_threshold: 10_000,   // 빠른 Flush
                cache_size: 500_000,       // 기본값의 5배
                compaction_interval: Duration::from_secs(60),
                enable_gpu: true,
            }
        } else {
            // 균형 모드 (기본값)
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

**기능**:
- **워크로드 감지**: OLTP/OLAP 비율 실시간 추적
- **자동 튜닝**: 워크로드에 따라 Tier 크기 동적 조정
- **핫 데이터 추적**: 자주 접근하는 키를 Delta Store에 유지
- **예측 기반 최적화**: 과거 패턴 기반 선제적 조정

**사용 예시**:
```rust
// 적응형 최적화 활성화
db.enable_adaptive_tuning(AdaptiveTuningConfig {
    analysis_window: Duration::from_secs(300),  // 5분 윈도우
    tuning_interval: Duration::from_secs(60),   // 1분마다 재조정
    auto_gpu: true,  // 워크로드에 따라 GPU 자동 활성화
})?;

// 시스템이 자동으로 워크로드 분석 및 최적화
// OLTP 중심 → Delta Store 확대
// OLAP 중심 → Columnar Cache 확대, GPU 활성화
```

**최적화 전략**:

| 워크로드 | OLTP 비율 | Delta 크기 | Cache 크기 | GPU | Compaction 주기 |
|---------|----------|-----------|-----------|-----|----------------|
| **순수 OLTP** | > 90% | 200K | 10K | ❌ | 10분 |
| **OLTP 중심** | 70-90% | 100K | 50K | ❌ | 5분 |
| **균형** | 30-70% | 50K | 100K | ⚠️ | 2분 |
| **OLAP 중심** | 10-30% | 10K | 500K | ✅ | 1분 |
| **순수 OLAP** | < 10% | 5K | 1M | ✅ | 30초 |

**성능 목표**:
- OLTP 워크로드: 쓰기 처리량 20% 향상
- OLAP 워크로드: 쿼리 응답 시간 30% 단축
- 혼합 워크로드: 전체 처리량 15% 향상

**성공 기준**:
- 워크로드 전환 시 자동 재조정 (< 1분)
- 리소스 사용률 최적화 (메모리 낭비 < 10%)
- 벤치마크에서 정적 설정 대비 평균 20% 성능 향상

---

### 0.3 HTAP 벤치마크 스위트 (3주)

**목표**: HTAP 성능 검증 및 회귀 방지

**구현 내용**:
```rust
pub struct HtapBenchmark {
    /// 동시 OLTP 트랜잭션 수
    oltp_threads: usize,
    /// 동시 OLAP 쿼리 수
    olap_threads: usize,
    /// 벤치마크 지속 시간
    duration: Duration,
}
```

**벤치마크 시나리오**:
1. **CH-benCHmark**: TPC-C (OLTP) + TPC-H (OLAP) 혼합
2. **실시간 분석**: INSERT 직후 집계 쿼리 지연 시간 측정
3. **워크로드 전환**: OLTP → OLAP 전환 시 적응 시간 측정
4. **격리 테스트**: OLAP 쿼리가 OLTP 처리량에 미치는 영향

**성능 기준**:
- OLTP 처리량: > 50,000 TPS (OLAP 동시 실행 시)
- OLAP 쿼리 지연: < 500ms (TPC-H Q1)
- 실시간 분석 지연: < 100ms (99 percentile)
- 격리 오버헤드: < 10%

---

### 0.4 모니터링 및 관측성 (2주)

**구현 내용**:
```rust
pub struct HtapMetrics {
    /// 실시간 동기화 지연
    sync_latency: Histogram,
    /// OLTP/OLAP 비율
    workload_ratio: Gauge,
    /// Tier별 히트율
    tier_hit_rates: HashMap<String, f64>,
    /// 적응형 조정 이벤트
    tuning_events: Vec<TuningEvent>,
}

// 메트릭 노출
db.export_metrics("/metrics")?;  // Prometheus 형식
```

**대시보드**:
- 실시간 워크로드 분포 (OLTP vs OLAP)
- Tier별 데이터 분포 및 히트율
- 동기화 지연 시간 히스토그램
- 적응형 조정 히스토리

---

## 🚀 Phase 1: 트리거 시스템 — ✅ **구현 완료** (v0.0.4-beta)

**목표**: 데이터 변경 시 자동 반응 시스템 구축

### 1.1 기본 트리거 (4주)

**구현 내용**:
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

**기능**:
- BEFORE/AFTER INSERT/UPDATE/DELETE 트리거
- 단일 테이블 트리거
- 트리거 활성화/비활성화

**사용 예시**:
```rust
db.create_trigger("audit_log", TriggerEvent::AfterInsert("users"), |_, new| {
    db.execute_sql("INSERT INTO audit_log VALUES (?, NOW())", &[new])?;
    Ok(())
})?;
```

**성공 기준**:
- 100,000 TPS 이상 유지 (트리거 활성화 시)
- 트리거 오버헤드 < 10%
- 모든 CRUD 작업에서 트리거 정상 작동

> ✅ **구현 확인**: `src/automation/trigger.rs`, `trigger_parser.rs`, `trigger_executor.rs` 존재, 424개 테스트 중 `automation::trigger` 모듈 테스트 통과 확인.

---

### 1.2 조건부 트리거 (2주)

**구현 내용**:
```rust
pub struct Trigger {
    condition: Option<Box<dyn Fn(&Row) -> bool>>,
    // ...
}
```

**기능**:
- WHERE 조건 지원
- 복잡한 조건식 (AND, OR, NOT)

**사용 예시**:
```rust
db.create_trigger_with_condition("log_vip",
    TriggerEvent::AfterInsert("users"),
    |row| row.get("tier")? == "VIP",
    |_, new| { /* ... */ }
)?;
```

---

### 1.3 고급 트리거 (4주)

**구현 내용**:
- INSTEAD OF 트리거 (뷰 업데이트)
- 트리거 체인 (트리거가 다른 트리거 발동)
- 트리거 우선순위
- 재귀 트리거 방지

**사용 예시**:
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

**성공 기준**:
- 트리거 체인 깊이 10 이상 지원
- 재귀 감지 및 방지
- 트리거 실행 순서 보장

---

## 🔧 Phase 2: User-Defined Functions — ✅ **구현 완료** (v0.0.4-beta)

**목표**: SQL 확장성 제공

### 2.1 Scalar UDF (4주)

**구현 내용**:
```rust
pub trait ScalarUDF: Send + Sync {
    fn call(&self, args: &[Value]) -> DbxResult<Value>;
    fn return_type(&self) -> DataType;
    fn arg_types(&self) -> Vec<DataType>;
}
```

**기능**:
- 단일 값 반환 함수
- 타입 검증
- 인라인 최적화

**사용 예시**:
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

**성공 기준**:
- UDF 호출 오버헤드 < 5%
- 타입 안전성 보장
- 1,000개 이상 UDF 등록 가능

---

### 2.2 Aggregate UDF (4주)

**구현 내용**:
```rust
pub trait AggregateUDF: Send + Sync {
    fn init(&mut self);
    fn update(&mut self, value: &Value);
    fn merge(&mut self, other: &Self);
    fn finalize(&self) -> Value;
}
```

**기능**:
- 집계 함수 (SUM, AVG, COUNT 등)
- 병렬 집계 (merge 지원)
- 윈도우 함수 지원

**사용 예시**:
```rust
db.register_aggregate_udf("median", MedianAggregator::new())?;
db.execute_sql("SELECT median(price) FROM products GROUP BY category")?;
```

---

### 2.3 Table UDF (3주)

> ✅ **구현 완료**: `src/automation/udf/` — scalar, aggregate, table UDF 전부 존재. `automation::udf::scalar`, `automation::udf::aggregate`, `automation::udf::table` 테스트 통과 확인.

**구현 내용**:
```rust
pub trait TableUDF: Send + Sync {
    fn call(&self, args: &[Value]) -> DbxResult<RecordBatch>;
}
```

**기능**:
- 테이블 반환 함수
- FROM 절에서 사용
- 동적 테이블 생성

**사용 예시**:
```rust
db.register_table_udf("generate_series", |start: i64, end: i64| {
    // start부터 end까지 숫자 생성
})?;

db.execute_sql("SELECT * FROM generate_series(1, 100)")?;
```

---

### 2.4 벡터화 UDF (3주)

**구현 내용**:
- 배치 처리 (한 번에 여러 행 처리)
- SIMD 최적화
- GPU UDF (CUDA 커널)

**성능 목표**:
- 벡터화 UDF: 10배 빠름
- GPU UDF: 100배 빠름 (대용량 데이터)

---

## 📦 Phase 3: 파티셔닝 (Q4 2026)

**목표**: 대용량 데이터 처리 및 쿼리 성능 향상

### 3.1 Range Partitioning (6주)

**구현 내용**:
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

**기능**:
- 날짜/시간 범위 파티션
- 숫자 범위 파티션
- 자동 파티션 프루닝 (Partition Pruning)

**사용 예시**:
```rust
db.create_partition("logs", PartitionType::Range {
    column: "created_at",
    ranges: vec![
        ("2024-01-01", "2024-02-01"),
        ("2024-02-01", "2024-03-01"),
        ("2024-03-01", "2024-04-01"),
    ]
})?;

// 쿼리 시 자동으로 필요한 파티션만 스캔
db.execute_sql("SELECT * FROM logs WHERE created_at >= '2024-02-15'")?;
// → 2024-02, 2024-03 파티션만 스캔 (10배 빠름!)
```

**성능 목표**:
- 파티션 프루닝으로 쿼리 시간 10-100배 단축
- 파티션별 병렬 쿼리 지원

---

### 3.2 Hash Partitioning (4주)

**구현 내용**:
```rust
pub enum PartitionType {
    Hash {
        column: String,
        num_partitions: usize,
    },
}
```

**기능**:
- 균등 분산
- 부하 분산
- 병렬 처리 최적화

**사용 예시**:
```rust
db.create_partition("users", PartitionType::Hash {
    column: "user_id",
    num_partitions: 10,
})?;

// 10개 파티션에 균등 분산
// 병렬 쿼리로 10배 빠름
```

---

### 3.3 List Partitioning (3주)

**구현 내용**:
```rust
pub enum PartitionType {
    List {
        column: String,
        values: Vec<Vec<Value>>,
    },
}
```

**기능**:
- 카테고리별 분할
- 지역별 분할

**사용 예시**:
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

### 3.4 자동 파티션 관리 (4주)

**구현 내용**:
- 자동 파티션 생성 (시계열 데이터)
- 자동 파티션 삭제 (오래된 데이터)
- 파티션 리밸런싱
- 파티션 병합/분할

**사용 예시**:
```rust
db.enable_auto_partition("logs", AutoPartitionConfig {
    type: PartitionType::Range { column: "created_at", interval: "1 month" },
    retention: Duration::from_days(180),  // 6개월 보관
    auto_create: true,
    auto_drop: true,
})?;

// 매달 자동으로 새 파티션 생성
// 6개월 지난 파티션 자동 삭제
```

---

## ⏰ Phase 4: Job Scheduler — ✅ **부분 구현 완료** (v0.0.4-beta)

**목표**: 자동화 작업 실행

### 4.1 기본 스케줄러 (4주)

**구현 내용**:
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

**기능**:
- 시간 기반 스케줄
- 작업 등록/삭제/실행
- 작업 활성화/비활성화

**사용 예시**:
```rust
db.schedule_job("cleanup", Schedule::Daily(2, 0), || {
    db.execute_sql("DELETE FROM temp WHERE created_at < NOW() - 7 DAYS")?;
    Ok(())
})?;
```

---

### 4.2 Cron 지원 — ✅ **구현 완료**

> ✅ **구현 확인**: `src/automation/schedule_parser.rs`에서 Cron 표현식 파싱 지원. `automation::schedule_parser` doctests 통과 확인.

**구현 내용**:
```rust
pub enum Schedule {
    Cron(String),  // "0 3 * * *"
}
```

**기능**:
- Cron 표현식 파싱
- 복잡한 스케줄 지원

**사용 예시**:
```rust
db.schedule_job("backup", Schedule::Cron("0 3 * * *"), || {
    db.backup("./backups/daily.tar.gz")?;
    Ok(())
})?;
```

---

### 4.3 작업 의존성 (3주)

**구현 내용**:
```rust
pub struct JobDependency {
    depends_on: Vec<String>,
    wait_for_completion: bool,
}
```

**기능**:
- 작업 간 의존성
- 순차 실행
- 병렬 실행

**사용 예시**:
```rust
db.schedule_job_with_deps("cleanup",
    Schedule::After("backup"),
    vec!["backup"],
    || { /* ... */ }
)?;
```

---

### 4.4 재시도 및 모니터링 (3주)

**구현 내용**:
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

**기능**:
- 실패 시 재시도
- 작업 히스토리
- 알림 (이메일, Slack 등)

---

## 🔮 Phase 5: 고급 기능 (Q2-Q4 2027)

### 5.1 Views (뷰)

**구현 내용**:
```rust
db.create_view("active_users", "
    SELECT * FROM users WHERE status = 'active'
")?;

db.execute_sql("SELECT * FROM active_users")?;
```

**기능**:
- Materialized Views (물리적 뷰)
- View 자동 갱신
- INSTEAD OF 트리거와 통합

---

### 5.2 Stored Procedures (저장 프로시저) — ✅ **구현 완료** (v0.0.4-beta)

> ✅ **구현 확인**: `src/automation/procedure.rs`, `procedure_parser.rs`, `procedure_executor.rs` 존재. `automation::procedure`, `automation::procedure_executor` 테스트 통과 확인.

**구현 내용**:
```rust
db.create_procedure("calculate_total", |order_id: i64| -> f64 {
    let items = db.execute_sql("SELECT price FROM order_items WHERE order_id = ?", &[order_id])?;
    items.iter().map(|r| r.get("price")).sum()
})?;

db.call_procedure("calculate_total", &[Value::Int(123)])?;
```

---

### 5.3 Replication (복제)

**구현 내용**:
- Master-Slave 복제
- Multi-Master 복제
- 자동 Failover

**사용 예시**:
```rust
db.enable_replication(ReplicationConfig {
    mode: ReplicationMode::MasterSlave,
    replicas: vec!["replica1:5432", "replica2:5432"],
    sync: true,
})?;
```

---

### 5.4 Sharding (샤딩)

**구현 내용**:
- 수평 샤딩
- 샤드 키 기반 라우팅
- 크로스 샤드 쿼리

**사용 예시**:
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

## 📈 성능 목표

| 기능 | 현재 | 목표 (Phase 5 완료 후) |
|------|------|----------------------|
| **단일 쿼리 TPS** | 100,000 | 100,000 (유지) |
| **범위 쿼리 (파티션)** | O(n) | O(n/p) (10-100배 빠름) |
| **UDF 오버헤드** | - | < 5% |
| **트리거 오버헤드** | - | < 10% |
| **병렬 쿼리** | 부분 지원 (JOIN, Sort, Columnar) | 완전 지원 (모든 연산) |
| **최대 데이터 크기** | 100GB | 10TB+ |

---

## 🎯 마일스톤

```
[완료] v0.0.4-beta: Phase 1 트리거 / Phase 2 UDF / Phase 4 스케줄러(Cron) / Phase 5 저장 프로시저
[완료] v0.1.1-beta: 멀티코어 병렬화 (P1~P9), WAL sequential append, wide SIMD

미완료:
2026 Q1: Phase 0 HTAP 최적화 (실시간 동기화, 적응형 워크로드)
2026 Q4: Phase 3 파티셔닝 (Range / Hash / List)
2027 Q1: Phase 4.3 작업 의존성, 4.4 재시도 모니터링
2027 Q2-Q4: Phase 5.1 Views / 5.3 Replication / 5.4 Sharding

→ DBX v1.0 목표: 2027 Q4
```

---

## 🤝 기여 방법

DBX는 오픈소스 프로젝트입니다. 기여를 환영합니다!

### 우선순위 높은 작업

1. **트리거 시스템 구현**
2. **UDF 프레임워크 설계**
3. **파티셔닝 알고리즘 최적화**
4. **스케줄러 Cron 파서**

### 기여 가이드

1. GitHub Issues에서 작업 선택
2. Fork & Pull Request
3. 테스트 작성 (커버리지 80% 이상)
4. 문서 업데이트

---

## 📝 라이선스

MIT OR Apache-2.0
