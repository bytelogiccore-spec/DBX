# 구체화된 뷰 & 스트리밍 수집 구현 계획

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**목표:** 로드맵에 남아있는 두 기능 구현 — (1) 구체화된 뷰(사전 계산된 쿼리 캐시, 자동 갱신) (2) 스트리밍 수집(채널 기반 실시간 데이터 파이프라인)

**아키텍처:**

- **구체화된 뷰**: 기존 `sql/view.rs`의 `ViewRegistry` 를 기반으로 `MaterializedViewRegistry`를 추가. 쿼리 정의와 Arrow `RecordBatch` 캐시를 함께 저장. 갱신 트리거: (a) `REFRESH MATERIALIZED VIEW` SQL, (b) `automation/` 스케줄러를 통한 Cron 자동 갱신.

- **스트리밍 수집**: `engine/stream_ingester.rs` 에 `StreamIngester` 구조체 추가. `StreamEvent` enum으로 INSERT / UPDATE / DELETE를 모두 표현하며, `std::sync::mpsc` 채널로 이벤트를 받아 백그라운드 스레드가 `batch_size` 또는 `max_latency` 조건 충족 시 각 DML 연산을 실행. CDC(Change Data Capture) 패턴을 완전 지원.

**기술 스택:** `dashmap`, `parking_lot::RwLock`, `std::sync::mpsc`, `arrow::RecordBatch`, 기존 `automation/scheduler`, `ViewRegistry`, `Database::execute_sql`, `Database::insert` / `Database::delete` / `Database::execute_sql(UPDATE ...)`.

---

## Feature 1 — 구체화된 뷰 (Materialized Views)

### Task 1: `MaterializedViewRegistry` 데이터 구조 추가

**파일:**
- 수정: `core/dbx-core/src/sql/view.rs`

**Step 1: 실패하는 테스트 먼저 작성**

```rust
#[test]
fn test_materialized_view_cache() {
    use arrow::array::Int64Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    let reg = MaterializedViewRegistry::new();
    let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int64, false)]));
    let batch = RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(vec![1, 2, 3]))]).unwrap();

    reg.create("mv_users", "SELECT id FROM users", None).unwrap();
    assert!(!reg.is_fresh("mv_users")); // 생성 직후에는 stale

    reg.set_cache("mv_users", vec![batch.clone()]).unwrap();
    assert!(reg.is_fresh("mv_users"));

    let cached = reg.get_cache("mv_users").unwrap();
    assert_eq!(cached[0].num_rows(), 3);
}
```

**Step 2: 테스트 실패 확인**

```bash
cargo test -p dbx-core test_materialized_view_cache -- --nocapture
```
예상 결과: FAIL — `MaterializedViewRegistry` not defined

**Step 3: 구현 (`sql/view.rs` 하단에 추가)**

```rust
use std::time::Instant;
use parking_lot::RwLock;

#[derive(Debug)]
struct MatViewEntry {
    sql: String,
    cache: Option<Vec<RecordBatch>>,
    refreshed_at: Option<Instant>,
    refresh_interval_secs: Option<u64>,
}

#[derive(Debug, Default)]
pub struct MaterializedViewRegistry {
    views: DashMap<String, RwLock<MatViewEntry>>,
}

impl MaterializedViewRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn create(&self, name: &str, sql: &str, refresh_interval_secs: Option<u64>) -> DbxResult<()> {
        self.views.insert(name.to_lowercase(), RwLock::new(MatViewEntry {
            sql: sql.to_string(), cache: None,
            refreshed_at: None, refresh_interval_secs,
        }));
        Ok(())
    }

    pub fn set_cache(&self, name: &str, batches: Vec<RecordBatch>) -> DbxResult<()> {
        let entry = self.views.get(&name.to_lowercase())
            .ok_or_else(|| DbxError::InvalidArguments(format!("'{}' 뷰를 찾을 수 없음", name)))?;
        let mut e = entry.write();
        e.cache = Some(batches);
        e.refreshed_at = Some(Instant::now());
        Ok(())
    }

    pub fn is_fresh(&self, name: &str) -> bool {
        let entry = match self.views.get(&name.to_lowercase()) { Some(e) => e, None => return false };
        let e = entry.read();
        match (e.refreshed_at, e.refresh_interval_secs) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(t), Some(secs)) => t.elapsed().as_secs() < secs,
        }
    }

    pub fn get_cache(&self, name: &str) -> Option<Vec<RecordBatch>> {
        Some(self.views.get(&name.to_lowercase())?.read().cache.clone()?)
    }

    pub fn get_sql(&self, name: &str) -> Option<String> {
        Some(self.views.get(&name.to_lowercase())?.read().sql.clone())
    }

    pub fn list(&self) -> Vec<String> {
        self.views.iter().map(|e| e.key().clone()).collect()
    }

    pub fn drop(&self, name: &str) -> DbxResult<()> {
        self.views.remove(&name.to_lowercase()).map(|_| ())
            .ok_or_else(|| DbxError::InvalidArguments(format!("'{}' 뷰를 찾을 수 없음", name)))
    }
}

pub type SharedMaterializedViewRegistry = Arc<MaterializedViewRegistry>;
```

