---
layout: default
title: 데이터베이스 API
nav_order: 1
parent: API Reference
---

# 데이터베이스 API
{: .no_toc }

DBX의 핵심 데이터베이스 작업을 위한 API 레퍼런스입니다.
{: .fs-6 .fw-300 }

---

## 생성자 (Constructors)

### `Database::open(path: &Path) -> DbxResult<Database>`
지정된 경로에 데이터베이스를 열거나 새로 생성합니다.

### `Database::open_in_memory() -> DbxResult<Database>`
테스트 또는 임시 저장용 인메모리 데이터베이스를 생성합니다.

### `Database::open_encrypted(path: &Path, encryption: EncryptionConfig) -> DbxResult<Database>`
암호화 설정이 적용된 데이터베이스를 엽니다.

---

## CRUD 작업

### `insert(table: &str, key: &[u8], value: &[u8]) -> DbxResult<()>`
테이블에 키-값 쌍을 삽입합니다.

### `insert_batch(table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()>`
여러 레코드를 한 번에 삽입합니다 (고성능).

### `get(table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>>`
키를 사용하여 데이터를 조회합니다.

### `delete(table: &str, key: &[u8]) -> DbxResult<bool>`
데이터를 삭제합니다.

---

## SQL 작업

### `execute_sql(sql: &str) -> DbxResult<RecordBatch>`
SQL 쿼리를 실행하고 결과를 Arrow `RecordBatch` 형식으로 반환합니다.
- 지원 구문: `SELECT`, `WHERE`, `JOIN`, `GROUP BY`, `ORDER BY` 등

---

## 트랜잭션 작업

### `begin() -> DbxResult<Transaction>`
스냅샷 격리(Snapshot Isolation)가 적용된 새 MVCC 트랜잭션을 시작합니다.

---

## 관리 및 유지보수

### `flush() -> DbxResult<()>`
메모리(Delta Store)에 있는 모든 데이터를 영구 저장소(WOS)로 플러시합니다.

### `set_durability(level: DurabilityLevel)`
쓰기 작업의 내구성 수준을 설정합니다.
- `Full`: 매 쓰기마다 디스크 동기화 (최고 안전)
- `Lazy`: 백그라운드 동기화 (기본값, 성능/안전 균형)
- `None`: WAL 비활성화 (최고 성능)

---

## 성능 팁

1. **대량 삽입 시 `insert_batch` 사용**: 단일 삽입보다 10~100배 빠릅니다.
2. **내구성 수준 조정**: 쓰기 부하가 큰 경우 `Lazy` 설정을 권장합니다.
3. **주기적 `flush()` 호출**: 메모리 사용량이 너무 커지지 않도록 관리하세요.

---

## 참고 항목

- [트랜잭션 가이드](../guides/transactions) — 비즈니스 로직에서의 트랜잭션 활용
- [CRUD 작업 가이드](../guides/crud-operations) — 상세한 사용 예시
- [SQL 레퍼런스](../guides/sql-reference) — 복잡한 쿼리 처리 방법
