---
layout: default
title: CRUD 작업
parent: 한국어
nav_order: 20
description: "DBX의 생성, 조회, 수정, 삭제 작업에 대한 전체 가이드"
---

# CRUD 작업
{: .no_toc }

DBX에서 생성(Create), 조회(Read), 수정(Update), 삭제(Delete) 작업을 수행하는 방법에 대한 종합 가이드입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX는 기본적인 데이터베이스 작업을 위해 단순하고 효율적인 API를 제공합니다. 모든 CRUD 작업은 `Database` 인스턴스를 통해 수행되며, 단일 레코드 작업과 배치(batch) 작업을 모두 지원합니다.

### 주요 특징

- **고성능**: 핫 데이터를 위한 인메모리 Delta Store 활용
- **ACID 보장**: 모든 작업은 원자성(Atomic)과 내구성(Durable)을 가짐
- **동시 액세스**: Lock-free 읽기 및 안전한 동시 쓰기 지원
- **자동 플러시**: Delta Store가 임계값 도달 시 영구 저장소로 자동 이동
---

## 삽입 작업 (Insert)

### 단일 삽입

테이블에 단일 키-값 쌍을 삽입합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 레코드 삽입
    db.insert("users", b"user:1", b"Alice")?;
    
    Ok(())
}
```

**매개변수:**
- `table`: 테이블 이름 (문자열 슬라이스)
- `key`: 고유 키 (바이트 슬라이스)
- `value`: 저장할 값 (바이트 슬라이스)

**반환값:**
- `DbxResult<()>`: 성공 또는 에러

### 배치 삽입

여러 레코드를 효율적으로 삽입합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 배치 데이터 준비
    let records = vec![
        (b"user:1".to_vec(), b"Alice".to_vec()),
        (b"user:2".to_vec(), b"Bob".to_vec()),
        (b"user:3".to_vec(), b"Charlie".to_vec()),
    ];
    
    // 배치 삽입 (반복문 사용)
    for (key, value) in records {
        db.insert("users", &key, &value)?;
    }
    
    Ok(())
}
```

### 직렬화를 통한 삽입

직렬화를 사용하여 구조화된 데이터를 저장합니다:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    // JSON으로 직렬화
    let value = serde_json::to_vec(&user).unwrap();
    let key = format!("user:{}", user.id);
    
    db.insert("users", key.as_bytes(), &value)?;
    
    Ok(())
}
```

---

## 조회 작업 (Read)

### 단일 레코드 조회

키를 사용하여 값을 가져옵니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 레코드 조회
    let value = db.get("users", b"user:1")?;
    
    match value {
        Some(data) => {
            let name = String::from_utf8(data).unwrap();
            println!("조회 결과: {}", name);
        }
        None => println!("데이터를 찾을 수 없습니다."),
    }
    
    Ok(())
}
```

**반환값:**
- `DbxResult<Option<Vec<u8>>>`: 찾은 경우 데이터 반환, 없으면 None

### 역직렬화를 통한 조회

구조화된 데이터를 조회하고 역직렬화합니다:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u32,
    name: String,
    email: String,
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let key = b"user:1";
    if let Some(data) = db.get("users", key)? {
        let user: User = serde_json::from_slice(&data).unwrap();
        println!("사용자 정보: {:?}", user);
    }
    
    Ok(())
}
```

### 레코드 수 확인

테이블의 총 레코드 수를 가져옵니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let count = db.count("users")?;
    println!("총 사용자 수: {}", count);
    
    Ok(())
}
```

---

## 삭제 작업 (Delete)

### 단일 삭제

키를 사용하여 레코드를 삭제합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 레코드 삭제
    db.delete("users", b"user:1")?;
    
    // 삭제 확인
    let value = db.get("users", b"user:1")?;
    assert!(value.is_none());
    
    Ok(())
}
```

### 배치 삭제

여러 레코드를 삭제합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    let keys_to_delete = vec![
        b"user:1".to_vec(),
        b"user:2".to_vec(),
        b"user:3".to_vec(),
    ];
    
    for key in keys_to_delete {
        db.delete("users", &key)?;
    }
    
    Ok(())
}
```

