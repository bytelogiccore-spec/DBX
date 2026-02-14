---
layout: default
title: 초보자 튜토리얼
parent: 한국어
nav_order: 30
description: "DBX 초보자를 위한 단계별 가이드"
---

# 초보자 튜토리얼 (Beginner Tutorial)
{: .no_toc }

DBX를 처음 시작하는 분들을 위한 단계별 가이드입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 소개

이 튜토리얼에서는 첫 번째 DBX 데이터베이스를 생성하고, 기본적인 작업을 수행하며, 간단한 SQL 쿼리를 실행하는 방법을 배웁니다.

**학습 내용:**
- DBX 설치 및 프로젝트 설정
- 데이터베이스 생성
- 데이터 삽입 및 조회
- 트랜잭션 사용법
- SQL 쿼리 실행

---

## 1단계: 프로젝트 생성 및 설정

새 Rust 프로젝트를 생성합니다:

```bash
cargo new my_dbx_app
cd my_dbx_app
```

`Cargo.toml`에 의존성을 추가합니다:

```toml
[dependencies]
dbx-core = "0.0.1-beta"
arrow = "50.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## 2단계: 데이터베이스 열기

인메모리 또는 파일 기반 데이터베이스를 열 수 있습니다.

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // 인메모리 데이터베이스 생성 (테스트용)
    let db = Database::open_in_memory()?;
    
    // 또는 파일 기반 영구 저장소
    // let db = Database::open("./my_database")?;
    
    println!("데이터베이스가 성공적으로 생성되었습니다.");
    Ok(())
}
```

---

## 3단계: 기본 CRUD 작업

데이터를 삽입하고 조회하는 가장 기본적인 방법입니다.

```rust
// 데이터 삽입
db.insert("users", b"user:1", b"Alice")?;

// 데이터 조회
if let Some(data) = db.get("users", b"user:1")? {
    let name = String::from_utf8(data).unwrap();
    println!("찾은 사용자: {}", name);
}
```

---

## 4단계: 트랜잭션 활용

여러 작업을 원자적으로(전부 성공하거나 전부 실패하게) 처리합니다.

```rust
let tx = db.begin_transaction()?;

tx.insert("users", b"user:2", b"Bob")?;
tx.insert("users", b"user:3", b"Charlie")?;

// 중요: 반드시 commit()을 호출해야 변경사항이 저장됩니다.
tx.commit()?;
```

---

## 5단계: SQL 쿼리 실행

Apache Arrow 기반의 강력한 SQL 엔진을 사용하여 데이터를 조회합니다.

```rust
let results = db.execute_sql(
    "SELECT name, age FROM users WHERE age > 20"
)?;

for batch in results {
    println!("{:?}", batch);
}
```

---

## 다음 단계

축하합니다! DBX의 기본을 배우셨습니다. 다음 가이드를 통해 더 깊이 있게 학습해 보세요.

- [CRUD 작업 가이드](../guides/crud-operations) — 상세 CRUD 활용법
- [트랜잭션 가이드](../guides/transactions) — MVCC 트랜잭션 마스터하기
- [SQL 레퍼런스](../guides/sql-reference) — 다양한 SQL 구문 학습
- [저장소 계층](../guides/storage-layers) — 내부 아키텍처 이해하기
