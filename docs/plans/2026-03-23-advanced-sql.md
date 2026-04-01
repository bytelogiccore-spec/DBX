---
layout: default
title: Advanced SQL (Window Functions, CTE, Subquery) Implementation Plan
parent: Plans
---

# Advanced SQL Implementation Plan (Window Function, CTE, Subquery)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date**: 2026-03-23
**Target**: `dbx-core` v0.2.0
**Priority**: 🏆 Tier A — HTAP 완성

**Goal:** Window Function, CTE(Common Table Expression), 서브쿼리를 구현하여 DBX의 HTAP 분석 쿼리 능력을 완성한다.

**Architecture:** `sqlparser` 0.52가 이미 Window/CTE/Subquery 구문을 파싱하므로, `LogicalPlanner`에서 논리 플랜 노드를 추가하고 `PhysicalPlanner` + 전용 `PhysicalOperator`를 구현한다. Arrow RecordBatch 기반으로 실행되며, 기존 `HashAggregateOperator`/`SortOperator`를 재활용한다.

**Tech Stack:** `sqlparser` 0.52 (파싱), `arrow` 54 (RecordBatch 연산), 기존 `sql::executor` 파이프라인

---

## 현재 코드 구조 참고

```
sql/
├── parser.rs              — sqlparser 래핑
├── planner/
│   ├── types.rs           — Expr, LogicalPlan, PhysicalPlan 정의
│   ├── logical/           — LogicalPlanner
│   └── physical.rs        — PhysicalPlanner
├── executor/
│   ├── expr.rs            — Expression 평가
│   ├── operators/
│   │   ├── filter.rs
│   │   ├── hash_aggregate.rs
│   │   ├── join.rs
│   │   ├── sort.rs
│   │   ├── projection.rs
│   │   ├── limit.rs
│   │   └── table_scan.rs
│   └── parallel_query.rs
├── optimizer/
│   ├── predicate_pushdown.rs
│   ├── projection_pushdown.rs
│   ├── constant_folding.rs
│   └── limit_pushdown.rs
└── interface.rs           — SQL 진입점 (execute_sql)
```

---

## Phase 1: 서브쿼리 (Subquery)

### Task 1: Scalar Subquery 지원

**Files:**
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `Expr::ScalarSubquery(Box<LogicalPlan>)` 추가
- Modify: `core/dbx-core/src/sql/planner/logical/` — 서브쿼리 Expression 변환
- Modify: `core/dbx-core/src/sql/executor/expr.rs` — 서브쿼리 평가
- Test: 통합 테스트

**Step 1: Write the failing test**

```rust
#[test]
fn test_scalar_subquery() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE orders (id INTEGER, amount FLOAT, region TEXT)").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (1, 100.0, 'KR')").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (2, 200.0, 'US')").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (3, 150.0, 'KR')").unwrap();

    // Scalar subquery: 평균보다 큰 주문
    let result = db.execute_sql(
        "SELECT id, amount FROM orders WHERE amount > (SELECT AVG(amount) FROM orders)"
    ).unwrap();

    // AVG = 150, 200 > 150 이므로 id=2만 반환
    assert_eq!(result[0].num_rows(), 1);
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p dbx-core test_scalar_subquery -v`
Expected: FAIL — subquery not supported

**Step 3: Implementation outline**

1. `types.rs`에 `Expr::ScalarSubquery(Box<LogicalPlan>)` 추가
2. `LogicalPlanner`에서 `sqlparser::ast::Expr::Subquery` → `Expr::ScalarSubquery` 변환
3. `expr.rs`의 `evaluate_expr`에서 `ScalarSubquery` → 내부적으로 execute 후 단일 값 추출
4. 스칼라가 아닌 경우 (여러 행/컬럼) → 에러 반환

**Step 4: Run test to verify passes**

**Step 5: Commit**

```bash
git commit -m "feat(sql): add scalar subquery support in WHERE clause"
```

---

### Task 2: IN Subquery 지원

**Files:**
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `Expr::InSubquery { expr, subquery, negated }` 추가
- Modify: `core/dbx-core/src/sql/executor/expr.rs` — IN 평가 (HashSet 기반)
- Test: 통합 테스트

**테스트:**

