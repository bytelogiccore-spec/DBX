---
layout: default
title: DB 설정 가이드
parent: 한국어
nav_order: 35
description: "DbConfig, DirtyBufferMode, ParallelismConfig, DurabilityLevel, EncryptionConfig — DBX 전체 설정 항목 상세 가이드"
---

# DB 설정 가이드
{: .no_toc }

소스코드 기반 DBX 전체 설정 항목 참조. `Database::open()` / `Database::open_with_config()` 두 가지 생성 방법을 설명합니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 데이터베이스 생성 방법

### 방법 1: `Database::open()` — 기본 설정

```rust
use dbx_core::Database;
use std::path::Path;

// 파일 기반 — 기본 설정 (모든 코어, 1000행 임계값, DurabilityLevel::Full)
let db = Database::open(Path::new("./data"))?;

// 인메모리 — 테스트/캐시용
let db = Database::open_in_memory()?;
```

### 방법 2: `Database::open_with_config()` — 커스텀 설정

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig};
use std::path::Path;

let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig {
            cpu_cap: 0.75,
            min_rows_for_parallel: 2_000,
        },
    },
)?;
```

### 방법 2b: `Database::open_encrypted()` — 암호화 DB

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// 패스워드 기반 (HKDF-SHA256으로 256-bit 키 유도)
let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted(Path::new("./data"), enc)?;

// 알고리즘 지정
let enc = EncryptionConfig::from_password("pw")
    .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);
let db = Database::open_encrypted(Path::new("./data"), enc)?;
```

---

## `DbConfig` 구조체

> **파일**: `src/engine/parallel_engine.rs`

```rust
pub struct DbConfig {
    pub parallelism: ParallelismConfig,
    pub sync: RealtimeSyncConfig,
    pub replication: ReplicationConfig,
    pub dirty_buffer_mode: DirtyBufferMode,  // v0.1.2-beta 추가
}
```

---

## `DirtyBufferMode` — WOS dirty 버퍼 자료구조 선택

> **파일**: `src/engine/parallel_engine.rs`  
> **추가**: v0.1.2-beta

WOS Tier 3의 `dirty` 버퍼(flush 전 임시 메모리 저장소)에 사용할 자료구조를 선택합니다.

| 값 | 기본 | 특징 | 권장 워크로드 |
|----|:----:|------|--------------|
| `BTreeMap` | ✅ | 키 정렬 유지 → `scan(range)` 효율적 | 범위 쿼리 많은 OLAP / 일반 OLTP |
| `DashMap` | | 샤드 락 → 다중 스레드 동시 접근 효율적 | 단순 get/insert 위주, 쓰기 폭발적 |

### 전환 안전성

`dirty`는 디스크에 저장되지 않는 순수 인메모리 버퍼입니다.  
재시작 시 항상 빈 상태로 시작하므로, 두 모드 간 자유롭게 전환해도 **기존 데이터 손상 없음**.

```
프로그램 시작
  └─ dirty = 빈 상태 (항상, BTreeMap/DashMap 무관)
       ↓
  WAL 파일에서 wal_entries 복구
       ↓
  SSTable 인덱스 로드
```

### 사용 예시

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, DirtyBufferMode};

// DashMap 모드 (단순 get/insert 위주 고성능 워크로드)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        dirty_buffer_mode: DirtyBufferMode::DashMap,
        ..Default::default()
    },
)?;

// 기본값 (BTreeMap — 범위 쿼리 최적화)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig::default(),
)?;
```

### DashMap 모드의 트레이드오프

| 연산 | BTreeMap | DashMap |
|------|:--------:|:-------:|
| `get(key)` 단순 조회 | 보통 | 빠름 |
| `scan(range)` 범위 쿼리 | **빠름** | 느림 (전체 순회 후 정렬) |
| 동시 다중 스레드 접근 | 순차 | **동시** |
| compact 시 정렬 비용 | 없음 | 있음 (내부 정렬 1회) |

> **참고**: `dirty`는 flush 전 임시 버퍼이므로 데이터가 쌓이기 전에 WAL로 이동합니다.  
> SSTable 범위 쿼리(`scan`)에는 영향 없음 — SSTable은 항상 정렬된 상태.

---

## `ParallelismConfig` — 병렬화 제어

> **파일**: `src/engine/parallel_engine.rs`

### 필드

| 필드 | 타입 | 기본값 | 설명 |
|------|------|--------|------|
| `cpu_cap` | `f64` | `1.0` | 사용할 CPU 코어 비율 (0.01 ~ 1.0) |
| `min_rows_for_parallel` | `usize` | `1_000` | 병렬화 활성화 최소 행 수 |

### `cpu_cap` 상세

```
실제 스레드 수 = ceil(base_threads × cpu_cap).max(1)
base_threads   = min(num_cpus, 16)   // Auto 정책 기준
```

| `cpu_cap` | 12코어 PC 스레드 수 |
|-----------|------------------|
| `1.0` (기본) | 12 |
| `0.75` | 9 |
| `0.5` | 6 |
| `0.25` | 3 |

### `min_rows_for_parallel` 상세

이 값보다 적은 데이터는 스레드 풀 오버헤드 없이 단일 스레드로 처리됩니다.

```
500행 → aggressive 수준, 작은 배치도 병렬화
1,000행 → 기본값, 균형
5,000행 → conservative 수준, 큰 배치만 병렬화
```

### 프리셋

```rust
// CPU 50%, 5,000행 임계값 — PC 부하 최소화
ParallelismConfig::conservative()

