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
  
- **성능 최적화**
  - GPU Acceleration (CUDA-based aggregation, filtering, joins)
  - Bloom Filter Indexing
  - LRU Cache
  - SIMD Vectorization
  - **병렬 쿼리** (Rayon 기반)
    - JOIN 연산 병렬화 (Build/Probe Phase)
    - Sort 연산 병렬화
    - Columnar Store 병렬 빌드
    - 임계값: 1000행 이상 시 자동 병렬화
  
- **데이터 보호**
  - Encryption (AES-256-GCM-SIV, ChaCha20-Poly1305)
  - Compression (ZSTD)
  - WAL (Write-Ahead Logging)
  
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

---

## 🚀 Phase 1: 트리거 시스템 (Q2 2026)

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

## 🔧 Phase 2: User-Defined Functions (Q3 2026)

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

## ⏰ Phase 4: Job Scheduler (Q1 2027)

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

### 4.2 Cron 지원 (2주)

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

### 5.2 Stored Procedures (저장 프로시저)

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
2026 Q2: Phase 1 (트리거) 완료
2026 Q3: Phase 2 (UDF) 완료
2026 Q4: Phase 3 (파티셔닝) 완료
2027 Q1: Phase 4 (스케줄러) 완료
2027 Q2-Q4: Phase 5 (고급 기능) 완료

→ DBX v1.0 릴리스 (2027 Q4)
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