**Step 4: 테스트 통과 확인**

```bash
cargo test -p dbx-core test_materialized_view_cache
```

**Step 5: 커밋**

```bash
git add core/dbx-core/src/sql/view.rs
git commit -m "[기능]: MaterializedViewRegistry 데이터 구조 추가"
```

---

### Task 2: `Database`에 SQL 명령어 연동

**파일:**
- 수정: `core/dbx-core/src/engine/database.rs`
- 수정: `core/dbx-core/src/engine/constructors.rs`
- 수정: `core/dbx-core/src/sql/interface.rs`

**Step 1: 테스트**

```rust
#[test]
fn test_db_create_materialized_view() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE MATERIALIZED VIEW mv_test AS SELECT 1 AS x").unwrap();
    assert!(db.mat_view_registry.get_sql("mv_test").is_some());
}
```

**Step 2: `database.rs` 필드 추가**

```rust
pub mat_view_registry: SharedMaterializedViewRegistry,
```

**Step 3: `constructors.rs` 초기화**

```rust
mat_view_registry: Arc::new(MaterializedViewRegistry::new()),
```

**Step 4: `interface.rs`의 `execute_sql()` 분기 추가**

```rust
// Step 0 상단에 추가
if sql_upper.starts_with("CREATE MATERIALIZED VIEW") {
    return self.handle_create_materialized_view(sql_trimmed);
}
if sql_upper.starts_with("DROP MATERIALIZED VIEW") {
    return self.handle_drop_materialized_view(sql_trimmed);
}
if sql_upper.starts_with("REFRESH MATERIALIZED VIEW") {
    return self.handle_refresh_materialized_view(sql_trimmed);
}
```

**Step 5: 핸들러 구현**

```rust
fn handle_create_materialized_view(&self, sql: &str) -> DbxResult<Vec<RecordBatch>> {
    let upper = sql.to_uppercase();
    let after = &sql[upper.find("VIEW").unwrap() + 4..].trim_start().to_owned();
    let as_pos = after.to_uppercase().find(" AS ")
        .ok_or_else(|| DbxError::SqlParse { message: "CREATE MATERIALIZED VIEW requires AS".into(), sql: sql.to_string() })?;
    let name = after[..as_pos].trim();
    let view_sql = after[as_pos + 4..].trim();
    self.mat_view_registry.create(name, view_sql, None)?;
    self.one_row_affected_batch()
}

fn handle_refresh_materialized_view(&self, sql: &str) -> DbxResult<Vec<RecordBatch>> {
    let upper = sql.to_uppercase();
    let name = sql[upper.find("VIEW").unwrap() + 4..].trim().to_lowercase();
    let view_sql = self.mat_view_registry.get_sql(&name)
        .ok_or_else(|| DbxError::InvalidArguments(format!("'{}' 뷰를 찾을 수 없음", name)))?;
    let result = self.execute_sql(&view_sql)?;
    self.mat_view_registry.set_cache(&name, result)?;
    self.one_row_affected_batch()
}

fn handle_drop_materialized_view(&self, sql: &str) -> DbxResult<Vec<RecordBatch>> {
    let upper = sql.to_uppercase();
    let name = sql[upper.find("VIEW").unwrap() + 4..].trim();
    self.mat_view_registry.drop(name)?;
    self.one_row_affected_batch()
}
```

**Step 6: SELECT 캐시 히트 로직 (`execute_sql()` 최상단)**

```rust
if let Some(cached) = self.try_matview_cache(sql_trimmed) {
    return Ok(cached);
}

fn try_matview_cache(&self, sql: &str) -> Option<Vec<RecordBatch>> {
    let upper = sql.to_uppercase();
    let from_pos = upper.find(" FROM ")?;
    let name = sql[from_pos + 6..].trim().split_whitespace().next()?;
    if self.mat_view_registry.is_fresh(name) { self.mat_view_registry.get_cache(name) } else { None }
}
```

**Step 7: 테스트 + 커밋**