```rust
#[test]
fn test_in_subquery() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE regions (name TEXT, active INTEGER)").unwrap();
    db.execute_sql("INSERT INTO regions VALUES ('KR', 1)").unwrap();
    db.execute_sql("INSERT INTO regions VALUES ('US', 0)").unwrap();

    db.execute_sql("CREATE TABLE orders (id INTEGER, region TEXT)").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (1, 'KR')").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (2, 'US')").unwrap();
    db.execute_sql("INSERT INTO orders VALUES (3, 'JP')").unwrap();

    let result = db.execute_sql(
        "SELECT id FROM orders WHERE region IN (SELECT name FROM regions WHERE active = 1)"
    ).unwrap();

    // KR만 active → id=1만 반환
    assert_eq!(result[0].num_rows(), 1);
}
```

**구현:**
- 서브쿼리를 먼저 실행 → 결과를 `HashSet` 으로 변환
- 외부 쿼리의 각 행에서 `HashSet.contains()` 체크
- `NOT IN`도 동일 로직 + 부정

```bash
git commit -m "feat(sql): add IN/NOT IN subquery support"
```

---

### Task 3: EXISTS Subquery 지원

**Files:**
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `Expr::ExistsSubquery { subquery, negated }`
- Test: 통합 테스트

```rust
#[test]
fn test_exists_subquery() {
    let db = Database::open_in_memory().unwrap();
    // ... 테이블 설정
    let result = db.execute_sql(
        "SELECT id FROM orders WHERE EXISTS (SELECT 1 FROM regions WHERE regions.name = orders.region AND active = 1)"
    ).unwrap();
    // Correlated subquery — 외부 행마다 서브쿼리 재실행
}
```

> **주의**: Correlated subquery는 성능상 비용이 크므로, 단순 EXISTS만 먼저 구현하고 Correlated는 Phase 2(최적화)로 미룰 수 있음.

```bash
git commit -m "feat(sql): add EXISTS/NOT EXISTS subquery support"
```

---

## Phase 2: CTE (Common Table Expression)

### Task 4: Non-Recursive CTE

**Files:**
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `LogicalPlan::CTE { name, body, query }` 추가
- Modify: `core/dbx-core/src/sql/planner/logical/` — `WITH` 절 변환
- Modify: `core/dbx-core/src/sql/interface.rs` — CTE 실행 플로우 추가
- Test: 통합 테스트

**Step 1: Write the failing test**

```rust
#[test]
fn test_non_recursive_cte() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE events (id INTEGER, type TEXT, value FLOAT)").unwrap();
    db.execute_sql("INSERT INTO events VALUES (1, 'click', 1.0)").unwrap();
    db.execute_sql("INSERT INTO events VALUES (2, 'click', 2.0)").unwrap();
    db.execute_sql("INSERT INTO events VALUES (3, 'view', 5.0)").unwrap();
    db.execute_sql("INSERT INTO events VALUES (4, 'click', 3.0)").unwrap();

    let result = db.execute_sql(
        "WITH click_events AS (
            SELECT id, value FROM events WHERE type = 'click'
        )
        SELECT id, value FROM click_events WHERE value > 1.5"
    ).unwrap();

    // click 이벤트 중 value > 1.5 → id=2 (2.0), id=4 (3.0)
    assert_eq!(result[0].num_rows(), 2);
}
```

**Step 2: Run to verify fails**

Run: `cargo test -p dbx-core test_non_recursive_cte -v`
Expected: FAIL

**Step 3: Implementation outline**

1. `interface.rs`에서 `sqlparser::ast::Query.with` 확인
2. CTE body를 먼저 실행 → 결과 RecordBatch를 임시 테이블로 등록
3. 메인 쿼리 실행 시 CTE 이름으로 임시 테이블 참조
4. 다중 CTE 지원 (`WITH a AS (...), b AS (...) SELECT ...`)
5. 실행 후 임시 테이블 정리

```bash
git commit -m "feat(sql): add non-recursive CTE (WITH clause) support"
```

---

### Task 5: 다중 CTE & CTE 간 참조

**테스트:**

```rust
#[test]
fn test_chained_cte() {
    let db = Database::open_in_memory().unwrap();
    // ... 테이블 설정

    let result = db.execute_sql(
        "WITH
            active_users AS (SELECT id, name FROM users WHERE active = 1),
            user_orders AS (SELECT user_id, SUM(amount) as total FROM orders GROUP BY user_id)
        SELECT a.name, u.total
        FROM active_users a, user_orders u
        WHERE a.id = u.user_id"
    ).unwrap();
}
```

```bash
git commit -m "feat(sql): support multiple CTEs with cross-references"
```

---

## Phase 3: Window Functions

### Task 6: Window Function 기본 프레임워크

