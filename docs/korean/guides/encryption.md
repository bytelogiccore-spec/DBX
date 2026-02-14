---
layout: default
title: 암호화
parent: Guides
nav_order: 8
---

# 암호화
{: .no_toc }

DBX는 AES-256-GCM-SIV 및 ChaCha20-Poly1305 알고리즘을 사용하여 강력한 데이터 암호화를 지원합니다.
{: .fs-6 .fw-300 }

## 암호화된 데이터베이스 생성

이미 암호화된 데이터베이스를 열거나 새로 생성할 때는 `EncryptionConfig`를 사용합니다.

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

fn main() -> dbx_core::DbxResult<()> {
    // 비밀번호로부터 암호화 설정 생성
    let enc = EncryptionConfig::from_password("my-secret-password");
    
    // 암호화된 데이터베이스 열기
    let db = Database::open_encrypted("./secure-data", enc)?;
    
    // 평소와 같이 데이터 삽입 (자동 암호화)
    db.insert("secrets", b"api-key", b"sk-1234567890")?;
    
    Ok(())
}
```

## 지원 알고리즘

- **AES-256-GCM-SIV (기본값)**: 하드웨어 가속이 가능한 환경에서 높은 성능과 보안성을 제공합니다.
- **ChaCha20-Poly1305**: 소프트웨어 구현에서 빠르며 다양한 플랫폼에서 안정적입니다.

```rust
use dbx_core::storage::encryption::{EncryptionConfig, Algorithm};

let enc = EncryptionConfig::new(key_32_bytes, Algorithm::ChaCha20Poly1305);
```

## 키 회전 (Key Rotation)

데이터 손실 없이 암호화 키를 변경할 수 있습니다.

```rust
let new_enc = EncryptionConfig::from_password("new-password");
let count = db.rotate_key(new_enc)?; // 모든 레코드 재암호화
```

## 성능 영향

암호화를 사용하면 약간의 성능 오버헤드가 발생하지만, DBX의 병렬 처리 아키텍처를 통해 이를 최소화합니다.
- **일반적인 오버헤드**: 약 10~15% 수준

## 보안 권장 사항

1. **강력한 비밀번호 사용**: 최소 12자 이상의 영문/숫자/특수문자 조합을 권장합니다.
2. **키 관리**: 비밀번호를 코드에 직접 하드코딩하지 말고 환경 변수나 보안 설정 시스템에서 로드하세요.
3. **정기적 키 회전**: 정기적으로 `rotate_key()`를 호출하여 보안 수준을 유지하세요.

## 다음 단계

- [트랜잭션](transactions) — 암호화와 ACID 트랜잭션 결합
- [저장소 계층](storage-layers) — 암호화가 내부적으로 적용되는 방식 이해
- [언어 바인딩](language-bindings) — 다른 언어에서 암호화 사용하기