```bash
cargo test -p dbx-core -- mat
git add core/dbx-core/src/
git commit -m "[기능]: CREATE/DROP/REFRESH MATERIALIZED VIEW SQL 명령어 및 캐시 히트 추가"
```

---

### Task 3: 자동 갱신 백그라운드 스레드 연동

**파일:**
- 수정: `core/dbx-core/src/engine/constructors.rs`

**Step 1: 테스트**

```rust
#[test]
fn test_matview_manual_refresh_updates_cache() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE t (id INT, v INT)").unwrap();
    db.execute_sql("INSERT INTO t VALUES (1, 100)").unwrap();
    db.execute_sql("CREATE MATERIALIZED VIEW mv_t AS SELECT * FROM t").unwrap();
    db.execute_sql("REFRESH MATERIALIZED VIEW mv_t").unwrap();
    let c1 = db.mat_view_registry.get_cache("mv_t").unwrap();
    assert_eq!(c1[0].num_rows(), 1);

    db.execute_sql("INSERT INTO t VALUES (2, 200)").unwrap();
    db.execute_sql("REFRESH MATERIALIZED VIEW mv_t").unwrap();
    let c2 = db.mat_view_registry.get_cache("mv_t").unwrap();
    assert_eq!(c2[0].num_rows(), 2);
}
```

**Step 2: 자동 갱신 백그라운드 스레드 (`constructors.rs`)**

```rust
// Database 초기화 후 Arc<Database>를 만든 직후 실행
let mv_reg = Arc::clone(&db.mat_view_registry);
let db_weak = Arc::downgrade(&db_arc);
std::thread::spawn(move || {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
        for name in mv_reg.list() {
            if !mv_reg.is_fresh(&name) {
                if let Some(db) = db_weak.upgrade() {
                    let _ = db.execute_sql(&format!("REFRESH MATERIALIZED VIEW {}", name));
                }
            }
        }
    }
});
```

**Step 3: `REFRESH EVERY <n>` 파싱 지원 (선택)**

```sql
-- 300초마다 자동 갱신
CREATE MATERIALIZED VIEW mv_name REFRESH EVERY 300 AS SELECT ...
```

`handle_create_materialized_view`에서 `REFRESH EVERY <n>` 토큰 감지 후 `refresh_interval_secs: Some(n)` 전달

**Step 4: 테스트 + 커밋**

```bash
cargo test -p dbx-core -- matview
git add core/dbx-core/src/
git commit -m "[기능]: Materialized View 자동 갱신 백그라운드 스레드 연동"
```

---

## Feature 2 — 스트리밍 수집 (Streaming Ingestion)

### Task 4: `StreamEvent` 및 `StreamIngester` 구조체 구현

> **설계 원칙**: 실제 스트리밍 파이프라인은 INSERT만 처리하지 않습니다.
> - **CDC(Change Data Capture)**: DB 변경 이벤트(INSERT/UPDATE/DELETE)를 실시간 구독
> - **이벤트 소싱(Event Sourcing)**: 상태 변경을 이벤트 스트림으로 표현
> - **Kafka/Kinesis 연동**: 메시지 형태로 DELETE나 UPDATE 이벤트가 전달됨
>
> 따라서 `StreamEvent` enum으로 모든 DML 연산을 표현합니다.

**파일:**
- 생성: `core/dbx-core/src/engine/stream_ingester.rs`
- 수정: `core/dbx-core/src/engine/mod.rs`

**Step 1: 테스트 (INSERT / UPDATE / DELETE 모두 검증)**

```rust
#[test]
fn test_stream_ingester_insert_update_delete() {
    use std::time::Duration;
    use std::sync::Arc;

    let db = Arc::new(Database::open_in_memory().unwrap());
    db.execute_sql("CREATE TABLE orders (id INT, status TEXT)").unwrap();

    let ingester = StreamIngester::new(Arc::clone(&db), "orders", 100, Duration::from_millis(50));
    let tx = ingester.sender();

    // INSERT 이벤트 2건
    tx.send(vec![
        StreamEvent::Insert { key: "1".into(), value: b"[1, \"pending\"]".to_vec() },
        StreamEvent::Insert { key: "2".into(), value: b"[2, \"pending\"]".to_vec() },
    ]).unwrap();

    // UPDATE 이벤트: key=1의 status를 "shipped"으로
    tx.send(vec![
        StreamEvent::Update { key: "1".into(), value: b"[1, \"shipped\"]".to_vec() },
    ]).unwrap();

    // DELETE 이벤트: key=2 삭제
    tx.send(vec![
        StreamEvent::Delete { key: "2".into() },
    ]).unwrap();

    ingester.flush().unwrap();

    let rows = db.scan("orders").unwrap();
    assert_eq!(rows.len(), 1); // key=2 삭제됐으므로 1건만 남아야 함
}
```

