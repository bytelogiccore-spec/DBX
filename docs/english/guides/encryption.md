---
layout: default
title: Encryption
parent: English
nav_order: 25
---

# Encryption

DBX supports strong encryption using AES-256-GCM-SIV and ChaCha20-Poly1305.

## Creating Encrypted Database

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

fn main() -> dbx_core::DbxResult<()> {
    // Create encryption config from password
    let enc = EncryptionConfig::from_password("my-secret-password");
    
    // Open encrypted database
    let db = Database::open_encrypted("./secure-data", enc)?;
    
    // Use normally
    db.insert("secrets", b"api-key", b"sk-1234567890")?;
    
    Ok(())
}
```

## In-Memory Encrypted Database

```rust
let enc = EncryptionConfig::from_password("password");
let db = Database::open_in_memory_encrypted(enc)?;

db.insert("temp-secrets", b"session", b"xyz123")?;
```

## Encryption Algorithms

### AES-256-GCM-SIV (Default)

```rust
use dbx_core::storage::encryption::{EncryptionConfig, Algorithm};

let enc = EncryptionConfig::new(
    b"32-byte-key-here-32-byte-key-he",
    Algorithm::Aes256GcmSiv,
);

let db = Database::open_encrypted("./data", enc)?;
```

### ChaCha20-Poly1305

```rust
let enc = EncryptionConfig::new(
    b"32-byte-key-here-32-byte-key-he",
    Algorithm::ChaCha20Poly1305,
);

let db = Database::open_encrypted("./data", enc)?;
```

## Key Derivation from Password

DBX uses PBKDF2 with SHA-256 for key derivation:

```rust
// Automatically derives 256-bit key from password
let enc = EncryptionConfig::from_password("my-password");

// Custom iterations (default: 100,000)
let enc = EncryptionConfig::from_password_with_iterations(
    "my-password",
    200_000,
);
```

## Key Rotation

Rotate encryption keys without data loss:

```rust
let db = Database::open_encrypted("./data", old_enc)?;

// Insert some data
db.insert("users", b"user:1", b"Alice")?;
db.flush()?;

// Rotate to new key
let new_enc = EncryptionConfig::from_password("new-password");
let count = db.rotate_key(new_enc)?;

println!("Rotated {} records", count);

// Data still accessible
assert_eq!(db.get("users", b"user:1")?, Some(b"Alice".to_vec()));
```

## Re-opening with New Key

```rust
// Close database
drop(db);

// Re-open with new key
let new_enc = EncryptionConfig::from_password("new-password");
let db = Database::open_encrypted("./data", new_enc)?;

// Data accessible with new key
let value = db.get("users", b"user:1")?;
```

## Checking Encryption Status

```rust
// Check if database is encrypted
if db.is_encrypted() {
    println!("Database is encrypted");
} else {
    println!("Database is plain");
}
```

## Performance Impact

Encryption adds minimal overhead:

```rust
use std::time::Instant;

// Plain database
let plain_db = Database::open("./plain-data")?;
let start = Instant::now();
for i in 0..10000 {
    plain_db.insert("test", &i.to_le_bytes(), b"value")?;
}
println!("Plain: {:?}", start.elapsed());

// Encrypted database
let enc = EncryptionConfig::from_password("password");
let enc_db = Database::open_encrypted("./enc-data", enc)?;
let start = Instant::now();
for i in 0..10000 {
    enc_db.insert("test", &i.to_le_bytes(), b"value")?;
}
println!("Encrypted: {:?}", start.elapsed());

// Typical overhead: ~10-15%
```

## Secure Key Management

### Environment Variables

```rust
use std::env;

let password = env::var("DB_PASSWORD")
    .expect("DB_PASSWORD environment variable not set");

let enc = EncryptionConfig::from_password(&password);
let db = Database::open_encrypted("./data", enc)?;
```

### Key Files

```rust
use std::fs;

// Read key from file
let key = fs::read("./keys/db.key")?;
assert_eq!(key.len(), 32); // 256 bits

let enc = EncryptionConfig::new(&key, Algorithm::Aes256GcmSiv);
let db = Database::open_encrypted("./data", enc)?;
```

## Complete Example: Encrypted User Database

```rust
use dbx_core::Database;
use dbx_core::storage::encryption::EncryptionConfig;

fn main() -> dbx_core::DbxResult<()> {
    // Create encrypted database
    let enc = EncryptionConfig::from_password("super-secret-password");
    let db = Database::open_encrypted("./user-data", enc)?;
    
    // Store sensitive data
    db.insert("users", b"user:1", b"Alice:alice@example.com:+1234567890")?;
    db.insert("users", b"user:2", b"Bob:bob@example.com:+0987654321")?;
    db.insert("api-keys", b"service-1", b"sk-1234567890abcdef")?;
    
    // Flush to encrypted storage
    db.flush()?;
    
    // Verify encryption
    assert!(db.is_encrypted());
    
    // Data is encrypted at rest
    println!("Data encrypted and stored securely");
    
    Ok(())
}
```

## Security Best Practices

1. **Use strong passwords**: Minimum 12 characters, mix of letters/numbers/symbols
2. **Store keys securely**: Use environment variables or key management systems
3. **Rotate keys regularly**: Use `rotate_key()` periodically
4. **Don't hardcode passwords**: Load from secure configuration
5. **Use HTTPS**: When transmitting passwords over network

## Next Steps

- [Transactions](transactions) — Combine encryption with ACID transactions
- [Architecture](../architecture) — Learn about encryption internals
- [Benchmarks](../benchmarks) — Encryption performance impact
