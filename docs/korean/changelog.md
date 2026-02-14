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
