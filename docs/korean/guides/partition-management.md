---
layout: default
title: 파티션 관리
parent: 한국어
nav_order: 27
---

# 파티션 관리 (Partition Management)
{: .no_toc }

파티션별 통계 수집, 차등 압축, 자동 아카이빙, Hot/Cold 데이터 분리를 통해 대용량 테이블을 효율적으로 관리합니다.
{: .fs-6 .fw-300 }

---

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX는 파티셔닝과 함께 4가지 시너지 기능을 제공합니다.

| 기능 | 설명 | 자동화 |
|------|------|--------|
| **PartitionStats** | 파티션별 row_count, min/max/null/distinct 추적 | INSERT 시 자동 갱신 |
| **차등 압축** | 파티션 나이별로 다른 압축 레벨 적용 | 수동 or Lifecycle 자동 |
| **PartitionLifecycle** | 일정 기간 후 고압축·삭제 자동 처리 | ✅ 백그라운드 완전 자동 |
| **PartitionTierHint** | Hot/Warm/Cold 스토리지 티어 분류 | 수동 or Lifecycle 자동 |

---

## 파티션 생성 (사전 조건)

모든 파티션 관리 기능은 `create_partition()` 호출 후 사용 가능합니다.

```rust
use dbx_core::{Database, storage::partition::{PartitionMap, PartitionType}};

let db = Database::open("./db")?;
db.execute_sql("CREATE TABLE orders (id INT, amount FLOAT, created_at INT)")?;

db.create_partition(PartitionMap {
    table: "orders".into(),
    partition_type: PartitionType::Hash {
        column: "id".into(),
        num_partitions: 4,
    },
    num_partitions: 4,
})?;
```

---

## 1. PartitionStats — 파티션별 통계

쿼리 옵티마이저가 실행 계획을 최적화하는 데 활용되는 파티션별 통계 정보입니다.

### 자동 갱신 (INSERT 시)

별도 호출 없이 INSERT 때마다 `row_count`가 자동으로 증가합니다.

```rust
// 파티셔닝된 테이블에 INSERT → row_count 자동 +1
for i in 0..1000 {
    db.insert("orders", format!("{}", i).as_bytes(), b"data")?;
}

// 파티션별 row_count 확인
let all_stats = db.all_partition_stats("orders")?;
for (partition, stats) in &all_stats {
    println!("{}: {}건", partition, stats.row_count);
}
// orders__p_part_0: 245건
// orders__p_part_1: 258건
// orders__p_part_2: 251건
// orders__p_part_3: 246건
```

### 수동 갱신 (정밀 통계 힌트)

min/max/null/distinct 값을 직접 제공해 옵티마이저 정확도를 높입니다.

```rust
use dbx_core::storage::partition::PartitionStats;

db.update_partition_stats("orders", "orders__p_part_0", PartitionStats {
    row_count: 245,
    min_value: 0,
    max_value: 999,
    null_count: 0,
    distinct_count: 245,
})?;

let stats = db.get_partition_stats("orders", "orders__p_part_0")?;
println!("행 수: {}", stats.row_count);
```

---

## 2. 파티션별 차등 압축

파티션 나이와 접근 패턴에 따라 다른 압축 레벨을 적용합니다.

```rust
use dbx_core::storage::compression::CompressionConfig;

// 최근 파티션(Hot) — 저압축, 빠른 읽기/쓰기
db.set_partition_compression("orders", "orders__p_part_3",
    CompressionConfig::zstd_level(3))?;

// 오래된 파티션(Cold) — 고압축, 디스크 절약
db.set_partition_compression("orders", "orders__p_part_0",
    CompressionConfig::zstd_level(9))?;

// 설정 조회 (미설정 시 기본값 Snappy 반환)
let config = db.get_partition_compression("orders", "orders__p_part_0")?;
```

### 압축 레벨 가이드

| 레벨 | 용도 | 압축률 | 속도 |
|------|------|--------|------|
| 1 ~ 3 | 실시간 데이터 (Hot) | 낮음 | 매우 빠름 |
| 4 ~ 6 | 일반 데이터 (Warm) | 중간 | 빠름 |
| 7 ~ 9 | 아카이브 (Cold) | 높음 | 보통 |

---

## 3. PartitionLifecycle — 완전 자동 아카이빙

`enable_auto_archive()` **한 번만** 호출하면 이후 모든 것이 자동입니다.

