---
layout: default
title: 한국어
nav_order: 3
has_children: true
description: "DBX — 고성능 임베디드 데이터베이스"
---

# DBX
{: .fs-9 }

5-Tier 하이브리드 스토리지 아키텍처 기반 고성능 임베디드 데이터베이스. HTAP(하이브리드 트랜잭션/분석 처리) 워크로드를 위해 설계되었으며, 순수 Rust로 구현되었습니다.
{: .fs-6 .fw-300 }

[시작하기](getting-started){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[GitHub에서 보기](https://github.com/bytelogiccore-spec/DBX){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## 주요 기능

### 🏗️ 아키텍처
- **5-Tier 하이브리드 스토리지** — Delta → Cache → Native WOS → Index → ROS
- **Native WOS** — Sled 종속성을 제거한 자체 SSTable 기반 고성능 엔진
- **HTAP 지원** — OLTP와 OLAP 워크로드 동시 처리
- **분산 그리드** — QUIC 기반 실시간 복제 및 분산 실행

### 🚀 성능
- **Fast-Path 최적화** — 단일 노드 스캔 레이턴시 **51µs** 달성 (v0.2.0-beta)
- SQLite 대비 **최대 29배 빠른** 분석 성능
- **멀티코어 병렬화** — Rayon 기반 scan, insert, 집계, JOIN 자동 병렬화
- **GPU 가속** — CUDA 기반 집계, 필터링, 조인
- **SIMD 벡터화** — `wide` crate stable SIMD (항상 활성)

### 🔐 보안
- **AES-256-GCM-SIV** — 산업 표준 암호화
- **ChaCha20-Poly1305** — 고속 모바일 암호화
- **키 교체** — 무중단 키 업데이트

### 🌐 다국어 바인딩
- **Rust** — 네이티브 API
- **Python** — PyO3 기반 바인딩
- **C#/.NET** — P/Invoke FFI
- **C/C++** — 표준 C API
- **Node.js** — 네이티브 N-API 바인딩

---

## 빠른 시작 예제

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

// CRUD
db.insert("users", b"user:1", b"Alice")?;
let val = db.get("users", b"user:1")?;

// SQL
let results = db.execute_sql("SELECT * FROM users WHERE age > 25")?;

// 트랜잭션
let tx = db.begin_transaction()?;
tx.insert("users", b"user:2", b"Bob")?;
tx.commit()?;
```
