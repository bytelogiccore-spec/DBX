---
layout: default
title: 저장소 계층
parent: Guides
nav_order: 4
description: "DBX의 5계층 하이브리드 저장소 아키텍처 이해"
---

# 저장소 계층
{: .no_toc }

DBX의 5계층 하이브리드 저장소(5-Tier Hybrid Storage) 아키텍처에 대한 상세 가이드입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 아키텍처 개요

DBX는 트랜잭션(OLTP)과 분석(OLAP) 워크로드를 모두 최적화하기 위해 설계된 정교한 **5계층 하이브리드 저장소** 구조를 사용합니다.

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← 인메모리 쓰기 버퍼 (Hot Data)
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← 분석용 컬럼형 캐시
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (Write-Optimized Store)    │  ← 영구 트랜잭션 저장소 (sled 기반)
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← 빠른 존재 확인 인덱스
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Read-Optimized Store)     │  ← 최종 컬럼형 압축 저장소 (Parquet)
└─────────────────────────────────────────┘
```

---

## 각 계층별 상세

### Tier 1: Delta Store
- **목적**: 초고속 데이터 삽입 및 핫 데이터 캐싱
- **특징**: `BTreeMap` 기반, Lock-free 읽기 지원, 자동 플러시 기능을 가짐

### Tier 2: Columnar Cache
- **목적**: OLAP 쿼리 최적화
- **특징**: Apache Arrow `RecordBatch` 형식 사용, SIMD 가속 및 제로 카피 연산 지원

### Tier 3: WOS (Write-Optimized Store)
- **목적**: 영구적인 트랜잭션 저장 및 ACID 보장
- **특징**: `sled` 기반의 KV 저장소, MVCC 스냅샷 격리 구현

### Tier 4: Index (Bloom Filter)
- **목적**: 불필요한 디스크 I/O 최소화
- **특징**: 데이터 존재 여부를 확률적으로 빠르게 확인 (O(1)), 찾지 못한 경우 ROS 조회를 생략함

### Tier 5: ROS (Read-Optimized Store)
- **목적**: 대규모 데이터의 장기 보관 및 압축 조회
- **특징**: Apache Parquet 형식, 높은 압축률 (ZSTD, Snappy), 컬럼 기반 조회 최적화

---

## 데이터 흐름 (Data Flow)

### 쓰기 경로 (Write Path)
1. 사용자가 데이터를 입력하면 **Delta Store**에 먼저 기록됩니다.
2. 설정된 임계치에 도달하면 **WOS**로 자동 플러시(Flush)됩니다.
3. 데이터가 충분히 쌓이면 가비지 컬렉션과 함께 **ROS**로 컴팩션(Compaction)됩니다.

### 읽기 경로 (Read Path)
1. **Delta Store** 확인 (메모리)
2. **WOS** 확인 (영구 KV)
3. **Bloom Filter** 확인 (존재 가능성 판단)
4. **ROS** 확인 (최종 저장소)

---

## 다음 단계

- [CRUD 작업](crud-operations) — 기본 데이터베이스 작업
- [트랜잭션 가이드](transactions) — MVCC 트랜잭션 상세
- [SQL 레퍼런스](sql-reference) — 분석 쿼리 최적화