**Files:**
- Create: `core/dbx-core/src/sql/executor/operators/window.rs`
- Modify: `core/dbx-core/src/sql/executor/operators/mod.rs` — `pub mod window;`
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `WindowExpr`, `WindowFrame` 등 타입 추가
- Modify: `core/dbx-core/src/sql/planner/physical.rs` — Window 물리 플랜 생성
- Test: 통합 테스트

**타입 정의 (planner/types.rs):**

```rust
/// 윈도우 함수 종류
#[derive(Debug, Clone, PartialEq)]
pub enum WindowFunction {
    RowNumber,
    Rank,
    DenseRank,
    Lag(usize),     // offset
    Lead(usize),    // offset
    NTile(usize),   // buckets
    // 집계 윈도우
    Sum,
    Avg,
    Count,
    Min,
    Max,
}

/// 윈도우 프레임 범위
#[derive(Debug, Clone, PartialEq)]
pub enum WindowFrameBound {
    UnboundedPreceding,
    Preceding(usize),
    CurrentRow,
    Following(usize),
    UnboundedFollowing,
}

/// 윈도우 프레임 정의
#[derive(Debug, Clone, PartialEq)]
pub struct WindowFrame {
    pub start: WindowFrameBound,
    pub end: WindowFrameBound,
}

/// 윈도우 Expression
#[derive(Debug, Clone, PartialEq)]
pub struct WindowExpr {
    pub function: WindowFunction,
    pub args: Vec<Expr>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<SortExpr>,
    pub frame: Option<WindowFrame>,
    pub alias: String,
}
```

**Step 1: Write the failing test**

```rust
#[test]
fn test_row_number() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE sales (id INTEGER, region TEXT, amount FLOAT)").unwrap();
    db.execute_sql("INSERT INTO sales VALUES (1, 'KR', 100.0)").unwrap();
    db.execute_sql("INSERT INTO sales VALUES (2, 'KR', 200.0)").unwrap();
    db.execute_sql("INSERT INTO sales VALUES (3, 'US', 150.0)").unwrap();
    db.execute_sql("INSERT INTO sales VALUES (4, 'US', 300.0)").unwrap();

    let result = db.execute_sql(
        "SELECT id, region, amount, ROW_NUMBER() OVER (PARTITION BY region ORDER BY amount DESC) as rn
         FROM sales"
    ).unwrap();

    // KR: (2, 200, rn=1), (1, 100, rn=2)
    // US: (4, 300, rn=1), (3, 150, rn=2)
    assert_eq!(result[0].num_rows(), 4);
}
```

**Step 3: Implementation outline for WindowOperator**

```rust
// executor/operators/window.rs

pub struct WindowOperator {
    pub input: Box<dyn PhysicalOperator>,
    pub window_exprs: Vec<WindowExpr>,
}

impl PhysicalOperator for WindowOperator {
    fn execute(&self) -> DbxResult<Vec<RecordBatch>> {
        let input_batches = self.input.execute()?;

        for batch in &input_batches {
            // 1. PARTITION BY 기준으로 행 그룹화
            // 2. 각 파티션 내에서 ORDER BY 정렬
            // 3. 각 행에 대해 윈도우 함수 계산
            // 4. 결과 컬럼을 원본에 append
        }
        todo!()
    }
}
```

```bash
git commit -m "feat(sql): add WindowOperator framework with ROW_NUMBER support"
```

---

### Task 7: RANK, DENSE_RANK, NTILE 구현

**테스트:**

```rust
#[test]
fn test_rank_and_dense_rank() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE scores (name TEXT, score INTEGER)").unwrap();
    db.execute_sql("INSERT INTO scores VALUES ('A', 100)").unwrap();
    db.execute_sql("INSERT INTO scores VALUES ('B', 100)").unwrap();
    db.execute_sql("INSERT INTO scores VALUES ('C', 90)").unwrap();

    let result = db.execute_sql(
        "SELECT name, score,
                RANK() OVER (ORDER BY score DESC) as rank,
                DENSE_RANK() OVER (ORDER BY score DESC) as dense_rank
         FROM scores"
    ).unwrap();

    // A: rank=1, dense_rank=1
    // B: rank=1, dense_rank=1
    // C: rank=3, dense_rank=2  (RANK는 3, DENSE_RANK는 2)
}
```

```bash
git commit -m "feat(sql): add RANK, DENSE_RANK, NTILE window functions"
```

---

### Task 8: LAG, LEAD 구현

**테스트:**