// CPU 100%, 500행 임계값 — 최대 성능
ParallelismConfig::aggressive()

// CPU 100%, 1,000행 임계값 — 기본값
ParallelismConfig::default()
```

---

## `ParallelizationPolicy` — 스레드 풀 정책

> **파일**: `src/engine/parallel_engine.rs`

`Database::open_with_config()`는 내부적으로 `ParallelizationPolicy::Auto`를 사용합니다.
저수준 직접 제어가 필요한 경우 `ParallelExecutionEngine`을 직접 생성할 수 있습니다.

| 정책 | 설명 | 스레드 수 |
|------|------|----------|
| `Auto` | 시스템 코어 수 기반 자동 (기본값) | `min(num_cpus, 16)` |
| `Fixed(n)` | 고정 스레드 수 | `n` (0이면 에러) |
| `Adaptive` | 절반에서 시작, 워크로드에 따라 조정 | `(num_cpus / 2).max(1)` |

```rust
use dbx_core::engine::parallel_engine::{
    ParallelExecutionEngine, ParallelizationPolicy
};

// 정확히 4스레드 고정
let engine = ParallelExecutionEngine::new_fixed(4)?;

// Auto (시스템 최적)
let engine = ParallelExecutionEngine::new_auto()?;
```

---

## `DurabilityLevel` — WAL 내구성 수준

> **파일**: `src/engine/types.rs`

WAL 쓰기 동기화 정책을 제어합니다. **현재 `Database::open()`에서는 `Full`이 기본값**입니다.

| 값 | 설명 | fsync | 성능 | 안전성 |
|----|------|-------|------|--------|
| `Full` | 모든 쓰기에 즉시 fsync | 매 쓰기 | 낮음 | 최대 안전 |
| `Lazy` | 백그라운드에서 지연 fsync | 비동기 | 중간 | 중간 |
| `None` | WAL 기록 없음 (메모리 전용) | 없음 | 최대 | 크래시 시 손실 |

> **주의**: `DurabilityLevel::None`은 벤치마크 또는 캐시 전용 DB에만 사용하세요.

---

## WAL 내부 상수 (소스코드 기준)

> **파일**: `src/storage/native_wos/table_store.rs`

직접 변경은 불가능하지만(컴파일 타임 상수), 동작에 영향을 주는 고정 값들입니다.

| 상수 | 값 | 의미 |
|------|-----|------|
| `WAL_COMPACT_THRESHOLD` | `5_000` | WAL 항목이 5,000개를 넘으면 compact() 자동 호출 |
| `PAGE_TARGET_BYTES` | `4_096` | SSTable 페이지 목표 크기 (4KB) |
| `FOOTER_SIZE` | `16` bytes | SSTable footer 크기 |

**compact 동작**:
```
flush() 호출 시:
  WAL 항목 수 < 5,000  → .wal 파일에 sequential append (빠름)
  WAL 항목 수 >= 5,000 → compact() 발동: WAL + SSTable 병합 후 재작성
```

---

## `EncryptionConfig` — 암호화 설정

> **파일**: `src/storage/encryption/config.rs`

### 지원 알고리즘

| 알고리즘 | 기본값 | 특징 | 권장 환경 |
|---------|--------|------|---------|
| `Aes256GcmSiv` | ✅ | AES-NI 하드웨어 가속, Nonce 오용 저항 | 서버·데스크탑·최신 모바일 |
| `ChaCha20Poly1305` | | 소프트웨어 고성능, 상수 시간 구현 | 임베디드·AES-NI 미지원 |

### 생성 방법

```rust
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// 1. 패스워드 기반 (HKDF-SHA256 → 256-bit 키)
let config = EncryptionConfig::from_password("my-password");

