---
layout: default
title: 기능 플래그
parent: 한국어
nav_order: 36
---

# 기능 플래그 (Feature Flags)
{: .no_toc }

런타임에 개별 기능을 켜고 끌 수 있는 토글 시스템입니다.
{: .fs-6 .fw-300 }

---

## 개요

기능 플래그를 사용하면 재시작 없이 기능을 활성화/비활성화할 수 있습니다. A/B 테스트, 점진적 롤아웃, 긴급 기능 비활성화에 유용합니다.

---

## 지원 기능 목록

| 플래그 | 설명 | 기본값 |
|--------|------|:------:|
| `BinarySerialization` | 바이너리 직렬화 | 꺼짐 |
| `MultiThreading` | 멀티스레드 실행 | 꺼짐 |
| `MvccExtension` | MVCC 확장 기능 | 꺼짐 |
| `QueryPlanCache` | 쿼리 플랜 캐시 | 꺼짐 |
| `ParallelQuery` | 병렬 쿼리 실행 | 꺼짐 |
| `ParallelWal` | WAL 병렬 쓰기 | 꺼짐 |
| `ParallelCheckpoint` | 병렬 체크포인트 | 꺼짐 |
| `SchemaVersioning` | 스키마 버저닝 | 꺼짐 |
| `IndexVersioning` | 인덱스 버저닝 | 꺼짐 |

---

## 사용법

```rust
use dbx_core::engine::feature_flags::{FeatureFlags, Feature};

let flags = FeatureFlags::new();

// 기능 활성화/비활성화
flags.enable(Feature::ParallelQuery);
flags.disable(Feature::ParallelQuery);
flags.toggle(Feature::QueryPlanCache);

// 상태 확인
if flags.is_enabled(Feature::ParallelQuery) {
    // 병렬 쿼리 실행 경로
}
```

---

## 영속화

### 파일 저장/로드

```rust
// JSON 파일로 저장
flags.save_to_file("./dbx_features.json")?;

// 파일에서 로드
let flags = FeatureFlags::load_from_file("./dbx_features.json")?;
```

### 환경변수

환경변수로 기능을 제어할 수 있습니다:

```bash
# 환경변수 설정
export DBX_FEATURE_PARALLEL_QUERY=true
export DBX_FEATURE_QUERY_PLAN_CACHE=true
```

```rust
// 환경변수에서 로드
let flags = FeatureFlags::load_from_env();
```

---

## 활용 사례

| 시나리오 | 방법 |
|---------|------|
| 신규 기능 점진적 롤아웃 | 일부 서버에서만 `enable` |
| 성능 문제 긴급 대응 | `ParallelQuery`를 `disable` |
| 환경별 설정 | dev/staging/prod별 JSON 파일 |
| CI 테스트 격리 | 환경변수로 기능 조합 테스트 |

---

## 다음 단계

- [병렬 쿼리 가이드](parallel-query) — 병렬 쿼리 상세 설정
- [플랜 캐시 가이드](plan-cache) — 캐시 기능 토글
