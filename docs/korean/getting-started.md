---
layout: default
title: 시작하기
nav_order: 1
parent: 한국어
description: "DBX 데이터베이스 빠른 시작 가이드"
---

# 시작하기
{: .no_toc }

이 가이드는 DBX를 설치하고 첫 번째 쿼리를 실행하는 데 도움을 줍니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 설치

### Rust

`Cargo.toml`에 DBX를 추가하세요:

```toml
[dependencies]
dbx-core = "0.0.1-beta"
```

### .NET (C#, VB.NET, F#)

NuGet을 통해 설치하세요:

```bash
dotnet add package DBX.Client
```

---

## 기본 사용법

### 데이터베이스 열기

```rust
use dbx_core::Database;

// 인메모리 데이터베이스
let db = Database::open_in_memory()?;

// 영구 저장 데이터베이스
let db = Database::open("./mydata")?;
```

### CRUD 작업

#### 삽입 (Insert)

```rust
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;
```

#### 조회 (Get)

```rust
let value = db.get("users", b"user:1")?;
assert_eq!(value, Some(b"Alice".to_vec()));
```

#### 삭제 (Delete)

```rust
db.delete("users", b"user:1")?;
```

#### 개수 확인 (Count)

```rust
let count = db.count("users")?;
println!("Total users: {}", count);
```

---

## MVCC 트랜잭션

DBX는 스냅샷 격리(Snapshot Isolation) 기능이 포함된 ACID 트랜잭션을 지원합니다:

```rust
use dbx_core::Database;

let db = Database::open("./data")?;

// 트랜잭션 시작
let tx = db.begin_transaction()?;

// 스냅샷 격리를 통한 일관된 읽기
tx.insert("users", b"user:3", b"Charlie")?;
tx.insert("users", b"user:4", b"David")?;

// 커밋 (또는 롤백)
tx.commit()?;
```

---

## SQL 쿼리

DBX는 표준 SQL 쿼리를 지원합니다:

```rust
use dbx_core::Database;
use arrow::array::{Int32Array, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

let db = Database::open_in_memory()?;

// 테이블 스키마 생성
let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int32, false),
    Field::new("age", DataType::Int32, false),
]));

// 데이터 생성
let batch = RecordBatch::try_new(
    schema.clone(),
    vec![
        Arc::new(Int32Array::from(vec![1, 2, 3])),
        Arc::new(Int32Array::from(vec![25, 30, 35])),
    ],
).unwrap();

// 테이블 등록
db.register_table("users", vec![batch]);

// SQL 실행
let results = db.execute_sql("SELECT id, age FROM users WHERE age > 28")?;
```

---

## 암호화

DBX는 AES-256-GCM-SIV 및 ChaCha20-Poly1305 암호화를 지원합니다:

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

// 암호화된 데이터베이스 생성
let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted("./secure-data", enc)?;

// 일반적인 사용
db.insert("secrets", b"key1", b"sensitive-data")?;
```

### 키 교체 (Key Rotation)

```rust
// 암호화 키 교체
let new_enc = EncryptionConfig::from_password("new-password");
let count = db.rotate_key(new_enc)?;
println!("교체된 레코드 수: {}", count);
```

---

## GPU 가속 (선택 사항)

`Cargo.toml`에서 GPU 기능을 활성화하세요:

```toml
[dependencies]
dbx-core = { version = "0.0.1-beta", features = ["gpu"] }
```

GPU 가속 사용:

```rust
let db = Database::open_in_memory()?;

// ... 데이터 등록 ...

// GPU 캐시 동기화
db.sync_gpu_cache("users")?;

// GPU 가속 작업
if let Some(gpu) = db.gpu_manager() {
    let sum = gpu.sum("users", "age")?;
    let filtered = gpu.filter_gt("users", "age", 30)?;
}
```

---

## 다음 단계

- [아키텍처 가이드](architecture) — 5-Tier 하이브리드 스토리지에 대해 알아보기
- [벤치마크](benchmarks) — 성능 비교 확인
- [예제](examples/quick-start) — 더 많은 코드 예제 살펴보기
- [API 문서](https://docs.rs/dbx-core) — 전체 Rust API 레퍼런스
