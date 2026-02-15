---
layout: default
title: GPU 가속
parent: 한국어
nav_order: 24
description: "DBX의 분석 쿼리를 위한 CUDA 기반 GPU 가속"
---

# GPU 가속
{: .no_toc }

CUDA를 사용한 DBX의 GPU 가속에 대한 전체 가이드입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX는 분석 쿼리를 위해 선택적으로 **CUDA 기반 GPU 가속**을 지원하며, 대규모 데이터셋에 대해 상당한 성능 향상을 제공합니다.

### 성능 향상 (1000만 행 기준)

| 작업 | CPU 시간 | GPU 시간 | 속도 향상 |
|-----------|----------|----------|---------|
| 합계 (SUM) | 4.5ms | 1.2ms | **3.75배** |
| 필터링 (Filter) | 20ms | 4.4ms | **4.57배** |
| GROUP BY | 35ms | 12ms | **2.92배** |
| Hash Join | 50ms | 18ms | **2.78배** |

> **참고**: GPU 가속은 데이터셋이 클수록(1000만 행 이상) 더 큰 효과를 발휘합니다.

---

## 설치 및 설정

### 1. Cargo.toml 설정

```toml
[dependencies]
dbx-core = { version = "0.1.0", features = ["gpu"] }
```

### 2. 빌드

```bash
cargo build --features gpu --release
```

---

## 기본 사용법

### GPU 가속 사용

데이터를 GPU 캐시로 동기화한 후 분석 작업을 수행합니다.

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // 테이블 데이터를 GPU 캐시로 동기화
    db.sync_gpu_cache("orders")?;

    if let Some(gpu) = db.gpu_manager() {
        // GPU 가속 합계 계산
        let total = gpu.sum("orders", "amount")?;
        println!("합계: {}", total);
    }

    Ok(())
}
```

---

## 해시 전략 (Hash Strategies)

DBX는 작업 특성에 따라 세 가지 GPU 해시 전략을 제공합니다:

| 전략 | 특징 | 권장 사례 |
|----------|-------------|--------|
| **Linear** | 안정적, 낮은 오버헤드 | 소규모 데이터셋 |
| **Cuckoo** | 최고 성능, 높은 메모리 사용 | 대규모 데이터셋 (추천) |
| **Robin Hood** | 균형 잡힌 성능 | 일반적인 워크로드 |

```rust
use dbx_core::gpu::HashStrategy;
db.set_gpu_hash_strategy(HashStrategy::Cuckoo)?;
```

---

## 샤딩 전략 (Sharding Strategies)

멀티 GPU 환경에서 데이터를 분산하기 위한 세 가지 전략을 제공합니다:

| 전략 | 동작 방식 | 권장 사례 |
|------|-----------|----------|
| **RoundRobin** | 순차적으로 디바이스에 행 분배 | 균등 분배 필요 시 |
| **Hash** | 첫 번째 컬럼 해시값 기반 분배 (ahash) | GROUP BY, JOIN 쿼리 |
| **Range** | 연속된 행 범위를 디바이스에 할당 | 정렬된 데이터, 범위 스캔 |

```rust
use dbx_core::storage::gpu::ShardingStrategy;

let manager = ShardManager::new(device_count, ShardingStrategy::Hash);
let shards = manager.shard_batch(&batch)?;
```

---

## PTX Persistent Kernel

NVRTC를 사용하여 CUDA C 커널을 런타임에 PTX로 컴파일합니다. 커널은 GPU에서 상주하며 work queue를 지속적으로 처리합니다.

```rust
use dbx_core::storage::gpu::persistent::PersistentKernelManager;

let manager = PersistentKernelManager::new(device.clone());
manager.compile_kernel()?;

if let Some(func) = manager.get_kernel_function() {
    // 커널 실행
}
```

> **참고**: `gpu` feature 활성화 시에만 사용 가능합니다. `cudarc` 0.19.2 기준으로 Unified Memory 및 P2P 접근은 지원되지 않으며, 호스트 메모리 + 명시적 전송을 사용합니다.

---

## CUDA 스트림 관리

`fork_default_stream()`을 통해 병렬 GPU 작업을 위한 별도 스트림을 생성합니다:

```rust
use dbx_core::engine::stream::GpuStreamContext;

let ctx = GpuStreamContext::new(device.clone())?;
// 별도 스트림에서 비동기 GPU 작업 실행
```

---

## SQL 통합

GPU 기능이 활성화되면 호환되는 SQL 작업은 자동으로 GPU를 사용합니다:

```rust
// 다음 SQL 작업들은 자동으로 GPU 가속을 사용합니다.
let results = db.execute_sql("
    SELECT city, SUM(amount)
    FROM orders
    WHERE amount > 1000
    GROUP BY city
")?;
```

---

## 다음 단계

- [SQL 레퍼런스](sql-reference) — SQL 쿼리에서 GPU 가속 활용
- [아키텍처 가이드](../architecture) — 데이터 흐름 이해
- [벤치마크](../benchmarks) — 상세 성능 비교 확인
