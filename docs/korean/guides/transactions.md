---
layout: default
title: 트랜잭션
parent: Guides
nav_order: 2
description: "DBX의 MVCC 트랜잭션 및 동시성 제어"
---

# 트랜잭션
{: .no_toc }

DBX의 MVCC 트랜잭션 및 동시성 제어에 대한 종합 가이드입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX는 높은 동시성을 허용하면서 ACID 보장을 제공하기 위해 **스냅샷 격리(Snapshot Isolation)** 기능이 포함된 **다중 버전 동시성 제어(MVCC)**를 구현합니다.

### 주요 특징

- **스냅샷 격리**: 각 트랜잭션은 데이터베이스의 일관된 스냅샷을 봅니다.
- **ACID 보장**: 원자성, 일관성, 격리성, 내구성 보장
- **읽기 잠금 없음**: 읽기 작업이 쓰기 작업을 차단하지 않으며, 그 반대도 마찬가지입니다.
- **쓰기 충돌 감지**: 쓰기 충돌을 자동으로 감지하고 처리합니다.
- **가비지 컬렉션**: 오래된 버전을 자동으로 정리합니다.

---

## 트랜잭션 기본

### 트랜잭션 시작

새 트랜잭션을 시작합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 새 트랜잭션 시작
    let tx = db.begin_transaction()?;
    
    // 트랜잭션 작업...
    
    Ok(())
}
```

### 트랜잭션 커밋

모든 변경 사항을 확정합니다:

```rust
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.commit()?;
```

### 트랜잭션 롤백

모든 변경 사항을 취소하고 폐기합니다:

```rust
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.rollback()?; // 변경 사항 폐기
```

---

## MVCC와 스냅샷 격리

### 작동 방식

트랜잭션이 시작되면 **읽기 타임스탬프(`read_ts`)**를 할당받습니다. 모든 읽기 작업은 해당 타임스탬프 시점의 데이터를 조회합니다.

**읽기 규칙**: 타임스탬프 `T`를 가진 트랜잭션은 `version <= T` 인 최신 버전을 봅니다.

---

## 동시성 패턴

### 읽기-쓰기 동시성

읽기 작업과 쓰기 작업은 서로를 차단하지 않습니다. 이는 분석(OLAP)과 트랜잭션(OLTP) 워크로드를 동시에 처리하는 데 필수적입니다.

### 쓰기-쓰기 충돌

DBX는 동일한 레코드에 대한 동시 쓰기를 감지합니다:

```rust
match tx2.commit() {
    Err(DbxError::WriteConflict) => {
        println!("쓰기 충돌 감지됨!");
    }
    _ => {}
}
```

---

## 권장 사항 (Best Practices)

1. **트랜잭션을 짧게 유지하세요**: 긴 트랜잭션은 가비지 컬렉션을 방해하고 리소스를 점유합니다.
2. **관련 작업을 배치로 처리하세요**: 여러 작업을 하나의 트랜잭션으로 묶어 효율성을 높이세요.
3. **충돌 발생 시 재시도 로직을 구현하세요**: 쓰기 충돌 발생 시 자동으로 재시도하는 패턴을 권장합니다.

---

## 다음 단계

- [CRUD 작업](crud-operations) — 기본 데이터베이스 작업
- [SQL 레퍼런스](sql-reference) — 트랜잭션 내에서 SQL 사용하기
- [API 레퍼런스](../api/transaction) — 전체 트랜잭션 API
