---
layout: default
title: Wire Protocol & PITR & Placement Rules Implementation Plan
parent: Plans
---

# Tier B Features Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date**: 2026-03-23
**Target**: `dbx-core` v0.3.0 (Tier A 완료 후)
**Priority**: 🥈 Tier B — 경쟁력 강화

**Goal:** PostgreSQL Wire Protocol, Point-in-Time Recovery, Placement Rules 3가지를 구현하여 DBX의 프로덕션 투입 가능성과 개발자 채택률을 높인다.

**Architecture:** Wire Protocol은 별도 `dbx-server` crate로 분리. PITR은 기존 WAL에 타임스탬프 인덱스를 추가. Placement Rules은 Grid Engine 파티션 메타데이터 확장.

---

## Feature A: PostgreSQL Wire Protocol

### 개요

DBeaver, DataGrip, psql 등 기존 PostgreSQL 도구로 DBX에 접속 가능하게 한다.

```
┌──────────────┐    PG Wire Protocol   ┌───────────┐
│ DBeaver      │ ──────────────────────►│dbx-server │
│ DataGrip     │                        │  (thin)   │
│ psql         │                        │     │     │
│ SQLAlchemy   │                        │ dbx-core  │
└──────────────┘                        └───────────┘
```

### Task A-1: `dbx-server` crate 생성

**Files:**
- Create: `core/dbx-server/Cargo.toml`
- Create: `core/dbx-server/src/main.rs`

**Cargo.toml:**
```toml
[package]
name = "dbx-server"
version = "0.1.0"
edition = "2024"

[dependencies]
dbx-core = { path = "../dbx-core" }
pgwire = "0.25"          # PostgreSQL wire protocol library
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
```

**main.rs 최소 구현:**
```rust
use pgwire::api::auth::noop::NoopStartupHandler;
use pgwire::api::query::SimpleQueryHandler;
use pgwire::api::MakeHandler;
use pgwire::tokio::process_socket;
use tokio::net::TcpListener;
use dbx_core::Database;
use std::sync::Arc;

struct DbxQueryHandler {
    db: Arc<Database>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Arc::new(Database::open("./dbx-data")?);
    let listener = TcpListener::bind("127.0.0.1:5432").await?;
    println!("DBX Server listening on port 5432 (PostgreSQL wire protocol)");

    loop {
        let (socket, _) = listener.accept().await?;
        let db = db.clone();
        tokio::spawn(async move {
            let handler = DbxQueryHandler { db };
            // process_socket(socket, handler).await;
        });
    }
}
```

**Step 1~5: TDD 사이클 (SimpleQuery "SELECT 1" 응답 확인)**

```bash
git commit -m "feat(server): create dbx-server crate with PG wire protocol skeleton"
```

---

### Task A-2: SimpleQuery 핸들러 구현

**핵심:** `SimpleQueryHandler` trait 구현
- 수신: SQL 문자열
- 실행: `db.execute_sql(sql)` 호출
- 변환: `Vec<RecordBatch>` → PostgreSQL RowDescription + DataRow 메시지
- 반환: pgwire Response

**타입 매핑:**
| Arrow DataType | PG Type OID |
|---------------|-------------|
| Int32 | INT4 (23) |
| Int64 | INT8 (20) |
| Float32 | FLOAT4 (700) |
| Float64 | FLOAT8 (701) |
| Utf8 | TEXT (25) |
| Boolean | BOOL (16) |

```bash
git commit -m "feat(server): implement SimpleQuery handler with Arrow→PG type mapping"
```

---

### Task A-3: psql 연결 테스트

**수동 검증:**
```bash
# 터미널 1
cargo run -p dbx-server

# 터미널 2
psql -h 127.0.0.1 -p 5432 -U dbx
> CREATE TABLE test (id INTEGER, name TEXT);
> INSERT INTO test VALUES (1, 'hello');
> SELECT * FROM test;
```

```bash
git commit -m "test(server): verify psql connectivity and basic SQL operations"
```

