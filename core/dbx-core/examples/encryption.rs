//! DBX Example: Encryption
//!
//! This example demonstrates:
//! - AES-256-GCM-SIV encryption
//! - ChaCha20-Poly1305 encryption
//! - Reading and writing encrypted data

use dbx_core::{Database, DbxResult};
use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};
use std::path::Path;

fn main() -> DbxResult<()> {
    println!("=== DBX Example: Encryption ===\n");
    
    // Example 1: AES-256-GCM-SIV Encryption (Default)
    println!("--- AES-256-GCM-SIV (Default) ---");
    demo_aes_encryption()?;
    
    // Example 2: ChaCha20-Poly1305 Encryption
    println!("\n--- ChaCha20-Poly1305 ---");
    demo_chacha_encryption()?;
    
    // Example 3: In-memory encrypted database
    println!("\n--- In-Memory Encrypted ---");
    demo_in_memory_encrypted()?;
    
    Ok(())
}

fn demo_aes_encryption() -> DbxResult<()> {
    // Create encryption config with password (uses AES-256-GCM-SIV by default)
    let encryption = EncryptionConfig::from_password("my-secret-password");
    
    // Open database with encryption
    let db = Database::open_encrypted(Path::new("./encrypted_aes_db"), encryption)?;
    
    println!("✓ Database opened with AES-256-GCM-SIV encryption");
    
    // Insert sensitive data (automatically encrypted)
    db.insert("secrets", b"password:admin", b"super_secret_password_123")?;
    db.insert("secrets", b"api_key:prod", b"sk-1234567890abcdef")?;
    db.insert("secrets", b"credit_card", b"4111-1111-1111-1111")?;
    
    println!("✓ Inserted 3 encrypted entries");
    
    // Read encrypted data (automatically decrypted)
    if let Some(value) = db.get("secrets", b"password:admin")? {
        println!("✓ Retrieved password: {}", String::from_utf8_lossy(&value));
    }
    
    // Count encrypted entries
    let count = db.count("secrets")?;
    println!("✓ Total encrypted entries: {}", count);
    
    // Flush to disk (data is encrypted on disk)
    db.flush()?;
    println!("✓ Data encrypted and flushed to disk");
    
    Ok(())
}

fn demo_chacha_encryption() -> DbxResult<()> {
    // Create encryption config with ChaCha20-Poly1305
    let encryption = EncryptionConfig::from_password_with_algorithm(
        "my-secret-password",
        EncryptionAlgorithm::ChaCha20Poly1305
    );
    
    // Open database with encryption
    let db = Database::open_encrypted(Path::new("./encrypted_chacha_db"), encryption)?;
    
    println!("✓ Database opened with ChaCha20-Poly1305 encryption");
    
    // Insert sensitive data
    db.insert("users", b"user:1", b"Alice (Confidential)")?;
    db.insert("users", b"user:2", b"Bob (Top Secret)")?;
    
    println!("✓ Inserted 2 encrypted users");
    
    // Read encrypted data
    if let Some(value) = db.get("users", b"user:1")? {
        println!("✓ Retrieved user: {}", String::from_utf8_lossy(&value));
    }
    
    Ok(())
}

fn demo_in_memory_encrypted() -> DbxResult<()> {
    // Create in-memory encrypted database
    let encryption = EncryptionConfig::from_password("temp-secret");
    let db = Database::open_in_memory_encrypted(encryption)?;
    
    println!("✓ In-memory encrypted database created");
    
    // Insert temporary encrypted data
    db.insert("cache", b"session:123", b"temporary_token_xyz")?;
    db.insert("cache", b"session:456", b"another_token_abc")?;
    
    println!("✓ Inserted 2 encrypted cache entries");
    
    // Read data
    if let Some(value) = db.get("cache", b"session:123")? {
        println!("✓ Retrieved session: {}", String::from_utf8_lossy(&value));
    }
    
    println!("\n=== Security Notes ===");
    println!("• Data is encrypted at rest (on disk)");
    println!("• Data is encrypted in WAL (Write-Ahead Log)");
    println!("• Encryption key must be provided on database open");
    println!("• Lost keys = lost data (no recovery possible)");
    println!("• AES-256-GCM-SIV: Hardware-accelerated, nonce-misuse-resistant");
    println!("• ChaCha20-Poly1305: Excellent software performance");
    
    Ok(())
}
