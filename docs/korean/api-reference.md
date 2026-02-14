---
layout: default
title: API 레퍼런스
nav_order: 7
parent: 한국어
---

# API 레퍼런스

## Rust API 문서

완전한 Rust API 문서는 **docs.rs**에서 확인할 수 있습니다 (영문):

[![docs.rs](https://docs.rs/dbx-core/badge.svg)](https://docs.rs/dbx-core)

**[→ docs.rs에서 전체 API 문서 보기](https://docs.rs/dbx-core)**

---

## 빠른 참조

### 핵심 타입

| 타입 | 설명 |
|------|------|
| [`Database`](https://docs.rs/dbx-core/latest/dbx_core/struct.Database.html) | 메인 데이터베이스 핸들 |
| [`Transaction`](https://docs.rs/dbx-core/latest/dbx_core/transaction/struct.Transaction.html) | MVCC 트랜잭션 |
| [`Table`](https://docs.rs/dbx-core/latest/dbx_core/derive.Table.html) | 스키마 derive 매크로 |

### 주요 메서드

```rust
// 데이터베이스 작업
Database::open(path) -> Result<Database>
Database::open_in_memory() -> Result<Database>
db.insert(table, key, value) -> Result<()>
db.get(table, key) -> Result<Option<Vec<u8>>>
db.delete(table, key) -> Result<()>

// SQL 인터페이스
db.execute_sql(sql) -> Result<SqlResult>

// 트랜잭션
db.begin_transaction() -> Result<Transaction>
tx.commit() -> Result<()>
tx.rollback() -> Result<()>
```

### 기능 플래그

| 플래그 | 설명 |
|--------|------|
| `simd` | SIMD 가속 연산 |
| `gpu` | CUDA를 통한 GPU 가속 |
| `logging` | 추적 로그 활성화 |

---

## 다른 언어 바인딩

- [.NET API 레퍼런스](../packages/dotnet#api-레퍼런스)
- [Python API 레퍼런스](../packages/python#api-레퍼런스)
- [Node.js API 레퍼런스](../packages/nodejs#api-레퍼런스)
- [C/C++ API 레퍼런스](../packages/cpp#c-api-reference)
