---
layout: default
title: Rust (dbx-core)
parent: 패키지
grand_parent: 한국어
nav_order: 1
---

# Rust — dbx-core

[![Crates.io](https://img.shields.io/crates/v/dbx-core.svg)](https://crates.io/crates/dbx-core)
[![docs.rs](https://docs.rs/dbx-core/badge.svg)](https://docs.rs/dbx-core)

DBX의 핵심 Rust 크레이트 — 5-Tier 하이브리드 스토리지 기반 고성능 임베디드 데이터베이스.

## 설치

```toml
[dependencies]
dbx-core = "0.0.3-beta"
```

## 빠른 시작

```rust
use dbx_core::Database;

fn main() -> dbx_core::error::DbxResult<()> {
    let db = Database::open_in_memory()?;

    // 삽입
    db.insert("users", b"user:1", b"Alice")?;

    // 조회
    if let Some(value) = db.get("users", b"user:1")? {
        println!("{}", String::from_utf8_lossy(&value));
    }

    // 삭제
    db.delete("users", b"user:1")?;

    Ok(())
}
```

## SQL 인터페이스

```rust
let db = Database::open_in_memory()?;

db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT, email TEXT)")?;
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com')")?;

let result = db.execute_sql("SELECT * FROM users WHERE id = 1")?;
println!("{:?}", result);
```

## 기능

| 기능 | 설명 |
|------|------|
| 5-Tier 스토리지 | WOS → L0 → L1 → L2 → Cold Storage |
| MVCC | 스냅샷 격리 트랜잭션 |
| SQL 엔진 | DDL + DML 지원 |
| WAL | 장애 복구 |
| 암호화 | AES-GCM-SIV, ChaCha20-Poly1305 |
| Arrow/Parquet | 네이티브 컬럼나 포맷 |

## 피처 플래그

```toml
dbx-core = { version = "0.0.3-beta", features = ["simd", "logging"] }
```

| 플래그 | 설명 |
|--------|------|
| `simd` | SIMD 가속 연산 |
| `gpu` | CUDA를 통한 GPU 가속 |
| `logging` | 트레이싱 출력 활성화 |

## API 문서

전체 API 문서: [docs.rs/dbx-core](https://docs.rs/dbx-core)
