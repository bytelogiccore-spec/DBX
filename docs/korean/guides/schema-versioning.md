---
layout: default
title: 스키마 버저닝
parent: 한국어
nav_order: 35
---

# 스키마 버저닝 (Schema Versioning)
{: .no_toc }

무중단으로 테이블 스키마를 변경하고, 이전 버전으로 롤백할 수 있습니다.
{: .fs-6 .fw-300 }

---

## 개요

운영 중인 데이터베이스의 스키마를 변경하는 것은 위험한 작업입니다. DBX의 스키마 버저닝은 모든 변경 이력을 보관하여 안전한 DDL 작업을 보장합니다.

---

## 기본 사용법

```rust
use dbx_core::engine::schema_versioning::SchemaVersionManager;

let manager = SchemaVersionManager::new();

// 테이블 등록 (v1)
let v = manager.register_table("users", schema_v1)?;
assert_eq!(v, 1);

// 컬럼 추가 (v2)
let v = manager.alter_table("users", schema_v2, "email 컬럼 추가")?;
assert_eq!(v, 2);

// 현재 스키마 조회 — O(1) 성능 (DashMap 캐시)
let schema = manager.get_current("users")?;
```

---

## 버전 관리

```rust
// 현재 버전 확인
let version = manager.current_version("users")?;  // → 2

// 특정 버전의 스키마 조회
let old_schema = manager.get_at_version("users", 1)?;

// 이전 버전으로 롤백
manager.rollback("users", 1)?;
let version = manager.current_version("users")?;  // → 1
```

---

## 버전 이력 조회

```rust
let history = manager.history("users")?;
for entry in &history {
    println!("v{}: {} ({})", entry.version, entry.description, entry.created_at);
}
// 출력:
// v1: Initial schema (2026-02-15 11:00)
// v2: email 컬럼 추가 (2026-02-15 14:30)
```

---

## 성능

| 연산 | 시간 | 비고 |
|------|------|------|
| `get_current` | **46 ns** | DashMap 캐시 직접 조회 |
| `alter_table` | **746 ns** | 새 버전 추가 |
| 8스레드 동시 조회 | **18.1M ops/s** | RwLock 대비 2.44x 빠름 |

---

## 다음 단계

- [인덱싱 가이드](indexing) — 인덱스 버저닝과의 연계
- [트랜잭션 가이드](transactions) — DDL과 트랜잭션의 관계
