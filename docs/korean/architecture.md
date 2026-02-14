---
layout: default
title: 아키텍처
nav_order: 3
description: "DBX 5-Tier 하이브리드 스토리지 아키텍처"
---

# 아키텍처
{: .no_toc }

DBX의 5-Tier 하이브리드 스토리지 아키텍처와 MVCC 트랜잭션 시스템에 대해 심층적으로 알아봅니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 5-Tier 하이브리드 스토리지

DBX는 OLTP와 OLAP 워크로드 모두에 최적화된 정교한 5계층 아키텍처를 사용합니다:

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← 인메모리 쓰기 버퍼
│     - Lock-free 동시성                  │
│     - 핫 데이터 캐싱                    │
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← OLAP 최적화
│     - RecordBatch 캐싱                  │
│     - Projection Pushdown               │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (sled)                     │  ← 영구 저장소
│     - Write-Optimized Store             │
│     - MVCC 및 스냅샷 격리               │
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← 빠른 존재 확인
│     - 오탐지 최소화                     │
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Parquet)                  │  ← 컬럼형 압축
│     - Read-Optimized Store              │
│     - Apache Arrow/Parquet              │
└─────────────────────────────────────────┘

                  선택 사항: GPU 가속
┌─────────────────────────────────────────┐
│  GPU Manager (CUDA)                     │  ← 분석 쿼리 가속
│     - GROUP BY, Hash Join               │
│     - 필터링, 집계                      │
└─────────────────────────────────────────┘
```

### Tier 1: Delta Store

**목적**: 핫 데이터를 위한 인메모리 쓰기 버퍼

**구현**: `BTreeMap<Vec<u8>, Vec<u8>>`

**특징**:
- Lock-free 동시 읽기
- 빠른 쓰기 (O(log n))
- 임계값 도달 시 자동 Flush
- 하위 계층을 가림 (Shadowing)

### Tier 2: Columnar Cache

**목적**: OLAP 쿼리 최적화

**구현**: Apache Arrow `RecordBatch`

**특징**:
- 컬럼형 저장 포맷
- Projection pushdown
- Predicate pushdown
- 제로 카피 연산
- 벡터화 실행 (SIMD)

### Tier 3: WOS (Write-Optimized Store)

**목적**: 영구적 트랜잭션 저장소

**구현**: `sled` 임베디드 데이터베이스 (현재는 BTreeMap으로 단순화됨)

**특징**:
- MVCC 및 스냅샷 격리
- ACID 트랜잭션
- 크래시 복구
- 컴팩션 (Compaction)

### Tier 4: Index

**목적**: 빠른 존재 확인

**구현**: Bloom Filter

**특징**:
- 오탐지 최소화
- 빠른 조회 (O(1))
- 공간 효율적

### Tier 5: ROS (Read-Optimized Store)

**목적**: 장기 컬럼형 저장소

**구현**: Apache Parquet

**특징**:
- 컬럼형 압축
- 효율적인 스캔
- Predicate pushdown
- 스키마 진화 (Schema evolution)

---

## MVCC 트랜잭션 시스템

DBX는 스냅샷 격리(Snapshot Isolation) 기능이 포함된 다중 버전 동시성 제어(MVCC)를 구현합니다.

### 트랜잭션 흐름

```
트랜잭션 시작 (Transaction Begin)
    ↓
스냅샷 격리 (read_ts 부여)
    ↓
읽기/쓰기 작업
    ↓
커밋 (commit_ts 부여)
    ↓
가비지 컬렉션 (비동기 처리)
```

### 버전 관리 (Versioning)

각 레코드는 타임스탬프와 함께 버전이 관리됩니다:

```rust
struct VersionedValue {
    value: Vec<u8>,
    version: u64,      // 트랜잭션 타임스탬프
    deleted: bool,     // 삭제 마커(Tombstone)
}
```

### 스냅샷 격리 (Snapshot Isolation)

- 각 트랜잭션은 일관된 스냅샷을 봅니다.
- 트랜잭션 시작 시 읽기 타임스탬프(`read_ts`)가 할당됩니다.
- 커밋 시 쓰기 타임스탬프(`commit_ts`)가 할당됩니다.
- 읽기 시 `version <= read_ts` 인 버전을 조회합니다.

### 가비지 컬렉션 (Garbage Collection)

- 비동기 백그라운드 프로세스
- 더 이상 보이지 않는 오래된 버전 제거
- 구성 가능한 보관 정책

---

## GPU 가속

DBX는 분석 쿼리를 위해 선택적으로 CUDA 기반 GPU 가속을 지원합니다.

### 지원되는 작업

- **집계 (Aggregations)**: SUM, COUNT, MIN, MAX, AVG
- **필터링 (Filtering)**: 조건문 평가
- **GROUP BY**: 해시 기반 그룹화
- **Hash Join**: 등가 조인 (Equi-joins)

### 해시 전략 (Hash Strategies)

DBX는 여러 GPU 해시 전략을 지원합니다:

| 전략 | 성능 | 사용 사례 |
|----------|-------------|----------|
| **Linear** | 안정적 | 소규모 그룹 (기본값) |
| **Cuckoo** | 공격적 | SUM +73%, Filtering +32% |
| **Robin Hood** | 균형 잡힘 | SUM +7%, Filtering +10% |

### 성능

GPU 가속은 대규모 데이터셋에서 상당한 성능 향상을 보여줍니다:

- **100만 행**: 3.06배 빠름 (필터링)
- **1000만 행 이상**: 최대 4.57배 빠름

---

## 쿼리 최적화

### Projection Pushdown

저장소에서 필요한 컬럼만 읽습니다:

```sql
SELECT id, name FROM users;  -- 'id'와 'name' 컬럼만 읽음
```

### Predicate Pushdown

저장 계층에서 데이터를 필터링합니다:

```sql
SELECT * FROM users WHERE age > 30;  -- 스캔 중에 필터 적용
```

### 벡터화 실행 (Vectorized Execution)

Arrow RecordBatch를 이용한 SIMD 연산:

- 여러 행을 동시에 처리
- CPU 캐시 친화적
- 제로 카피 데이터 액세스

---

## 데이터 흐름

### 쓰기 경로 (Write Path)

```
애플리케이션
    ↓
Delta Store (Tier 1)
    ↓ (임계값 도달 시 자동 Flush)
WOS (Tier 3)
    ↓ (컴팩션)
ROS (Tier 5)
```

### 읽기 경로 (OLTP)

```
애플리케이션
    ↓
Delta Store (Tier 1) → 있으면 즉시 반환
    ↓
WOS (Tier 3) → 있으면 즉시 반환
    ↓
Index (Tier 4) → 존재 여부 확인
    ↓
ROS (Tier 5) → Parquet에서 읽기
```

### 읽기 경로 (OLAP)

```
애플리케이션 (SQL 쿼리)
    ↓
쿼리 옵티마이저
    ↓
Columnar Cache (Tier 2) → 캐시되어 있으면 즉시 사용
    ↓
Delta Store 데이터를 Cache로 동기화
    ↓
벡터화 실행 (SIMD)
    ↓
선택 사항: GPU 가속
    ↓
결과 반환
```

---

## 다음 단계

- [벤치마크](benchmarks) — 성능 비교 확인
- [예제](examples/quick-start) — 코드 예제 살펴보기
- [API 문서](https://docs.rs/dbx-core) — 전체 Rust API 레퍼런스
