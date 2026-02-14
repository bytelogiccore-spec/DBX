---
layout: default
title: Encryption
parent: 한국어
nav_order: 13
---

# Encryption Quick Start

DBX에서 암호화를 사용하는 가장 빠른 방법입니다.

## 1. 암호화된 데이터베이스 생성

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted("./secure-data", enc)?;
```

## 2. 데이터 삽입 및 조회

```rust
// 데이터 삽입 (자동 암호화)
db.insert("secrets", b"api-key", b"sk-1234567890")?;

// 데이터 조회 (자동 복호화)
let value = db.get("secrets", b"api-key")?;
```

## 3. 암호화 알고리즘 선택

```rust
use dbx_core::storage::encryption::Algorithm;

// AES-256-GCM-SIV (기본값)
let enc = EncryptionConfig::new(
    b"32-byte-key-here-32-byte-key-he",
    Algorithm::Aes256GcmSiv,
);

// ChaCha20-Poly1305
let enc = EncryptionConfig::new(
    b"32-byte-key-here-32-byte-key-he",
    Algorithm::ChaCha20Poly1305,
);
```

## 4. 키 회전

```rust
// 기존 키로 열기
let db = Database::open_encrypted("./data", old_enc)?;

// 새 키로 회전
let new_enc = EncryptionConfig::from_password("new-password");
db.rotate_key(new_enc)?;
```

## 5. 완전한 예제

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

fn main() -> dbx_core::DbxResult<()> {
    // 암호화된 데이터베이스 생성
    let enc = EncryptionConfig::from_password("super-secret");
    let db = Database::open_encrypted("./secure-db", enc)?;
    
    // 민감한 데이터 저장
    db.insert("users", b"user:1", b"Alice:alice@example.com")?;
    db.insert("api-keys", b"service-1", b"sk-1234567890")?;
    
    // 데이터 조회
    if let Some(value) = db.get("users", b"user:1")? {
        println!("User: {}", String::from_utf8_lossy(&value));
    }
    
    // 암호화 상태 확인
    assert!(db.is_encrypted());
    
    Ok(())
}
```

## Next Steps

- [**Encryption Guide**](../guides/encryption.md) — 완전한 암호화 가이드
- [**Compression**](compression.md) — 데이터 압축
- [**Quick Start**](quick-start.md) — 기본 CRUD