```rust
use dbx_core::storage::partition::PartitionLifecycle;

db.enable_auto_archive("orders", PartitionLifecycle {
    archive_after_days: 90,   // 90일 후 → ZSTD 레벨 9 + Cold 티어 자동 적용
    delete_after_days: 365,   // 365일 후 → 메타데이터 자동 삭제
})?;

// 이후 아무것도 안 해도 됩니다.
// 백그라운드에서 1시간마다 자동으로 실행됩니다.
```

### 자동화 동작 방식

```
enable_auto_archive() 호출
    │
    └─ [dbx-lifecycle-scheduler] 백그라운드 스레드 자동 기동
            ↓ 매 1시간
        모든 파티션 나이 확인
        ├─ archive_after_days 경과 → ZSTD 9 압축 + Cold 티어 자동 적용
        └─ delete_after_days 경과 → 메타데이터 전체 삭제
```

> **스레드 보장**: 여러 테이블에 `enable_auto_archive()`를 호출해도 스케줄러 스레드는 1개만 생성됩니다 (CAS 보장).

### 온디맨드 즉시 실행

자동 스케줄 외에 원하는 시점에 즉시 실행할 수 있습니다.

```rust
// 단일 테이블
let (archived, deleted) = db.run_partition_lifecycle("orders")?;
println!("아카이브: {}개, 삭제: {}개", archived, deleted);

// 모든 테이블 일괄 처리
let (total_a, total_d) = db.run_all_partition_lifecycles()?;
```

### 파티션 생성 시각 조회

INSERT가 발생한 순간 자동으로 기록됩니다.

```rust
// 조회 (없으면 None)
let created_at: Option<u64> = db.get_partition_creation_time("orders__p_part_0");

// 수동 조건 확인
let needs_archive = db.partition_needs_archive("orders", created_at.unwrap_or(0))?;
let needs_delete  = db.partition_needs_delete("orders", created_at.unwrap_or(0))?;
```

---

## 4. PartitionTierHint — Hot/Cold 데이터 분리

파티션에 스토리지 티어 힌트를 부여해 쿼리 라우팅과 비용 최적화에 활용합니다.

```rust
use dbx_core::storage::partition::PartitionTierHint;

// 최근 파티션 → Hot (메모리/SSD 우선)
db.set_partition_tier("orders", "orders__p_part_3", PartitionTierHint::Hot)?;
// 중간 파티션 → Warm (SSD)
db.set_partition_tier("orders", "orders__p_part_2", PartitionTierHint::Warm)?;
// 오래된 파티션 → Cold (HDD + 고압축)
db.set_partition_tier("orders", "orders__p_part_0", PartitionTierHint::Cold)?;

// 티어 조회 (미설정 시 기본값 Hot 반환)
let tier = db.get_partition_tier("orders", "orders__p_part_3")?;

// 티어별 파티션 목록
let hot_list  = db.list_partitions_by_tier("orders", PartitionTierHint::Hot)?;
let cold_list = db.list_partitions_by_tier("orders", PartitionTierHint::Cold)?;
```

---

## 권장 패턴 — 전체 조합

```rust
use dbx_core::{Database, storage::partition::*};

let db = Database::open("./db")?;
db.execute_sql("CREATE TABLE logs (id INT, msg TEXT)")?;

// 1. 파티션 생성
db.create_partition(PartitionMap {
    table: "logs".into(),
    partition_type: PartitionType::Hash {
        column: "id".into(),
        num_partitions: 8,
    },
    num_partitions: 8,
})?;

// 2. 완전 자동 아카이빙 설정 — 이 한 줄로 전체 자동화
db.enable_auto_archive("logs", PartitionLifecycle {
    archive_after_days: 90,
    delete_after_days: 365,
})?;

// 3. 데이터 삽입 — row_count와 creation_time 자동 기록
for i in 0..10000 {
    db.insert("logs", format!("{}", i).as_bytes(), b"log data")?;
}

// 4. 이후 관리 불필요 — 백그라운드에서 자동 처리
```

**예상 효과**:
- 📊 쿼리 속도: 10-50배 향상 (파티션 프루닝)
- 💾 디스크 사용: 50-70% 절감 (고압축 아카이브)
- 🤖 운영 부담: 완전 자동화

---

## 다음 단계

- [압축 가이드](compression) — 글로벌 압축 설정
- [스토리지 계층](storage-layers) — 5-Tier 아키텍처와 파티셔닝의 관계
- [스케줄러 가이드](scheduler) — 커스텀 스케줄 작업 등록