```rust
#[test]
fn test_lag_lead() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE ts (date TEXT, value FLOAT)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES ('2026-01', 100.0)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES ('2026-02', 120.0)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES ('2026-03', 110.0)").unwrap();

    let result = db.execute_sql(
        "SELECT date, value,
                LAG(value, 1) OVER (ORDER BY date) as prev_value,
                LEAD(value, 1) OVER (ORDER BY date) as next_value
         FROM ts"
    ).unwrap();

    // 2026-01: prev=NULL, next=120
    // 2026-02: prev=100, next=110
    // 2026-03: prev=120, next=NULL
}
```

```bash
git commit -m "feat(sql): add LAG/LEAD window functions for time-series analysis"
```

---

### Task 9: 집계 Window Function (SUM/AVG OVER)

**테스트:**

```rust
#[test]
fn test_rolling_sum() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE ts (month INTEGER, revenue FLOAT)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES (1, 100.0)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES (2, 200.0)").unwrap();
    db.execute_sql("INSERT INTO ts VALUES (3, 150.0)").unwrap();

    let result = db.execute_sql(
        "SELECT month, revenue,
                SUM(revenue) OVER (ORDER BY month ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) as cumulative
         FROM ts"
    ).unwrap();

    // month=1: cum=100, month=2: cum=300, month=3: cum=450
}
```

```bash
git commit -m "feat(sql): add aggregate window functions (SUM/AVG/COUNT OVER)"
```

---

## Phase 4: UNION & 기타

### Task 10: UNION / UNION ALL

**Files:**
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `LogicalPlan::Union { plans, all }`
- Modify: `core/dbx-core/src/sql/interface.rs` — UNION 처리
- Test: 통합 테스트

```rust
#[test]
fn test_union_all() {
    let db = Database::open_in_memory().unwrap();
    db.execute_sql("CREATE TABLE t1 (id INTEGER, name TEXT)").unwrap();
    db.execute_sql("CREATE TABLE t2 (id INTEGER, name TEXT)").unwrap();
    db.execute_sql("INSERT INTO t1 VALUES (1, 'A')").unwrap();
    db.execute_sql("INSERT INTO t2 VALUES (2, 'B')").unwrap();
    db.execute_sql("INSERT INTO t2 VALUES (1, 'A')").unwrap();

    let result = db.execute_sql("SELECT * FROM t1 UNION ALL SELECT * FROM t2").unwrap();
    assert_eq!(result[0].num_rows(), 3); // 중복 포함

    let result = db.execute_sql("SELECT * FROM t1 UNION SELECT * FROM t2").unwrap();
    assert_eq!(result[0].num_rows(), 2); // 중복 제거 (1,'A'는 1번만)
}
```

```bash
git commit -m "feat(sql): add UNION/UNION ALL support"
```

---

### Task 11: 벤치마크 & 문서화

**Files:**
- Create: `core/dbx-core/benches/advanced_sql_benchmark.rs`
- Modify: `core/dbx-core/Cargo.toml` — `[[bench]]` 추가

**벤치마크 시나리오:**

| 쿼리 타입 | 데이터 크기 | 측정 대상 |
|----------|-----------|----------|
| Window ROW_NUMBER | 10K 행, 100 파티션 | 실행 시간 |
| CTE 2단계 | 10K 행 | CTE vs 인라인 서브쿼리 |
| IN Subquery | 10K 외부 × 1K 서브쿼리 | HashSet 최적화 효과 |
| Cumulative SUM OVER | 10K 행 시계열 | 프레임 처리 성능 |

```bash
git commit -m "bench(sql): add advanced SQL feature benchmarks"
```

---

## 진행 체크리스트

### Phase 1: 서브쿼리
- [ ] Task 1: Scalar Subquery
- [ ] Task 2: IN/NOT IN Subquery
- [ ] Task 3: EXISTS/NOT EXISTS Subquery

### Phase 2: CTE
- [ ] Task 4: Non-Recursive CTE
- [ ] Task 5: 다중 CTE & CTE 간 참조

### Phase 3: Window Functions
- [ ] Task 6: WindowOperator + ROW_NUMBER
- [ ] Task 7: RANK, DENSE_RANK, NTILE
- [ ] Task 8: LAG, LEAD
- [ ] Task 9: SUM/AVG/COUNT OVER (집계 윈도우)

### Phase 4: 기타
- [ ] Task 10: UNION / UNION ALL
- [ ] Task 11: 벤치마크 & 문서화

## 의존성 변경

**없음** — `sqlparser` 0.52와 `arrow` 54가 이미 모든 파싱/실행 기반을 제공합니다.
