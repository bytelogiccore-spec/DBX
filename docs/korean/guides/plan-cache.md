---
layout: default
title: 쿼리 플랜 캐시
parent: 한국어
nav_order: 33
---

# 쿼리 플랜 캐시 (Query Plan Cache)
{: .no_toc }

동일한 SQL을 반복 실행할 때 파싱과 최적화를 건너뛰어 성능을 극대화합니다.
{: .fs-6 .fw-300 }

---

## 개요

SQL 실행은 **파싱 → 논리 계획 → 물리 계획 → 실행** 단계를 거칩니다. 동일한 SQL을 반복 실행하면 매번 파싱/최적화가 발생하여 불필요한 오버헤드가 생깁니다.

플랜 캐시는 이를 해결합니다:

```
첫 실행:  SQL → [파싱 → 최적화 → 플랜 생성] → 실행    (느림)
재실행:  SQL → [캐시 히트!]              → 실행    (빠름)
```

---

## 2계층 캐시 구조

| 계층 | 저장소 | 속도 | 용도 |
|------|--------|------|------|
| **L1** | 메모리 (DashMap) | ~1.6 µs | 자주 사용하는 쿼리 |
| **L2** | 디스크 | ~수 ms | L1에서 밀려난 쿼리 재활용 |

---

## 사용법

```rust
use dbx_core::engine::plan::PlanCache;

// 캐시 생성 (최대 1000개 엔트리)
let cache = PlanCache::new(1000);

// SQL 실행 시 자동으로 캐시됨
let plan = cache.get_or_insert("SELECT * FROM users WHERE id = 1", || {
    // 파싱 + 최적화 (캐시 미스 시에만 실행)
    planner.plan(sql)?
})?;
```

---

## 캐시 통계 확인

```rust
let stats = cache.stats();
println!("히트율: {:.1}%", stats.hit_rate() * 100.0);
println!("히트: {} / 미스: {} / 퇴거: {}", stats.hits, stats.misses, stats.evictions);
```

---

## 성능

| 시나리오 | 캐시 없음 | 캐시 사용 | 개선 |
|---------|:---------:|:---------:|:----:|
| 동일 SQL 10회 반복 | 146 µs | 20 µs | **7.3x** |
| L1 히트 | - | 1.6 µs | - |

---

## 다음 단계

- [병렬 쿼리 가이드](parallel-query) — 대량 데이터 병렬 처리
- [SQL 레퍼런스](sql-reference) — 지원 SQL 구문