---

## 성능 최적화

### Delta Store 캐싱

DBX는 핫 데이터를 위해 인메모리 Delta Store를 사용합니다:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 첫 쓰기는 Delta Store로 이동 (매우 빠름)
    db.insert("users", b"user:1", b"Alice")?;
    
    // Delta Store에서 즉시 읽기 (매우 빠름)
    let value = db.get("users", b"user:1")?;
    
    Ok(())
}
```

**성능 특성:**
- **삽입 (Insert)**: O(log n) - BTreeMap 삽입
- **조회 (Get)**: O(1) - Delta Store 적중 시, O(log n) - WOS/ROS 조회
- **삭제 (Delete)**: O(log n) - Tombstone(삭제 마커) 삽입

### 배치 작업 활용

더 나은 성능을 위해 작업을 배치로 처리하세요:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 모든 데이터를 먼저 준비
    let mut records = Vec::new();
    for i in 0..1000 {
        let key = format!("user:{}", i);
        let value = format!("User {}", i);
        records.push((key, value));
    }
    
    // 배치 삽입
    for (key, value) in records {
        db.insert("users", key.as_bytes(), value.as_bytes())?;
    }
    
    Ok(())
}
```

---

## 에러 처리

### 기본 에러 처리

```rust
use dbx_core::{Database, DbxError};

fn main() {
    let db = match Database::open("./data") {
        Ok(db) => db,
        Err(e) => {
            eprintln!("데이터베이스 열기 실패: {}", e);
            return;
        }
    };
    
    match db.insert("users", b"user:1", b"Alice") {
        Ok(_) => println!("삽입 성공"),
        Err(DbxError::DuplicateKey) => println!("키가 이미 존재합니다."),
        Err(e) => eprintln!("삽입 실패: {}", e),
    }
}
```

### ? 연산자 사용

```rust
use dbx_core::Database;

fn insert_user(db: &Database, id: u32, name: &str) -> dbx_core::DbxResult<()> {
    let key = format!("user:{}", id);
    db.insert("users", key.as_bytes(), name.as_bytes())?;
    Ok(())
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    insert_user(&db, 1, "Alice")?;
    Ok(())
}
```

---

## 권장 사항 (Best Practices)

### 1. 의미 있는 키 사용

```rust
// 좋음: 구조화된 키
db.insert("users", b"user:1", b"Alice")?;
db.insert("orders", b"order:2023-001", b"...")?;

// 피해야 함: 무작위이거나 불분명한 키
db.insert("data", b"abc123", b"...")?;
```

### 2. 복잡한 데이터 직렬화

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
    created_at: i64,
}

// JSON 또는 bincode로 저장
let user = User { /* ... */ };
let value = serde_json::to_vec(&user)?;
db.insert("users", b"user:1", &value)?;
```

### 3. 예외 상황 처리

```rust
match db.get("users", b"user:1")? {
    Some(data) => {
        // 데이터 처리
    }
    None => {
        // 데이터가 없는 경우 처리
        println!("사용자를 찾을 수 없습니다.");
    }
}
```

### 4. 관련 작업은 트랜잭션 사용

함께 성공하거나 실패해야 하는 작업들은 트랜잭션을 사용하세요:

```rust
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.insert("profiles", b"profile:1", b"...")?;
tx.commit()?;
```

자세한 내용은 [트랜잭션 가이드](transactions)를 참조하세요.

---

## 다음 단계

- [트랜잭션 가이드](transactions) — MVCC 트랜잭션 알아보기
- [SQL 레퍼런스](sql-reference) — 복잡한 쿼리를 위한 SQL 사용
- [API 레퍼런스](../api/database) — 전체 API 문서
