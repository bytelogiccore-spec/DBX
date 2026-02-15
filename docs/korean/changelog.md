---
layout: default
title: 변경 이력
nav_order: 8
parent: 한국어
---

# 변경 이력 (Changelog)

DBX의 주요 변경사항을 기록합니다.

[Keep a Changelog](https://keepachangelog.com/ko/1.1.0/) 형식을 따르며,
[Semantic Versioning](https://semver.org/lang/ko/) 규칙을 준수합니다.

---

## [0.0.4-beta] - 2026-02-15

첫 번째 기능 릴리스. 쿼리 실행 파이프라인 전면 최적화.

### 새로운 기능

- **쿼리 플랜 캐시** — 동일 SQL을 반복 실행할 때 파싱/최적화를 건너뛰는 2계층(메모리 + 디스크) 캐시 도입
- **병렬 쿼리 실행** — 대량 데이터 필터링, 집계, 프로젝션을 Rayon 스레드 풀로 병렬 처리
- **WAL 파티셔닝** — 테이블별 독립 WAL 파티션으로 쓰기 병목 해소
- **스키마 버저닝** — 무중단 DDL 지원. 스키마 변경 이력 관리 및 버전별 롤백
- **인덱스 버저닝** — 인덱스 재구축 이력 관리 및 성능 추적
- **기능 플래그** — 런타임에 개별 기능을 켜고 끌 수 있는 토글 시스템 (환경변수 / 파일 저장 지원)
- **UDF 프레임워크** — 사용자 정의 함수 (스칼라, 집계, 테이블), 트리거, 스케줄러
- **벤치마크 프레임워크** — Criterion 기반 성능 측정 및 Before/After 비교 도구
- **PTX Persistent Kernel** — NVRTC 기반 런타임 CUDA 커널 컴파일. GPU에서 상주하며 work queue 처리 (`gpu` feature, 옵셔널)
- **Hash/Range 샤딩** — GPU 샤드 전략: 해시 기반(ahash) 및 범위 기반 행 분배
- **CUDA 스트림 관리** — `fork_default_stream()`을 통한 별도 스트림 생성
- **스키마 기반 INSERT 직렬화** — 테이블 스키마 존재 시 컬럼명 키를 가진 JSON 객체 직렬화
- **JOIN 최적화** — INNER JOIN에서 크기 기반 build/probe 테이블 스왑 (작은 테이블을 build로 사용)
- **Tombstone 삭제** — 컬럼나 델타 스토리지에 버저닝 tombstone 지원
- **테이블별 캐시 무효화** — 전체 캐시 초기화 대신 테이블 단위 선택적 제거

### 성능 개선

| 항목 | 이전 | 이후 | 개선폭 |
|------|:----:|:----:|:------:|
| SQL 반복 파싱 (10회) | 146 µs | 20 µs | 7.3x |
| WAL 100건 쓰기 | 1,016 µs | 71 µs | 14.2x |
| 스키마 조회 (싱글스레드) | 86 ns | 46 ns | 47% |
| 스키마 조회 (8스레드 동시) | 7.4M ops/s | 18.1M ops/s | 2.44x |
| 소규모 집계 (150행) | 32.5 µs | 991 ns | 33x |

### 리팩토링

- **SQL 옵티마이저** — 874줄 단일 파일 `optimizer.rs`를 모듈 디렉토리 구조로 분리 (6파일: trait, 규칙 4개, 테스트)
- **CREATE FUNCTION** — 괄호 파라미터 실제 파싱 구현
- **ORDER BY** — `sqlparser` 0.52 `OrderBy.exprs` API 테스트 활성화

### 내부 변경

- `SchemaVersionManager` 내부 저장소를 `RwLock<HashMap>` → `DashMap`으로 전환하여 동시 읽기 성능 향상
- `ParallelQueryExecutor`의 병렬화 판단 기준을 batch 수에서 **총 행 수 기반**으로 변경 (기본 1,000행 미만은 순차 실행)
- SQL 파서에 동적 스레딩 및 배치 크기 자동 조절 적용
- `cudarc` 0.19.2의 Unified Memory, P2P 감지, Persistent Kernel 제한사항 문서화

### 의존성

- `dashmap` 6.x 추가 (락프리 동시 해시맵)
- `rayon` 1.x 추가 (병렬 처리)
- `criterion` 0.5 추가 (벤치마크)

---

## [0.0.3-beta] - 2026-02-15

### 추가
- Python, Node.js, .NET 패키지 상세 사용법 가이드
  - JSON 데이터 처리 예제
  - 배치 작업 및 에러 처리
  - 실전 예제 (KV Store, Session Manager, Cache Wrapper)
  - Node.js TypeScript 지원
  - ASP.NET Core 통합 예제
- 모든 언어 바인딩에 대한 이중 언어 문서 (영문 + 한글)

### 변경
- **플랫폼 지원**: **Windows x64 전용**으로 수정 (Linux/macOS 계획됨)
- **Cargo.toml**: `homepage`를 GitHub Pages로 변경
- **crates.io**: `dbx-core`만 배포 (`dbx-derive`, `dbx-ffi` 제거)
- **문서**: Derive Macro 섹션 제거 (프로덕션에서 미사용)
- **Doc Comment**: Rust doc comment를 영문으로 변환 (docs.rs 일관성)

### 수정
- 과대 광고된 플랫폼 지원 (이전: 모든 플랫폼, 현재: Windows x64 전용)
- 패키지 간 버전 불일치

---

## [0.0.2-beta] - 2026-02-15

### 추가
- 모든 언어 바인딩 패키지 문서화 (Rust, .NET, Python, Node.js, C/C++)
- GitHub Pages 이중 언어 문서 (영어 + 한국어) 패키지별 제공
- CHANGELOG.md 생성
- NuGet 패키지 메타데이터 (버전, 라이선스, README)
- 모든 Rust 크레이트 Cargo.toml에 `readme` 필드 추가
- GitHub Release 워크플로우에 `permissions: contents: write` 추가

### 변경
- **CI/CD**: 단일 릴리스 워크플로우를 레지스트리별 독립 워크플로우로 분리
  - `publish-crates.yml` — crates.io (dbx-derive → dbx-core → dbx-ffi 순서)
  - `publish-nuget.yml` — NuGet
  - `publish-pypi.yml` — PyPI
  - `publish-npm.yml` — npm
  - `release.yml` — 빌드 + 테스트 + GitHub Release 생성만 담당
- **버전**: 모든 패키지를 `0.0.2-beta`로 통일
- **라이선스**: crates.io 호환을 위해 `MIT`로 단순화
- **워크스페이스 메타데이터**: `repository`, `homepage`, `documentation` 상속 추가
- **crates.io**: publish 명령에서 `|| true` 제거, `--no-verify` 추가, 인덱스 대기 60초로 증가

### 수정
- NuGet 403 오류: API 키 권한 설정 가이드
- PyPI 400 오류: PEP 440 형식으로 버전 수정 (`0.0.2b0`)
- npm EOTP 오류: 2FA 우회를 위한 Granular Access Token 가이드
- crates.io 순환 의존성: `dbx-derive` dev-dependency에서 `version` 제거
- GitHub Release 403: `contents: write` 권한 추가
- `edition = "2024"` 유지하여 `let chains` 문법 지원

---

## [0.0.1-beta] - 2026-02-12

### 추가
- 최초 릴리스
- 5-Tier 하이브리드 스토리지 엔진 (WOS → L0 → L1 → L2 → Cold)
- MVCC 트랜잭션 지원 (스냅샷 격리)
- SQL 엔진 (CREATE TABLE, INSERT, SELECT, UPDATE, DELETE)
- Write-Ahead Logging (WAL) 장애 복구
- 언어 바인딩: Rust, C#/.NET, Python, Node.js, C/C++
- 암호화 지원 (AES-GCM-SIV, ChaCha20-Poly1305)
- Arrow/Parquet 네이티브 컬럼나 포맷
- GitHub Pages 문서 사이트
- GitHub Actions CI/CD 파이프라인
- SQLite, Sled, Redb 비교 벤치마크