// 2. 원시 256-bit 키 (32바이트)
let raw_key: [u8; 32] = [0x42u8; 32];
let config = EncryptionConfig::from_key(raw_key);

// 3. 알고리즘 지정
let config = EncryptionConfig::from_password("pw")
    .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);

// 4. 키 + 알고리즘 동시 지정
let config = EncryptionConfig::from_key_with_algorithm(
    raw_key,
    EncryptionAlgorithm::ChaCha20Poly1305,
);
```

### 와이어 포맷

```
암호화 출력 = [nonce 12바이트] || [ciphertext + auth_tag]
            = self-contained (외부 nonce 저장 불필요)
```

---

## `WosVariant` — WOS 백엔드 선택

> **파일**: `src/engine/wos_variant.rs`

데이터베이스 생성 시 자동으로 선택됩니다. 직접 선택 불필요.

| 변형 | 언제 사용 | 특징 |
|------|---------|------|
| `Native` | `Database::open()` | Native SSTable + WAL, 파일 영구 저장 |
| `InMemory` | `open_in_memory()` | 메모리 전용, 빠름, 재시작 시 초기화 |
| `Encrypted` | `open_encrypted()` | Native + AES/ChaCha20 암호화 |

---

## `DeltaVariant` — Delta Store 구현체

> **파일**: `src/engine/delta_variant.rs`

Tier 1 (Delta Store)의 내부 구현체입니다.

| 변형 | 설명 |
|------|------|
| `RowBased(DeltaStore)` | BTreeMap 기반 행 저장소 (기본값) |
| `Columnar(ColumnarDelta)` | Apache Arrow RecordBatch 기반 컬럼형 저장소 |

---

## `TablePersistence` — 테이블 지속성

> **파일**: `src/engine/types.rs`

테이블별 저장소 타입을 지정합니다.

| 값 | 설명 |
|----|------|
| `Memory` | 프로세스 종료 시 삭제 (캐시, 임시 데이터) |
| `File` | 파일 시스템에 영구 저장 |

---

## 전체 설정 예시

```rust
use dbx_core::Database;
use dbx_core::engine::parallel_engine::{DbConfig, ParallelismConfig, DirtyBufferMode};
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};

// 고성능 서버 환경 (모든 코어, aggressive 병렬화)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig::aggressive(),
        ..Default::default()
    },
)?;

// 보수적 환경 (CPU 절반, 큰 배치만 병렬화)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        parallelism: ParallelismConfig::conservative(),
        ..Default::default()
    },
)?;

// 단순 get/insert 위주 고성능 (DashMap 모드)
let db = Database::open_with_config(
    Path::new("./data"),
    DbConfig {
        dirty_buffer_mode: DirtyBufferMode::DashMap,
        parallelism: ParallelismConfig::aggressive(),
        ..Default::default()
    },
)?;

// 암호화 DB
let enc = EncryptionConfig::from_password("secret");
let db = Database::open_encrypted(Path::new("./data"), enc)?;

// 인메모리 (벤치마크 / 테스트)
let db = Database::open_in_memory()?;
```

---

## 설정별 권장 시나리오 요약

| 시나리오 | 권장 설정 |
|---------|---------|
| 전용 서버 분석 쿼리 | `open_with_config` + `aggressive()` |
| 범위 쿼리 많은 OLAP | `open_with_config` + `DirtyBufferMode::BTreeMap` (기본값) |
| 단순 get/insert 고처리량 | `open_with_config` + `DirtyBufferMode::DashMap` |
| 개발 PC / 노트북 | `open_with_config` + `conservative()` |
| 일반 OLTP 서버 | `open()` (기본값) |
| 데이터 보호 필요 | `open_encrypted()` |
| 테스트 / 캐시 | `open_in_memory()` |
| 벤치마크 (최대 성능) | `open_in_memory()` 또는 `DurabilityLevel::None` |

---

## 다음 단계

- [병렬 쿼리 가이드](parallel-query) — SQL 병렬 집계 상세
- [암호화 가이드](encryption) — EncryptionConfig 심화
- [WAL 복구 가이드](wal-recovery) — WAL 동작 원리
- [스토리지 레이어](storage-layers) — 5-Tier 아키텍처
