---
layout: default
title: 소개 (README)
nav_order: 0
parent: 한국어
---

# DBX — 고성능 임베디드 데이터베이스
{: .fs-9 }

SQLite보다 29배 빠른 파일 GET 속도 • 순수 Rust 구현 • GPU 가속 지원 • MVCC 트랜잭션
{: .fs-6 .fw-300 }

**DBX**는 현대적인 HTAP(Hybrid Transactional/Analytical Processing) 워크로드를 위해 설계된 **5계층 하이브리드 스토리지(5-Tier Hybrid Storage)** 아키텍처 기반의 차세대 임베디드 데이터베이스입니다.

---

## 💖 프로젝트 후원하기

DBX가 유용하다고 생각하신다면 개발을 지원해 주세요!

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/Q5Q41TDHWG)

여러분의 후원은 다음에 사용됩니다:
- 🚀 새로운 기능 추가 및 성능 최적화
- 🐛 버그 수정 및 안정성 향상
- 📚 문서화 및 튜토리얼 제작
- 💻 테스트 인프라 및 CI/CD 유지보수

---

## ⚡ 왜 DBX인가요?

### 🏆 압도적인 성능

**최신 벤치마크 결과 (10,000건 기준):**

| 항목 | DBX | SQLite | 성능 향상 |
|-----------|-----|--------|---------|
| **메모리 INSERT** | 25.37 ms | 29.50 ms | **1.16배 빠름** ✅ |
| **파일 GET** | 17.28 ms | 497.64 ms | **28.8배 빠름** 🔥🔥🔥 |

### 🎯 주요 장점

- **🚀 5계층 하이브리드 스토리지** — OLTP와 OLAP 워크로드 모두에 최적화
- **🎮 GPU 가속** — CUDA 기반 분석 연산 (필터링 최대 4.5배 가속)
- **🔒 MVCC 트랜잭션** — 잠금 없는 읽기를 위한 스냅샷 격리
- **💾 컬럼형 캐시** — Apache Arrow 기반의 쿼리 최적화
- **🔐 엔터프라이즈급 보안** — AES-256-GCM-SIV 암호화, ZSTD 압축 지원
- **🦀 순수 Rust 구현** — 보장된 메모리 안전성 및 제로 코스트 추상화

📊 **[전체 벤치마크 보고서](https://bytelogiccore-spec.github.io/DBX/korean/benchmarks)** — SQLite, Sled, Redb 상세 비교

---

## 📦 5계층 하이브리드 스토리지 아키텍처

```
┌─────────────────────────────────────────┐
│  Tier 1: Delta Store (BTreeMap)         │  ← 인메모리 쓰기 버퍼 (52.8만 건/초)
└─────────────────┬───────────────────────┘
                  │ Flush
┌─────────────────▼───────────────────────┐
│  Tier 2: Columnar Cache (Arrow)         │  ← OLAP 최적화 (Projection Pushdown)
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 3: WOS (sled)                     │  ← MVCC 스냅샷 격리
└─────────────────┬───────────────────────┘
                  │ Compaction
┌─────────────────▼───────────────────────┐
│  Tier 4: Index (Bloom Filter)           │  ← 빠른 존재 여부 확인
└─────────────────┬───────────────────────┘
                  │
┌─────────────────▼───────────────────────┐
│  Tier 5: ROS (Parquet)                  │  ← 컬럼형 압축 저장
└─────────────────────────────────────────┘

                  선택 사항: GPU 가속 (CUDA)
```

🏗️ **[아키텍처 심층 분석](https://bytelogiccore-spec.github.io/DBX/korean/architecture)** — DBX가 6.7배의 성능을 달성하는 방법

---

## 🌐 언어 바인딩 (Language Bindings)

DBX는 다양한 언어를 위한 공식 바인딩을 제공합니다:

- **Python** - Context Manager를 지원하는 파이썬다운 API
- **C#/.NET** - 고성능 .NET 바인딩
- **C/C++** - 저수준 C API 및 현대적인 C++17 래퍼
- **Node.js** - 네이티브 N-API 바인딩

**[언어 바인딩 가이드 보기 →](https://bytelogiccore-spec.github.io/DBX/korean/guides/language-bindings)**

---

## 📚 문서 (Documentation)

### 🎓 시작하기
- **[빠른 시작 가이드](https://bytelogiccore-spec.github.io/DBX/korean/getting-started)** — 설치 및 첫 쿼리 실행
- **[초보자 튜토리얼](https://bytelogiccore-spec.github.io/DBX/korean/tutorials/beginner)** — 단계별 학습 경로

### 📖 기능 가이드
- **[CRUD 작업](https://bytelogiccore-spec.github.io/DBX/korean/guides/crud-operations)** — 삽입, 조회, 삭제, 배치 작업
- **[트랜잭션](https://bytelogiccore-spec.github.io/DBX/korean/guides/transactions)** — MVCC, 스냅샷 격리, 동시성 제어
- **[SQL 레퍼런스](https://bytelogiccore-spec.github.io/DBX/korean/guides/sql-reference)** — 지원 구문 및 쿼리 최적화
- **[저장소 계층](https://bytelogiccore-spec.github.io/DBX/korean/guides/storage-layers)** — 5계층 아키텍처 상세 설명
- **[GPU 가속](https://bytelogiccore-spec.github.io/DBX/korean/guides/gpu-acceleration)** — CUDA 설정 및 성능 튜닝

---

## ✨ 주요 기능 및 로드맵

### 핵심 기능 ✅
- ✅ **5계층 하이브리드 스토리지** — Delta → Cache → WOS → Index → ROS
- ✅ **MVCC 트랜잭션** — 스냅샷 격리, 가비지 컬렉션
- ✅ **SQL 지원** — SELECT, WHERE, JOIN, GROUP BY, ORDER BY
- ✅ **GPU 가속** — CUDA 기반 집계 및 필터링
- ✅ **암호화** — AES-256-GCM-SIV, ChaCha20-Poly1305
- ✅ **압축** — ZSTD, Brotli
- ✅ **WAL 2.0** — 비동기 fsync를 지원하는 이진 로깅

### 로드맵 🚧
- **Phase 1: 트리거 시스템** — BEFORE/AFTER 트리거, 조건부 로직
- **Phase 2: 사용자 정의 함수 (UDF)** — 스칼라, 집계, 테이블 UDF
- **Phase 3: 파티셔닝** — 범위, 해시, 리스트 파티셔닝 및 프루닝
- **Phase 4: 작업 스케줄러** — 자동 유지보수 및 주기적 작업 실행
- **Phase 5: 고급 기능** — 구체화된 뷰(Materialized Views), 복제, 샤딩

---

## 📄 라이선스

DBX는 **이중 라이선스 모델**로 제공됩니다:

- **🆓 MIT 라이선스** — 개인, 스타트업, 소규모 조직에 무료
- **💼 상업용 라이선스** — 대규모 조직(구성원 100명 이상 또는 매출 500만 달러 이상) 필수

📚 **[라이선스 정책 가이드](https://github.com/bytelogiccore-spec/DBX/blob/main/legal/korean/LICENSE-POLICY.md)** — 나에게 맞는 라이선스는?

📧 **상업용 라이선스 문의:** license@bytelogic.studio

---

## 🤝 기여하기

이슈 제보와 풀 리퀘스트는 언제나 환영합니다!

코드 규약 및 PR 제출 프로세스에 대한 자세한 내용은 [기여 가이드](https://github.com/bytelogiccore-spec/DBX/blob/main/legal/korean/CONTRIBUTING.md)를 확인해 주세요.

---

**Made with ❤️ in Rust**