**Step 2: 테스트 실패 확인**

```bash
cargo test -p dbx-core test_stream_ingester_insert_update_delete
```
예상: FAIL — `StreamEvent` / `StreamIngester` not defined

**Step 3: 구현 (`engine/stream_ingester.rs` 신규 생성)**

```rust
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::thread;
use crate::engine::Database;
use crate::error::DbxResult;

/// 스트림 이벤트 — INSERT / UPDATE / DELETE를 통합 표현
///
/// CDC (Change Data Capture) 및 이벤트 소싱 패턴에서
/// 스트림으로 전달되는 모든 DML 연산을 지원합니다.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// 새 레코드 삽입
    Insert {
        key: String,
        value: Vec<u8>,
    },
    /// 기존 레코드 갱신 (키 기반)
    Update {
        key: String,
        value: Vec<u8>,
    },
    /// 레코드 삭제 (키 기반)
    Delete {
        key: String,
    },
}

pub struct StreamIngester {
    sender: mpsc::SyncSender<Vec<StreamEvent>>,
    _handle: thread::JoinHandle<()>,
}

impl StreamIngester {
    /// 스트림 인제스터 생성
    ///
    /// - `table`: 대상 테이블 이름
    /// - `batch_size`: 이 이벤트 수에 도달하면 즉시 flush
    /// - `max_latency`: 버퍼가 차지 않아도 이 시간마다 강제 flush
    pub fn new(db: Arc<Database>, table: &str, batch_size: usize, max_latency: Duration) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Vec<StreamEvent>>(batch_size * 4);
        let table = table.to_string();

        let handle = thread::spawn(move || {
            let mut buffer: Vec<StreamEvent> = Vec::with_capacity(batch_size);
            let mut deadline = std::time::Instant::now() + max_latency;

            loop {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                match rx.recv_timeout(remaining) {
                    Ok(events) => buffer.extend(events),
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        // 채널 닫힘: 남은 버퍼 모두 flush 후 종료
                        Self::flush_buffer(&db, &table, &mut buffer);
                        break;
                    }
                }

                if buffer.len() >= batch_size || std::time::Instant::now() >= deadline {
                    Self::flush_buffer(&db, &table, &mut buffer);
                    deadline = std::time::Instant::now() + max_latency;
                }
            }
        });

        Self { sender: tx, _handle: handle }
    }

    /// 버퍼의 이벤트를 DB에 적용 (DML 종류에 따라 insert / delete / update)
    fn flush_buffer(db: &Database, table: &str, buffer: &mut Vec<StreamEvent>) {
        for event in buffer.drain(..) {
            match event {
                StreamEvent::Insert { key, value } => {
                    let _ = db.insert(table, key.as_bytes(), &value);
                }
                StreamEvent::Update { key, value } => {
                    // UPDATE = DELETE 후 INSERT (덮어쓰기 시맨틱)
                    // dbx의 insert는 key 기반이므로 그냥 재삽입하면 됨
                    let _ = db.insert(table, key.as_bytes(), &value);
                }
                StreamEvent::Delete { key } => {
                    let _ = db.delete(table, key.as_bytes());
                }
            }
        }
    }

    /// 이벤트 배치 송신용 sender 클론
    pub fn sender(&self) -> mpsc::SyncSender<Vec<StreamEvent>> {
        self.sender.clone()
    }

    /// 채널을 닫아 백그라운드 스레드가 남은 이벤트 flush 후 종료하도록 함
    pub fn flush(self) -> DbxResult<()> {
        drop(self.sender); // 채널 닫기 → 백그라운드 스레드 Disconnected 감지 → 남은 이벤트 flush → 종료
        self._handle.join().ok();
        Ok(())
    }
}
```

**Step 4: `mod.rs`에 모듈 노출**

```rust
pub mod stream_ingester;
pub use stream_ingester::{StreamIngester, StreamEvent};
```

**Step 5: 테스트 통과 + 커밋**

```bash
cargo test -p dbx-core test_stream_ingester_insert_update_delete
git add core/dbx-core/src/engine/stream_ingester.rs core/dbx-core/src/engine/mod.rs
git commit -m "[기능]: StreamIngester — INSERT/UPDATE/DELETE 이벤트를 지원하는 CDC 스타일 수집 파이프라인 추가"
```

