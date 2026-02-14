---
layout: default
title: Encryption
parent: Examples
nav_order: 3
---

# Encryption Quick Start

The fastest way to use encryption in DBX.

## 1. Creating an Encrypted Database

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

let enc = EncryptionConfig::from_password("my-secret-password");
let db = Database::open_encrypted("./secure-data", enc)?;
```

## 2. Inserting and Querying Data

```rust
// Insert data (automatic encryption)
db.insert("secrets", b"api-key", b"sk-1234567890")?;

// Query data (automatic decryption)
let value = db.get("secrets", b"api-key")?;
```

## 3. Choosing Encryption Algorithm

```rust
use dbx_core::storage::encryption::Algorithm;

// AES-256-GCM-SIV (Default)
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

## 4. Key Rotation

```rust
// Open with existing key
let db = Database::open_encrypted("./data", old_enc)?;

// Rotate to new key
let new_enc = EncryptionConfig::from_password("new-password");
db.rotate_key(new_enc)?;
```

## 5. Complete Example

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

fn main() -> dbx_core::DbxResult<()> {
    // Create encrypted database
    let enc = EncryptionConfig::from_password("super-secret");
    let db = Database::open_encrypted("./secure-db", enc)?;
    
    // Store sensitive data
    db.insert("users", b"user:1", b"Alice:alice@example.com")?;
    db.insert("api-keys", b"service-1", b"sk-1234567890")?;
    
    // Query data
    if let Some(value) = db.get("users", b"user:1")? {
        println!("User: {}", String::from_utf8_lossy(&value));
    }
    
    // Check encryption status
    assert!(db.is_encrypted());
    
    Ok(())
}
```

## Next Steps

- [**Encryption Guide**](../guides/encryption.md) — Complete encryption guide
- [**Compression**](compression.md) — Data compression
- [**Quick Start**](quick-start.md) — Basic CRUD
