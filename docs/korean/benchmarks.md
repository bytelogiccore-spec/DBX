---
layout: default
title: 벤치마크
nav_order: 3
parent: 한국어
description: "DBX 성능 벤치마크"
---

# 벤치마크
{: .no_toc }

DBX와 다른 임베디드 데이터베이스 간의 성능 비교 벤치마크입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 요약 (Executive Summary)

DBX는 순수 Rust로 작성된 고성능 임베디드 데이터베이스 엔진입니다.

### 최신 벤치마크 결과 (v0.2.0-beta, 10,000개 레코드)

| 작업 | DBX (Fast-Path) | SQLite | Sled | Redb | 순위 |
|------|-----|--------|------|------|------|
| **SCAN (Count)** | **51µs** 🥇 | 340µs | 4.64ms | 2.08ms | **1위** |
| **INSERT** | **25.21ms** 🥇 | 29.4ms | 56.6ms | 54.05ms | **1위** |
| **GET** | **2.84ms** 🥇 | 33.8ms | 5.88ms | 2.96ms | **1위** |

> **SCAN 참고**: v0.2.0에서 도입된 **Fast-Path** 기술을 통해 단일 노드 쿼리 레이턴시를 마이크로초(µs) 단위로 단축했습니다.

> **SCAN 참고**: Redb는 mmap + B-tree 특화 구조로 KV 순회에서 최상위. DBX는 SQL·컬럼형 집계에 특화.

### v0.1.1 병렬화 적용 후 개선폭

| 항목 | 개선폭 | 원인 |
|------|--------|------|
| `dbx_scan_10k` | **-29.2%** | `rayon::join()` Delta+WOS 병렬 스캔 |
| `dbx_get_10k` | **-6.8%** | 최적화 누적 효과 |

**버전**: DBX v0.2.0-beta
**테스트 날짜**: 2026년 4월 3일
**보고서 유형**: 공식 성능 비교 분석


---

## 테스트 환경

### 하드웨어 사양

| 항목 | 사양 |
|------|------|
| **운영체제** | Microsoft Windows 11 Pro (Build 26200) |
| **시스템 유형** | x64 기반 PC |
| **프로세서** | 1 Processor (Multiprocessor Free) |
| **메모리** | 16,273 MB (약 16GB) |

### 소프트웨어 환경

| 컴포넌트 | 버전 |
|-----------|------|
| **Rust Compiler** | rustc 1.92.0 (ded5c06cf 2025-12-08) |
| **Cargo** | 1.92.0 (344c4567c 2025-10-21) |
| **빌드 프로필** | `release` (최적화 활성화) |
| **벤치마크 프레임워크** | Criterion.rs v0.5 |

---

## 테스트 대상 데이터베이스

| 데이터베이스 | 버전 | 언어 | 특징 |
|------------|------|------|------|
| **DBX** | 0.0.6-beta | Pure Rust | 5-Tier Hybrid Storage, MVCC |
| **SQLite** | 0.32 (rusqlite) | C (bundled) | 업계 표준 임베디드 DB |
| **Sled** | 0.34 | Pure Rust | Lock-free B+ tree |
| **Redb** | 2.1 | Pure Rust | LMDB 영감, 파일 전용 |

---

## 벤치마크 방법론

### 측정 프레임워크

- **도구**: Criterion.rs v0.5 (Rust 표준 벤치마킹 라이브러리)
- **샘플 수**: 테스트당 100회 반복
- **웜업 (Warmup)**: 각 테스트 전 3초간 실행
- **통계 분석**: 평균, 표준 편차, 95% 신뢰 구간
- **이상치 감지**: 자동 이상치 제거 및 보고

### 공정한 비교 조건

#### DBX 설정

```rust
// Default features 활성화
features = ["wal", "mvcc", "index"]

// 벤치마크용 durability 비활성화 (공정한 비교)
durability = DurabilityLevel::None
```

#### 모든 데이터베이스 공통 설정