---

## Feature B: Point-in-Time Recovery (PITR)

### 개요

기존 WAL(Write-Ahead Log)에 타임스탬프 기반 복구 지점 인덱스를 추가하여, 임의 시점으로 데이터를 복구한다.

```
현재: WAL → 크래시 복구 (마지막 체크포인트~최신)
목표: WAL → 특정 타임스탬프까지만 재적용 (PITR)
```

### Task B-1: WAL 엔트리에 타임스탬프 추가

**Files:**
- Modify: `core/dbx-core/src/wal/mod.rs` — `WalEntry`에 `timestamp_ns: u64` 필드 추가
- Test: WAL 직렬화/역직렬화 테스트

**Step 1: Write the failing test**

```rust
#[test]
fn test_wal_entry_has_timestamp() {
    let entry = WalEntry::new_insert("table", b"key", b"value");
    assert!(entry.timestamp_ns > 0); // 자동으로 현재 시각 기록
}
```

**구현:**
- `WalEntry::new_*()` 메서드에서 `std::time::SystemTime::now()` 의 나노초를 기록
- 기존 WAL 포맷과 역호환: 이전 WAL 파일은 `timestamp_ns = 0`으로 처리

```bash
git commit -m "feat(wal): add nanosecond timestamp to WAL entries"
```

---

### Task B-2: WAL 타임스탬프 인덱스

**Files:**
- Create: `core/dbx-core/src/wal/timestamp_index.rs`
- Test: inline tests

**구조:**
```rust
/// WAL 파일 내 타임스탬프 → 오프셋 매핑
pub struct WalTimestampIndex {
    /// (timestamp_ns, wal_file_id, entry_offset)
    entries: BTreeMap<u64, (u64, u64)>,
}

impl WalTimestampIndex {
    /// 주어진 타임스탬프 이하의 마지막 엔트리 위치 찾기
    pub fn find_entry_before(&self, timestamp_ns: u64) -> Option<(u64, u64)> {
        self.entries.range(..=timestamp_ns).last().map(|(_, v)| *v)
    }

    /// WAL 파일에서 인덱스 빌드
    pub fn build_from_wal(wal_dir: &Path) -> DbxResult<Self> {
        // WAL 파일들을 순서대로 읽으며 인덱스 구축
        todo!()
    }
}
```

```bash
git commit -m "feat(wal): add timestamp index for PITR lookups"
```

---

### Task B-3: Database PITR API

**Files:**
- Create: `core/dbx-core/src/engine/pitr.rs`
- Modify: `core/dbx-core/src/engine/mod.rs` — `pub mod pitr;`
- Test: inline tests

```rust
impl Database {
    /// 특정 시점으로 복구된 읽기 전용 Database 스냅샷 반환
    pub fn recover_to_timestamp(&self, timestamp_ns: u64) -> DbxResult<Database> {
        // 1. 새 in-memory Database 생성
        // 2. WAL 엔트리를 처음부터 timestamp_ns 까지만 재적용
        // 3. 읽기 전용 모드로 반환
        todo!()
    }

    /// ISO 8601 문자열로 PITR
    pub fn recover_to_point(&self, iso_datetime: &str) -> DbxResult<Database> {
        let ts = chrono::DateTime::parse_from_rfc3339(iso_datetime)?;
        let nanos = ts.timestamp_nanos_opt().unwrap_or(0) as u64;
        self.recover_to_timestamp(nanos)
    }
}
```

**테스트:**
```rust
#[test]
fn test_pitr_basic() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path().join("test.db")).unwrap();

    db.insert("t", b"k1", b"v1").unwrap();
    let t1 = std::time::SystemTime::now();
    std::thread::sleep(std::time::Duration::from_millis(10));

    db.insert("t", b"k2", b"v2").unwrap();

    // t1 시점으로 복구 → k1만 있어야 함
    let snapshot = db.recover_to_timestamp(
        t1.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
    ).unwrap();

    assert!(snapshot.get("t", b"k1").unwrap().is_some());
    assert!(snapshot.get("t", b"k2").unwrap().is_none());
}
```

