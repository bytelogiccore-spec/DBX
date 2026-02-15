---
layout: default
title: 병렬 쿼리
parent: 한국어
nav_order: 34
---

# 병렬 쿼리 실행 (Parallel Query)
{: .no_toc }

대량 데이터를 Rayon 스레드 풀로 병렬 처리하여 쿼리 성능을 높입니다.
{: .fs-6 .fw-300 }

---

## 개요

DBX의 병렬 쿼리 실행기는 여러 RecordBatch를 동시에 처리합니다. 데이터가 충분히 클 때만 병렬화하여 소규모 데이터에서는 오버헤드를 방지합니다.

```
소규모 (< 1,000행):  순차 실행    → 오버헤드 없음
대규모 (≥ 1,000행):  병렬 실행    → 멀티코어 활용
```

---

## 지원 연산

| 연산 | 메서드 | 설명 |
|------|--------|------|
| **필터** | `par_filter()` | 조건에 맞는 행 병렬 선별 |
| **집계** | `par_aggregate()` | SUM, COUNT, AVG, MIN, MAX 병렬 계산 |
| **프로젝션** | `par_project()` | 필요한 컬럼만 병렬 추출 |

---

## 사용법

```rust
use dbx_core::sql::executor::parallel_query::{
    ParallelQueryExecutor, AggregateType
};

let executor = ParallelQueryExecutor::new();  // 기본: 1000행 이상 시 병렬

// 병렬 집계
let result = executor.par_aggregate(&batches, 0, AggregateType::Sum)?;
println!("합계: {}, 건수: {}", result.value, result.count);

// 커스텀 설정
let executor = ParallelQueryExecutor::new()
    .with_min_rows(5000)         // 5000행 이상 시 병렬
    .with_threshold(4)           // batch 4개 이상 시
    .with_thread_pool(pool);     // 커스텀 스레드 풀
```

---

## 병렬화 판단 기준

병렬 실행은 두 조건을 **모두** 만족할 때 활성화됩니다:

1. **batch 수** ≥ `parallel_threshold` (기본 2)
2. **총 행 수** ≥ `min_rows_for_parallel` (기본 1,000)

---

## 성능

| 데이터 크기 | 순차 | 병렬 | 비고 |
|-----------|:----:|:----:|------|
| 150행 | 431 ns | 32.5 µs | 🚫 병렬이 느림 → 순차 fallback |
| 10,000행 | ~50 µs | ~15 µs | ✅ 병렬이 빠름 |
| 100만행 | ~5 ms | ~1.2 ms | 🔥 4x 개선 |

---

## 다음 단계

- [플랜 캐시 가이드](plan-cache) — SQL 반복 실행 최적화
- [WAL 복구 가이드](wal-recovery) — WAL 파티셔닝과의 연계