1. **트랜잭션/배치 모드**
   - 개별 INSERT 대신 배치 커밋을 사용한 공정한 비교
   - DBX: `begin()` → `insert()` × N → `commit()`
   - SQLite: `unchecked_transaction()` → `execute()` × N → `commit()`
   - Sled: `insert()` × N → `flush()`
   - Redb: `begin_write()` → `insert()` × N → `commit()`

2. **WAL (Write-Ahead Logging) 비활성화**
   - DBX: `durability = DurabilityLevel::None`
   - SQLite: `PRAGMA synchronous = OFF`
   - Sled: 기본 설정 (flush 기반)
   - Redb: 기본 설정 (트랜잭션 기반)

3. **동일한 데이터 크기**
   - Key: `"key_{i}"` 형식의 문자열
   - Value: `"value_data_{i}"` 형식의 문자열
   - 테스트 규모: 10,000개 레코드

---

## 상세 벤치마크 결과

### INSERT 성능 (10,000개 레코드)

| 데이터베이스 | 평균 시간 | 표준 편차 | 처리량 (rec/sec) | DBX 대비 |
|------------|----------|----------|------------------|----------|
| **DBX** | **44.92ms** | ±0.20ms | **222,619** | **1.0× (기준)** |
| SQLite | 53.06ms | ±0.38ms | 188,465 | **0.85× (18% 느림)** |
| Redb | 54.05ms | ±0.72ms | 185,015 | **0.83× (20% 느림)** |
| Sled | 60.56ms | ±1.55ms | 165,123 | **0.74× (35% 느림)** |

**DBX 우위**:
- ✅ **모든 경쟁사보다 빠름**
- ✅ SQLite 대비 18% 빠름
- ✅ 안정적인 성능 (낮은 표준 편차)

### GET 성능 (10,000개 레코드)

| 데이터베이스 | 평균 시간 | 표준 편차 | 처리량 (rec/sec) | DBX 대비 |
|------------|----------|----------|------------------|----------|
| **DBX** | **2.84ms** | ±0.01ms | **3,521,127** | **1.0× (기준)** |
| Redb | 3.25ms | ±0.17ms | 3,076,923 | **0.87× (14% 느림)** |
| Sled | 5.88ms | ±0.03ms | 1,700,680 | **0.48× (107% 느림)** |
| SQLite | 37.39ms | ±0.48ms | 267,452 | **0.08× (1,217% 느림)** |

**DBX 우위**:
- ✅ **SQLite 대비 13배 빠름**
- ✅ Redb 대비 14% 빠름
- ✅ Sled 대비 2배 빠름

### SCAN 성능 (10,000개 레코드)

| 데이터베이스 | 평균 시간 | 표준 편차 | 처리량 (rec/sec) | DBX 대비 |
|------------|----------|----------|------------------|----------|
| **DBX** | **1.60ms** | ±0.07ms | **6,250,000** | **1.0× (기준)** |
| Redb | 2.15ms | ±0.02ms | 4,651,163 | **0.74× (34% 느림)** |
| SQLite | 2.98ms | ±0.16ms | 3,355,705 | **0.54× (86% 느림)** |
| Sled | 4.64ms | ±0.10ms | 2,155,172 | **0.34× (190% 느림)** |

**DBX 우위**:
- ✅ **모든 경쟁사보다 빠름**
- ✅ Redb 대비 34% 빠름
- ✅ SQLite 대비 86% 빠름

---

## 성능 최적화 기술

### Phase 1: GET 최적화 (+70% 개선)

1. **Inline 속성 추가** - `#[inline(always)]`로 함수 호출 오버헤드 제거
2. **MVCC 오버헤드 제거** - Hot path에서 불필요한 타임스탬프 획득 비용 절감
3. **코드 경로 단순화** - 분기 예측 개선으로 CPU 파이프라인 효율 향상

**결과**: 9.63ms → 2.84ms (3.4배 빠름)

### Phase 2: SCAN 최적화 (+57% 개선)