```bash
git commit -m "feat(pitr): add Point-in-Time Recovery API via WAL replay"
```

---

## Feature C: Placement Rules (Grid Engine 배치 정책)

### 개요

Grid Engine에서 테이블/파티션별로 데이터가 어떤 노드에 배치되어야 하는지 정책을 설정한다.

### Task C-1: PlacementRule 타입 & API

**Files:**
- Create: `core/dbx-core/src/grid/placement.rs`
- Modify: `core/dbx-core/src/grid/mod.rs` — `pub mod placement;`
- Test: inline tests

```rust
/// 데이터 배치 정책
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementRule {
    /// 주 저장 영역 (zone 이름)
    pub primary_zone: String,
    /// 복제 영역 목록
    pub replica_zones: Vec<String>,
    /// 최소 복제 수
    pub min_replicas: usize,
    /// 지리적 제한 (true면 primary_zone 밖으로 나가지 않음)
    pub geo_fence: bool,
}

/// 노드별 zone 매핑
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeZoneConfig {
    pub node_id: u32,
    pub zone: String,
    pub weight: f32, // 용량 가중치
}

impl Database {
    pub fn set_placement_rule(&self, table: &str, rule: PlacementRule) -> DbxResult<()> { ... }
    pub fn get_placement_rule(&self, table: &str) -> DbxResult<Option<PlacementRule>> { ... }
    pub fn remove_placement_rule(&self, table: &str) -> DbxResult<bool> { ... }
}
```

**테스트:**
```rust
#[test]
fn test_placement_rule_set_get() {
    let db = Database::open_in_memory().unwrap();
    let rule = PlacementRule {
        primary_zone: "korea".to_string(),
        replica_zones: vec!["japan".to_string()],
        min_replicas: 2,
        geo_fence: false,
    };
    db.set_placement_rule("users", rule.clone()).unwrap();
    let got = db.get_placement_rule("users").unwrap().unwrap();
    assert_eq!(got.primary_zone, "korea");
    assert_eq!(got.min_replicas, 2);
}
```

```bash
git commit -m "feat(grid): add PlacementRule API for data locality control"
```

---

### Task C-2: ShardManager와 PlacementRule 연동

**Files:**
- Modify: `core/dbx-core/src/replication/` — Shard 배치 시 PlacementRule 참조
- Test: 배치 정책에 따라 올바른 노드에 할당되는지 검증

**핵심 로직:**
```
INSERT 발생
  → ShardManager가 대상 Shard 결정
  → PlacementRule 확인
  → primary_zone의 노드 중 weight 기반 선택
  → replica_zones에 비동기 복제
  → geo_fence=true 면 zone 외 노드 배제
```

```bash
git commit -m "feat(grid): integrate PlacementRule with ShardManager for zone-aware placement"
```

---

## 전체 진행 체크리스트

### Feature A: PostgreSQL Wire Protocol
- [ ] Task A-1: `dbx-server` crate 생성
- [ ] Task A-2: SimpleQuery 핸들러 (Arrow → PG 변환)
- [ ] Task A-3: psql 연결 검증

### Feature B: Point-in-Time Recovery
- [ ] Task B-1: WAL 엔트리 타임스탬프 추가
- [ ] Task B-2: WAL 타임스탬프 인덱스
- [ ] Task B-3: Database PITR API

### Feature C: Placement Rules
- [ ] Task C-1: PlacementRule 타입 & API
- [ ] Task C-2: ShardManager 연동

## 의존성 추가

```toml
# core/dbx-server/Cargo.toml (새 crate)
pgwire = "0.25"
```

```toml
# core/dbx-core/Cargo.toml (PITR 관련, 기존 의존성으로 충분)
# chrono는 이미 있음 (0.4)
# WAL, bincode, serde는 이미 있음
```
