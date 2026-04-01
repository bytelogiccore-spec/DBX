---
layout: default
title: Coprocessor Pushdown Implementation Plan
parent: Plans
---

# Coprocessor Pushdown (Grid Query Execution) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date**: 2026-03-23
**Target**: `dbx-core` v0.2.0
**Priority**: 🏆 Tier A — Game Changer

**Goal:** Grid Engine의 원격 노드에서 필터링/집계를 직접 실행하고 결과만 반환하는 Coprocessor Pushdown을 구현하여, 분산 쿼리 성능을 10~100배 향상시킨다.

**Architecture:** 기존 `GridMessage` enum에 `Query`/`QueryResult` variant를 추가한다. 쿼리 실행 시 Coordinator 노드가 `QueryMessage`를 각 Shard 노드에 전송하면, Remote 노드가 로컬 데이터에 대해 필터링/집계를 수행한 뒤 `QueryResult`만 반환한다. 이는 TiDB의 TiKV Coprocessor와 동일한 패턴이며, 기존 `GridRouter`의 멀티플렉싱 구조에 자연스럽게 통합된다.

**Tech Stack:** `bincode` (직렬화), `tokio` (비동기 통신), 기존 `sql::executor` (로컬 실행)

---

## Phase 1: 프로토콜 확장

### Task 1: GridMessage에 Query variant 추가

