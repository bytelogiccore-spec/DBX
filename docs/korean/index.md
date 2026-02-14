---
layout: default
title: 홈
nav_order: 1
description: "DBX — 고성능 임베디드 데이터베이스"
permalink: /
---

# DBX
{: .fs-9 }

4-Tier 하이브리드 스토리지 아키텍처 기반 고성능 임베디드 데이터베이스. HTAP(하이브리드 트랜잭션/분석 처리) 워크로드를 위해 설계되었으며, 순수 Rust로 구현되었습니다.
{: .fs-6 .fw-300 }

[시작하기](getting-started){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[GitHub에서 보기](https://github.com/ByteLogicCore/DBX){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## 주요 기능

### 🏗️ 아키텍처
- **4-Tier 하이브리드 스토리지** — Delta → Cache → WOS → ROS
- **HTAP 지원** — OLTP와 OLAP 워크로드 동시 처리
- **MVCC 트랜잭션** — 가비지 컬렉션을 포함한 스냅샷 격리
- **컬럼형 캐시** — Apache Arrow 기반 분석 쿼리 최적화

### ⚡ 성능
- **GPU 가속** — CUDA 기반 집계 및 필터링 (최대 4.57배 빠름)
- **쿼리 최적화** — Projection Pushdown, Predicate Pushdown
- **제로 카피 연산** — Arrow RecordBatch 직접 활용
- **벡터화 실행** — SIMD 벡터화 연산

### 🔒 보안 및 안정성
- **암호화** — AES-256-GCM-SIV, ChaCha20-Poly1305
- **압축** — ZSTD, Brotli
- **WAL 2.0** — Bincode 바이너리 직렬화 및 비동기 fsync
- **ACID** — 완전한 트랜잭션 보장 및 크래시 복구

### 🎯 개발자 경험
- **순수 Rust** — 메모리 안전성 보장
- **SQL 지원** — SELECT, WHERE, JOIN, GROUP BY, ORDER BY
- **임베디드** — 별도 서버 불필요
- **철저한 테스트** — 100개 이상의 통합 테스트

---

## 빠른 예제

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // 데이터베이스 열기
    let db = Database::open("./data")?;
    
    // 데이터 삽입
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    
    // 데이터 조회
    let value = db.get("users", b"user:1")?;
    assert_eq!(value, Some(b"Alice".to_vec()));
    
    Ok(())
}
```

---

## 성능 하이라이트

| 연산 | CPU | GPU | 속도 향상 |
|-----------|-----|-----|---------|
| SUM | 456.66µs | 783.36µs | 0.58x |
| Filter (>500K) | 2.06ms | 673.38µs | **3.06x** |

*1,000,000행 기준 벤치마크. GPU는 더 큰 데이터셋(>10M 행)에서 더 큰 성능 향상을 보입니다.*

---

## 문서

### 📚 가이드

포괄적인 기능 가이드:

- **[CRUD 작업](guides/crud-operations)** — 완전한 CRUD 가이드
- **[SQL 레퍼런스](guides/sql-reference)** — 전체 SQL 문법 레퍼런스
- **[트랜잭션](guides/transactions)** — MVCC 및 스냅샷 격리
- **[GPU 가속](guides/gpu-acceleration)** — CUDA 기반 쿼리 가속
- **[암호화](guides/encryption)** — AES-256 및 ChaCha20 암호화
- **[압축](guides/compression)** — ZSTD 압축
- **[WAL 복구](guides/wal-recovery)** — Write-Ahead Logging 및 크래시 복구

### 🎓 튜토리얼

DBX 학습을 위한 단계별 튜토리얼:

- **[초보자 튜토리얼](tutorials/beginner)** — 첫 DBX 데이터베이스

### 📖 예제

실용적인 코드 예제:

- **[빠른 시작](examples/quick-start)** — 5분 시작 가이드
- **[SQL 빠른 시작](examples/sql-quick-start)** — SQL 기본 사용법
- **[암호화](examples/encryption)** — 데이터 암호화
- **[압축](examples/compression)** — 데이터 압축
- **[WAL 복구](examples/wal-recovery)** — 크래시 복구

### 🔧 API 레퍼런스

완전한 API 문서:

- **[Database API](api/database)** — 핵심 데이터베이스 작업
- **[Transaction API](api/transaction)** — 트랜잭션 관리
- **[SQL API](api/sql)** — SQL 실행

### 🗺️ 로드맵

- **[로드맵](roadmap)** — 향후 기능 및 개발 계획

---

## 시작하기

준비되셨나요? [시작 가이드](getting-started)를 확인하여 DBX를 설치하고 첫 쿼리를 실행해보세요.

자세한 아키텍처 정보는 [아키텍처 가이드](architecture)를 참조하세요.

---

## 라이선스

MIT OR Apache-2.0