---

### Task 5: `Database` 공개 API 연동 및 통합 테스트

**파일:**
- 수정: `core/dbx-core/src/engine/database.rs`

**Step 1: 편의 메소드 추가**

```rust
impl Database {
    /// 채널 기반 스트리밍 수집 파이프라인 생성
    ///
    /// INSERT / UPDATE / DELETE 이벤트를 `StreamEvent`로 전송하면
    /// 백그라운드에서 자동 배치 처리합니다.
    ///
    /// # 예시
    /// ```rust
    /// let ingester = db.create_stream_ingester("orders", 1000, 100);
    /// let tx = ingester.sender();
    /// tx.send(vec![StreamEvent::Insert { key: "1".into(), value: ... }]).unwrap();
    /// ingester.flush().unwrap();
    /// ```
    pub fn create_stream_ingester(
        self: &Arc<Self>,
        table: &str,
        batch_size: usize,
        max_latency_ms: u64,
    ) -> StreamIngester {
        StreamIngester::new(Arc::clone(self), table, batch_size, Duration::from_millis(max_latency_ms))
    }
}
```

**Step 2: 혼합 DML 고처리량 통합 테스트**

```rust
#[test]
fn test_mixed_dml_stream() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    db.execute_sql("CREATE TABLE inventory (id INT, qty INT)").unwrap();

    let ingester = db.create_stream_ingester("inventory", 500, 100);
    let tx = ingester.sender();

    // 1,000건 INSERT
    for i in 0..1000_u32 {
        tx.send(vec![
            StreamEvent::Insert { key: format!("{}", i), value: format!("[{}, 10]", i).into_bytes() },
        ]).unwrap();
    }

    // 짝수 key UPDATE (qty → 99)
    for i in (0..1000_u32).step_by(2) {
        tx.send(vec![
            StreamEvent::Update { key: format!("{}", i), value: format!("[{}, 99]", i).into_bytes() },
        ]).unwrap();
    }

    // 100의 배수 key DELETE
    for i in (0..1000_u32).step_by(100) {
        tx.send(vec![
            StreamEvent::Delete { key: format!("{}", i) },
        ]).unwrap();
    }

    ingester.flush().unwrap();

    let rows = db.scan("inventory").unwrap();
    // 1000 - 10 (100의 배수 삭제) = 990건
    assert_eq!(rows.len(), 990);
}
```

**Step 3: 테스트 + 커밋**

```bash
cargo test -p dbx-core -- stream
git add core/dbx-core/src/
git commit -m "[기능]: create_stream_ingester 공개 API 및 혼합 DML 통합 테스트 추가"
```

---

### Task 6: 문서 및 로드맵 업데이트

**파일:**
- 수정: `README.md`
- 수정: `docs/korean/README.md`

**Step 1: 두 README 로드맵에서 두 항목을 ✅로 이동**

```diff
-### 로드맵 🚧
-- **구체화된 뷰** — 사전 계산된 쿼리 결과 및 자동 갱신
-- **스트리밍 수집** — 실시간 데이터 파이프라인 지원

+### 핵심 기능 ✅ (추가)
+- ✅ **구체화된 뷰** — CREATE/DROP/REFRESH MATERIALIZED VIEW, 자동 갱신 스케줄러
+- ✅ **스트리밍 수집** — StreamIngester, CDC 스타일 INSERT/UPDATE/DELETE 이벤트 파이프라인
```

**Step 2: 커밋**

```bash
git add README.md docs/korean/README.md
git commit -m "[문서]: Materialized Views & Streaming Ingestion 구현 완료 반영"
```

---

## 난이도 요약

| 기능 | 난이도 | 예상 시간 |
|------|--------|----------|
| `MaterializedViewRegistry` 구조체 | ⭐⭐ | 2~3시간 |
| SQL 명령어 연동 (CREATE/DROP/REFRESH) | ⭐⭐⭐ | 2시간 |
| 자동 갱신 백그라운드 스레드 | ⭐⭐⭐ | 1.5시간 |
| `StreamEvent` enum + `StreamIngester` | ⭐⭐ | 2시간 |
| **합계** | | **약 8~10시간** |

> **팁:** `StreamEvent::Update`는 DBX의 key 기반 insert가 덮어쓰기(upsert) 시맨틱을 가지므로 별도 UPDATE 로직 없이 `db.insert()`를 재활용할 수 있습니다. 단, 명시적 UPDATE SQL을 실행해야 하는 경우라면 `db.execute_sql("UPDATE ... WHERE key = ...")` 패턴으로 전환하세요.