**Files:**
- Modify: `core/dbx-core/src/grid/protocol.rs` — `Query`/`QueryResult` variant 추가
- Test: `core/dbx-core/src/grid/protocol.rs` (inline tests)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_message_serialization() {
        let msg = GridMessage::Query(QueryMessage::FilterScan {
            request_id: 1,
            table: "users".to_string(),
            predicate: SerializedPredicate::Equals {
                column: "name".to_string(),
                value: PredicateValue::Text("Alice".to_string()),
            },
            projection: vec!["id".to_string(), "name".to_string()],
            limit: Some(100),
        });

        let encoded = bincode::serialize(&msg).unwrap();
        let decoded: GridMessage = bincode::deserialize(&encoded).unwrap();
        assert_eq!(msg, decoded);
        assert!(!decoded.is_replication());
        assert!(!decoded.is_lock());
        assert!(decoded.is_query());
    }

    #[test]
    fn test_pre_aggregate_message() {
        let msg = GridMessage::Query(QueryMessage::PreAggregate {
            request_id: 2,
            table: "orders".to_string(),
            group_by: vec!["region".to_string()],
            aggregates: vec![
                AggregateSpec { function: AggFunc::Sum, column: "amount".to_string() },
                AggregateSpec { function: AggFunc::Count, column: "*".to_string() },
            ],
            predicate: None,
        });

        let encoded = bincode::serialize(&msg).unwrap();
        let decoded: GridMessage = bincode::deserialize(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_query_result_message() {
        let msg = GridMessage::QueryResult(QueryResultMessage {
            request_id: 1,
            node_id: 42,
            status: QueryStatus::Ok,
            row_count: 15,
            payload: vec![1, 2, 3], // serialized RecordBatch (Arrow IPC)
        });

        let encoded = bincode::serialize(&msg).unwrap();
        let decoded: GridMessage = bincode::deserialize(&encoded).unwrap();
        assert_eq!(msg, decoded);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p dbx-core grid::protocol::tests --no-default-features`
Expected: FAIL — `QueryMessage` not found

**Step 3: Write minimal implementation**

```rust
// grid/protocol.rs에 추가할 타입들

/// 직렬화 가능한 필터 조건 (WHERE절 표현)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PredicateValue {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Null,
}

/// 직렬화 가능한 조건식 (재귀 구조)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SerializedPredicate {
    Equals { column: String, value: PredicateValue },
    NotEquals { column: String, value: PredicateValue },
    GreaterThan { column: String, value: PredicateValue },
    LessThan { column: String, value: PredicateValue },
    GreaterEq { column: String, value: PredicateValue },
    LessEq { column: String, value: PredicateValue },
    And(Box<SerializedPredicate>, Box<SerializedPredicate>),
    Or(Box<SerializedPredicate>, Box<SerializedPredicate>),
    Not(Box<SerializedPredicate>),
    IsNull { column: String },
    IsNotNull { column: String },
}

/// 집계 함수 종류
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggFunc {
    Count,
    Sum,
    Min,
    Max,
    Avg,
}

/// 집계 명세
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AggregateSpec {
    pub function: AggFunc,
    pub column: String,
}

/// Coordinator → Shard 노드로 보내는 쿼리 요청
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryMessage {
    /// 필터링 + 프로젝션 스캔
    FilterScan {
        request_id: u64,
        table: String,
        predicate: SerializedPredicate,
        projection: Vec<String>,
        limit: Option<usize>,
    },
    /// 사전 집계 (GROUP BY를 원격에서 실행)
    PreAggregate {
        request_id: u64,
        table: String,
        group_by: Vec<String>,
        aggregates: Vec<AggregateSpec>,
        predicate: Option<SerializedPredicate>,
    },
    /// 원격 COUNT (스키마 없이 행 수만)
    CountScan {
        request_id: u64,
        table: String,
        predicate: Option<SerializedPredicate>,
    },
}

/// 쿼리 실행 상태
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QueryStatus {
    Ok,
    Error(String),
    PartialResult, // 일부 노드만 응답
}

/// Shard 노드 → Coordinator로 반환하는 결과
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryResultMessage {
    pub request_id: u64,
    pub node_id: u32,
    pub status: QueryStatus,
    pub row_count: usize,
    pub payload: Vec<u8>, // Arrow IPC serialized RecordBatch
}

// GridMessage enum 확장
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GridMessage {
    Replication(ReplicationMessage),
    Lock(LockMessage),
    Query(QueryMessage),              // 🆕
    QueryResult(QueryResultMessage),  // 🆕
}

impl GridMessage {
    pub fn is_replication(&self) -> bool {
        matches!(self, GridMessage::Replication(_))
    }
    pub fn is_lock(&self) -> bool {
        matches!(self, GridMessage::Lock(_))
    }
    pub fn is_query(&self) -> bool {
        matches!(self, GridMessage::Query(_))
    }
    pub fn is_query_result(&self) -> bool {
        matches!(self, GridMessage::QueryResult(_))
    }
}
```

**Step 4: Run test to verify passes**

Run: `cargo test -p dbx-core grid::protocol::tests -v`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add core/dbx-core/src/grid/protocol.rs
git commit -m "feat(grid): extend GridMessage with Query/QueryResult variants for coprocessor pushdown"
```

---

### Task 2: GridRouter에 Query 라우팅 추가

**Files:**
- Modify: `core/dbx-core/src/grid/router.rs` — `Query`/`QueryResult` 매칭 추가
- Test: `core/dbx-core/src/grid/router.rs` (기존 테스트 확장)

**변경 사항:**
- `GridRouter` dispatch 루프에 `GridMessage::Query(q) => { query_tx.send(q) }` 경로 추가
- `GridRouter::new()` 에 `query_rx` 채널 추가
- 기존 `Replication`, `Lock` 라우팅은 변경 없음

```bash
git commit -m "feat(grid): route Query/QueryResult messages through GridRouter"
```

---

## Phase 2: 원격 실행 엔진

### Task 3: Coprocessor 실행 엔진 (Remote Executor)

**Files:**
- Create: `core/dbx-core/src/grid/coprocessor.rs`
- Modify: `core/dbx-core/src/grid/mod.rs` — `pub mod coprocessor;`
- Test: `core/dbx-core/src/grid/coprocessor.rs` (inline tests)

**핵심 구현:**

```rust
// grid/coprocessor.rs

/// Shard 노드에서 수행되는 원격 쿼리 실행 엔진
pub struct Coprocessor<'a> {
    db: &'a Database,
}

impl<'a> Coprocessor<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// QueryMessage를 받아 로컬 데이터에 대해 실행 후 결과 반환
    pub fn execute(&self, msg: QueryMessage) -> QueryResultMessage {
        match msg {
            QueryMessage::FilterScan { request_id, table, predicate, projection, limit } => {
                self.execute_filter_scan(request_id, &table, &predicate, &projection, limit)
            },
            QueryMessage::PreAggregate { request_id, table, group_by, aggregates, predicate } => {
                self.execute_pre_aggregate(request_id, &table, &group_by, &aggregates, predicate.as_ref())
            },
            QueryMessage::CountScan { request_id, table, predicate } => {
                self.execute_count_scan(request_id, &table, predicate.as_ref())
            },
        }
    }

    fn execute_filter_scan(&self, request_id: u64, table: &str,
        predicate: &SerializedPredicate, projection: &[String], limit: Option<usize>
    ) -> QueryResultMessage {
        // 1. 로컬 테이블에서 scan
        // 2. predicate 적용 (필터링)
        // 3. projection 적용 (필요한 컬럼만)
        // 4. limit 적용
        // 5. Arrow IPC로 직렬화하여 반환
        todo!()
    }

    fn execute_pre_aggregate(&self, request_id: u64, table: &str,
        group_by: &[String], aggregates: &[AggregateSpec], predicate: Option<&SerializedPredicate>
    ) -> QueryResultMessage {
        // 1. 로컬 데이터에 predicate 적용
        // 2. GROUP BY 실행
        // 3. 집계 결과를 Arrow RecordBatch로 변환
        // 4. IPC 직렬화 반환 → Coordinator에서 최종 병합
        todo!()
    }

    fn execute_count_scan(&self, request_id: u64, table: &str,
        predicate: Option<&SerializedPredicate>
    ) -> QueryResultMessage {
        // 단순 행 수 카운트 (predicate 있으면 필터 후 카운트)
        todo!()
    }
}
```

**테스트:**
- 로컬 Database에 데이터 삽입 → Coprocessor로 FilterScan → 결과 검증
- PreAggregate → 부분 집계 결과 검증

```bash
git commit -m "feat(grid): add Coprocessor remote execution engine"
```

---

### Task 4: SerializedPredicate → 실제 필터 변환기

**Files:**
- Create: `core/dbx-core/src/grid/predicate_eval.rs`
- Modify: `core/dbx-core/src/grid/mod.rs` — `pub mod predicate_eval;`
- Test: `core/dbx-core/src/grid/predicate_eval.rs` (inline tests)

**핵심:**
- `SerializedPredicate`를 네트워크로 전송 가능한 형태로 설계 (bincode 직렬화)
- 수신 측에서 `evaluate(row: &[(&str, &[u8])]) -> bool`로 평가

```rust
pub fn evaluate_predicate(pred: &SerializedPredicate, row: &HashMap<String, Value>) -> bool {
    match pred {
        SerializedPredicate::Equals { column, value } => {
            row.get(column).map(|v| v == value).unwrap_or(false)
        },
        SerializedPredicate::And(left, right) => {
            evaluate_predicate(left, row) && evaluate_predicate(right, row)
        },
        // ... 나머지 연산
    }
}
```

```bash
git commit -m "feat(grid): add SerializedPredicate evaluator for remote execution"
```

---

## Phase 3: Coordinator 통합

### Task 5: 분산 쿼리 Coordinator

**Files:**
- Create: `core/dbx-core/src/grid/coordinator.rs`
- Modify: `core/dbx-core/src/grid/mod.rs` — `pub mod coordinator;`
- Test: `core/dbx-core/src/grid/coordinator.rs` (inline tests)

**Coordinator 역할:**

```
SQL 쿼리 수신
    ↓
LogicalPlan 생성 (기존 LogicalPlanner)
    ↓
분산 Plan으로 변환 (pushdown 판단)
    ↓
각 Shard 노드에 QueryMessage 전송
    ↓
QueryResultMessage 수集
    ↓
결과 병합 (Merge Sort / Final Aggregate)
    ↓
최종 결과 반환
```

**Pushdown 판단 기준:**
- `SELECT ... WHERE ...` → FilterScan pushdown ✅
- `SELECT COUNT(*) ...` → CountScan pushdown ✅
- `SELECT ... GROUP BY ...` → PreAggregate pushdown ✅
- `JOIN` → 현재는 pushdown 안 함 (향후 Broadcast/Shuffle JOIN)

```bash
git commit -m "feat(grid): add distributed query Coordinator with pushdown optimization"
```

---

### Task 6: Arrow IPC 직렬화 유틸

**Files:**
- Modify: `core/dbx-core/src/storage/arrow_ipc.rs` — RecordBatch IPC 직렬화/역직렬화 추가
- Test: 기존 파일에 추가

**현재 arrow_ipc.rs 에 RecordBatch→bytes, bytes→RecordBatch 변환이 없으면 추가:**

```rust
pub fn batch_to_ipc(batch: &RecordBatch) -> DbxResult<Vec<u8>> {
    let mut buf = Vec::new();
    let mut writer = arrow::ipc::writer::StreamWriter::try_new(&mut buf, &batch.schema())?;
    writer.write(batch)?;
    writer.finish()?;
    Ok(buf)
}

pub fn ipc_to_batches(data: &[u8]) -> DbxResult<Vec<RecordBatch>> {
    let reader = arrow::ipc::reader::StreamReader::try_new(std::io::Cursor::new(data), None)?;
    Ok(reader.collect::<Result<Vec<_>, _>>()?)
}
```

```bash
git commit -m "feat(grid): add Arrow IPC serialization for coprocessor result transfer"
```

---

## Phase 4: 통합 테스트 & 벤치마크

### Task 7: End-to-End 분산 쿼리 테스트

**Files:**
- Create: `core/dbx-core/tests/grid_coprocessor_test.rs`

**시나리오:**
1. 3개 노드 (InMemory Transport) 설정
2. 각 노드에 다른 데이터 삽입 (Shard 시뮬레이션)
3. Coordinator에서 `SELECT * FROM users WHERE age > 25` 실행
4. 결과가 3개 노드의 데이터 병합임을 검증

```bash
git commit -m "test(grid): add end-to-end coprocessor pushdown integration tests"
```

---

### Task 8: 벤치마크 (Pushdown vs No-Pushdown)

**Files:**
- Create: `core/dbx-core/benches/coprocessor_benchmark.rs`

**측정:**
| 시나리오 | No Pushdown | With Pushdown | 예상 개선 |
|---------|-------------|---------------|----------|
| 1M 행, 1% 선택도 필터 | 전체 전송 | 1%만 전송 | ~100x |
| COUNT(*) | 전체 전송 | 숫자 1개 전송 | ~1000x |
| GROUP BY 10 그룹 | 전체 전송 | 사전 집계 전송 | ~100x |

```bash
git commit -m "bench(grid): add coprocessor pushdown benchmarks"
```

---

## 진행 체크리스트

- [ ] Task 1: GridMessage Query/QueryResult variant 추가
- [ ] Task 2: GridRouter Query 라우팅
- [ ] Task 3: Coprocessor 원격 실행 엔진
- [ ] Task 4: SerializedPredicate 평가기
- [ ] Task 5: 분산 쿼리 Coordinator
- [ ] Task 6: Arrow IPC 직렬화
- [ ] Task 7: End-to-End 통합 테스트
- [ ] Task 8: Pushdown vs No-Pushdown 벤치마크
