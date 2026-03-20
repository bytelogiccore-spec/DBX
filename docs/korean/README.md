---
layout: default
title: 소개 (README)
nav_order: 0
parent: 한국어
---

# DBX — 차세대 HTAP 임베디드 데이터베이스
{: .fs-9 }

[![Version](https://img.shields.io/badge/version-0.1.2--beta-blue.svg)](https://github.com/ByteLogicCore/DBX)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://bytelogiccore-spec.github.io/DBX/)

**DBX**는 현대적인 **HTAP(Hybrid Transactional/Analytical Processing)** 워크로드를 위해 설계된 고성능 임베디드 데이터베이스입니다. 고유한 **5계층 하이브리드 스토리지(5-Tier Hybrid Storage)** 아키텍처를 통해 초고속 인메모리 트랜잭션과 대규모 컬럼형 분석의 간극을 완벽하게 메웁니다.
{: .fs-6 .fw-300 }

---

## ⚡ 왜 DBX인가? 핵심 강점

### 🚄 1. 무한 확장성 (5계층 아키텍처)
속도와 용량 사이에서 고민할 필요가 없습니다. DBX는 데이터를 5개의 특화된 계층으로 유연하게 흐르게 합니다:
- **Tier 1 (Delta)**: 밀리초 미만의 쓰기를 위한 초고속 BTreeMap.
- **Tier 2 (Cache)**: 즉각적인 OLAP 분석을 위한 Apache Arrow 기반 컬럼형 캐시.
- **Tier 3 (WOS)**: 스냅샷 격리를 위한 SSD 최적화 MVCC 저장소.
- **Tier 4 (Index)**: 레이턴시 없는 조회를 위한 고속 Bloom 필터.
- **Tier 5 (ROS)**: 페타바이트급 아카이브를 위한 고압축 Parquet 저장소.

### 🏎️ 2. 압도적인 성능
10,000건 기준 벤치마크 결과, DBX는 업계 표준을 월등히 앞섭니다:
- **파일 GET**: SQLite 대비 **29배 빠름** (17ms vs 497ms) 🔥
- **메모리 INSERT**: SQLite 대비 **1.16배 빠름** (25ms vs 29ms) ✅

### 🧠 3. 네이티브 GPU 가속
DBX는 **CUDA 가속**을 기본 지원하는 업계 최초의 임베디드 데이터베이스입니다.
- 대규모 데이터셋 필터링 및 집계 시 **최대 4.5배 가속**.
- 무거운 JOIN 및 GROUP BY 연산을 GPU로 투명하게 오프로드.

### 🛡️ 4. 잠금 없는 동시성 (MVCC)
차단(Blocking) 없이 대규모 동시 워크로드를 처리합니다.
- **스냅샷 격리**: 읽기 작업이 쓰기를 방해하지 않으며, 그 반대도 마찬가지입니다.
- **ACID 준수**: WAL 2.0을 통한 완벽한 트랜잭션 무결성 보장.

---

## 📦 현대적인 주요 기능

### 💎 구체화된 뷰 (Materialized Views)
복잡한 분석 쿼리를 미리 계산하여 즉각적인 응답을 제공합니다.
- **자동 갱신**: 백그라운드 스레드가 60초마다 결과를 최신 상태로 유지합니다.
- **투명한 캐싱**: SQL 실행 시 자동으로 캐시를 확인하여 즉시 결과를 반환합니다.

### 🌊 실시간 스트리밍 수집 (CDC)
고처리량 데이터 파이프라인을 위한 `StreamIngester`를 내장하고 있습니다.
- **MPSC 파이프라인**: 수만 개의 프로듀서로부터 동시에 데이터를 수집합니다.
- **Full DML 지원**: 실시간 INSERT, UPDATE, DELETE 처리를 지원합니다.

---

## 🏗️ 아키텍처 시각화

```mermaid
graph TD
    A[Application] -->|Write| B[Tier 1: Delta Store]
    B -->|Flush| C[Tier 2: Columnar Cache]
    C -->|Persist| D[Tier 3: WOS - SSD]
    D -->|Compact| E[Tier 4: Index]
    E -->|Archive| F[Tier 5: ROS - Parquet]
    
    G[GPU 가속] -.->|가속화| C
    G -.->|가속화| F
    
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#bbf,stroke:#333,stroke-width:2px
    style F fill:#bfb,stroke:#333,stroke-width:2px
```

---

## 🚀 빠른 시작 (Rust)

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // 메모리 또는 디스크에 데이터베이스 오픈
    let db = Database::open_in_memory()?;

    // 초고속 CRUD
    db.insert("users", b"user:123", b"{\"name\": \"Alice\", \"age\": 30}")?;
    let val = db.get("users", b"user:123")?;

    // 강력한 HTAP SQL 지원
    let results = db.execute_sql("SELECT name, AVG(age) FROM users GROUP BY name")?;

    Ok(())
}
```

---

## 🤝 후원 및 기여

DBX는 커뮤니티의 힘으로 성장하는 오픈 소스 프로젝트입니다.
- ⭐️ **Star**를 눌러 프로젝트를 응원해 주세요!
- 🐛 **이슈 제보** 및 기능 제안은 언제나 환영합니다.
- 🛠️ **기여하기**: [기여 가이드](./legal/korean/CONTRIBUTING.md)에서 빌드 최적화 팁(LLD, mold 등)을 확인하세요.

---

**데이터의 미래를 위한 Rust의 열정으로 만들어졌습니다.**
