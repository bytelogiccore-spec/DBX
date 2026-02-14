---
layout: default
title: 트랜잭션 API
nav_order: 42
parent: 한국어
---

# 트랜잭션 API
{: .no_toc }

DBX의 MVCC 트랜잭션 관리를 위한 API 레퍼런스입니다.
{: .fs-6 .fw-300 }

---

## 개요

DBX는 **MVCC (Multi-Version Concurrency Control)**를 기반으로 한 **스냅샷 격리(Snapshot Isolation)** 트랜잭션을 제공합니다. 컴파일 타임 안정성을 위해 **Typestate 패턴**을 사용합니다.

### 트랜잭션 상태
- **Active**: 작업 수행 가능 상태
- **Committed**: 변경 사항이 영구 반영된 상태 (최종)
- **Aborted**: 변경 사항이 폐기된 상태 (최종)

---

## 트랜잭션 생성

### `Database::begin() -> DbxResult<Transaction<'_, Active>>`
새로운 MVCC 트랜잭션을 시작합니다.

```rust
let tx = db.begin()?;
```

---

## 트랜잭션 작업

### `insert(table: &str, key: &[u8], value: &[u8]) -> DbxResult<()>`
트랜잭션 내부에서 키-값 쌍을 삽입합니다.

### `get(table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>>`
트랜잭션 시작 시점의 스냅샷을 기준으로 데이터를 조회합니다.

### `delete(table: &str, key: &[u8]) -> DbxResult<bool>`
트랜잭션 내부에서 데이터를 삭제합니다.

---

## 트랜잭션 종료

### `commit(self) -> DbxResult<Transaction<'_, Committed>>`
트랜잭션을 커밋하여 변경 사항을 영구적으로 저장합니다.

### `abort(self) -> DbxResult<Transaction<'_, Aborted>>`
트랜잭션을 중단하고 모든 변경 사항을 폐기합니다.

---

## 스냅샷 격리 (Snapshot Isolation)의 작동 원리

1. **스냅샷 생성**: `begin()` 호출 시점의 데이터베이스 상태를 고정합니다.
2. **일관된 읽기**: 트랜잭션 내부의 모든 읽기 작업은 다른 트랜잭션의 방해 없이 동일한 스냅샷을 봅니다.
3. **충돌 감지**: 커밋 시점에 다른 트랜잭션과 동일한 키에 대해 쓰기 충돌이 발생했는지 확인합니다.

---

## 성능 및 스레드 안정성

- **Database**: 스레드 세이프하며 여러 스레드에서 공유 가능합니다.
- **Transaction**: 스레드 세이프하지 않으므로 한 스레드당 하나의 트랜잭션을 사용해야 합니다.
- **가비지 컬렉션**: 오래된 트랜잭션 버전은 시스템에서 자동으로 정리(GC)됩니다.

---

## 주요 에러 처리

- `DbxError::TransactionConflict`: 같은 데이터에 대해 동시에 쓰기가 발생하여 충돌이 일어난 경우 (재시도 필요)
- `DbxError::TransactionAborted`: 트랜잭션이 중단된 상태에서 작업을 시도한 경우

---

## 다음 단계
- [트랜잭션 가이드](../guides/transactions) — 상세 트랜잭션 패턴 및 예제
- [데이터베이스 API](database) — 기본 CRUD 작업 API
- [SQL API](sql) — 트랜잭션 내 SQL 실행 (향후 지원 예정)