1. **Delta Store Fast-path** - Delta가 비어있을 때 조기 반환
2. **2-way Merge 최적화** - Merge 오버헤드 완전 제거
3. **캐시 지역성 개선** - 단일 스캔으로 메모리 접근 패턴 최적화

**결과**: 3.70ms → 1.60ms (2.3배 빠름)

### Phase 3: 멀티코어 병렬화 (v0.1.1, P1~P9)

| 항목 | 기법 | 임계값 |
|------|------|--------|
| INSERT batch | `par_iter()` | 1,000행 이상 |
| SCAN | `rayon::join()` Delta+WOS | 항상 |
| GROUP BY 집계 | `par_iter()` 그룹별 독립 계산 | 1,000 그룹 이상 |
| JOIN Build/Probe | `into_par_iter()` | 1,000행 이상 |
| WAL encode | `par_iter()` (파일 쓰기는 순차) | 500건 이상 |
| compact() 역직렬화 | `par_iter()` (파일 읽기는 순차) | 4 페이지 이상 |
| SIMD | `wide` crate stable (항상 활성) | - |

**결과**: scan -29.2%, get -6.8% 추가 개선

### Phase 9: Fast-Path 기술 (v0.2.0)

1. **Local Bypass**: 분산 DAG 스케줄링 오버헤드를 걷어내고 로컬 실행기로 직접 연결
2. **Sync Data Stream**: mpsc 채널을 우회하여 메모리 데이터를 동기식으로 즉시 반환 (`sync_batches`)
3. **Lazy Setup**: 쿼리 시작 시 발생하는 불필요한 I/O(디렉토리 스캔 등) 제거

**결과**: scan 수치 340µs → 51µs (6.6배 향상)

---

## 아키텍처 강점

### 5-Tier Hybrid Storage

1. **Delta Store (Tier 1)**
   - DashMap + SkipMap (Lock-free)
   - 초고속 INSERT 성능

2. **WOS (Tier 2)**
   - BTreeMap (정렬된 저장소)
   - 효율적인 범위 스캔

3. **최적화된 데이터 흐름**
   - Delta → WOS 자동 flush
   - 메모리와 디스크의 효율적 활용

---

## 벤치마크 재현 방법

```bash
# 프로젝트 클론
git clone https://github.com/ByteLogicCore/DBX.git
cd DBX

# 전체 비교 벤치마크 실행
cargo bench -p dbx-benchmarks --bench official_db_comparison

# 개별 데이터베이스 벤치마크
cargo bench -p dbx-benchmarks --bench official_db_comparison -- dbx_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- sqlite_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- sled_
cargo bench -p dbx-benchmarks --bench official_db_comparison -- redb_
```

---

## 결론

DBX v0.0.6-beta는 **모든 주요 작업(INSERT, GET, SCAN)에서 1위를 달성**했습니다.

### 핵심 성과

- ✅ **INSERT 1위**: 44.92ms (경쟁사 대비 18-35% 빠름)
- ✅ **GET 1위**: 2.84ms (SQLite 대비 13배 빠름)
- ✅ **SCAN 1위**: 1.60ms (경쟁사 대비 34-190% 빠름)

### 기술적 차별점

1. **5-Tier Hybrid Storage**: 메모리와 디스크의 효율적 활용
2. **Lock-Free 아키텍처**: DashMap + SkipMap
3. **Pure Rust**: 메모리 안전성과 제로 비용 추상화
4. **최적화된 알고리즘**: Fast-path 및 inline 최적화

DBX는 **고성능 쓰기 작업과 균형 잡힌 읽기 성능이 필요한 애플리케이션에 최적의 선택**입니다.

---

## 다음 단계

- [아키텍처](architecture) — 5-Tier 하이브리드 스토리지 이해하기
- [시작하기](getting-started) — 직접 사용해보기
- [GPU 가속](guides/gpu-acceleration) — 분석 쿼리 가속화
- [예제](examples/quick-start) — 코드 예제 살펴보기
