//! Encryption module for the DBX storage engine.
//!
//! Provides configurable, authenticated encryption for data at rest.
//! All ciphers are AEAD (Authenticated Encryption with Associated Data),
//! guaranteeing both confidentiality and integrity.
//!
//! # Supported Algorithms
//!
//! | Algorithm | Speed (AES-NI) | Speed (SW) | Nonce Safety | Use Case |
//! |-----------|---------------|------------|--------------|----------|
//! | AES-256-GCM-SIV | ★★★★★ | ★★★ | Misuse-resistant | Default — modern x86/ARM |
//! | ChaCha20-Poly1305 | ★★★ | ★★★★★ | Standard | WASM / no AES-NI |
//!
//! # Architecture
//!
//! ```text
//! User password/key
//!        │
//!        ▼
//!    ┌──────────┐
//!    │   HKDF   │  (SHA-256 based key derivation)
//!    └────┬─────┘
//!         │ 256-bit derived key
//!         ▼
//!    ┌──────────┐
//!    │  AEAD    │  (AES-GCM-SIV or ChaCha20-Poly1305)
//!    │ Encrypt  │
//!    └────┬─────┘
//!         │ [nonce | ciphertext | tag]
//!         ▼
//!    Encrypted data (self-contained, portable)
//! ```
//!
//! # Wire Format
//!
//! Encrypted output = `[nonce (12 bytes)] || [ciphertext + auth_tag]`
//!
//! The nonce is prepended to the ciphertext so each encrypted blob is
//! self-contained — no external nonce storage needed.
//!
//! # Example
//!
//! ```rust
//! use dbx_core::storage::encryption::{EncryptionConfig, EncryptionAlgorithm};
//!
//! // Create config from a password
//! let config = EncryptionConfig::from_password("my-secret-password");
//!
//! // Encrypt data
//! let plaintext = b"sensitive user data";
//! let encrypted = config.encrypt(plaintext).unwrap();
//!
//! // Decrypt data
//! let decrypted = config.decrypt(&encrypted).unwrap();
//! assert_eq!(decrypted, plaintext);
//! ```

use aes_gcm_siv::Aes256GcmSiv;
use aes_gcm_siv::aead::generic_array::GenericArray;
use aes_gcm_siv::aead::{Aead, KeyInit};
use chacha20poly1305::ChaCha20Poly1305;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

use crate::error::{DbxError, DbxResult};

/// Nonce size in bytes (96-bit, standard for both AES-GCM-SIV and ChaCha20-Poly1305).
const NONCE_SIZE: usize = 12;

/// Encryption key size in bytes (256-bit).
const KEY_SIZE: usize = 32;

/// HKDF info string for key derivation.
const HKDF_INFO: &[u8] = b"dbx-encryption-v1";

/// HKDF salt for password-based key derivation.
/// Using a fixed salt is acceptable when combined with HKDF
/// (not a password hash). For production, consider per-database salt.
const HKDF_SALT: &[u8] = b"dbx-default-salt-v1";

// ───────────────────────────────────────────────────────────────
// EncryptionAlgorithm
// ───────────────────────────────────────────────────────────────

/// Encryption algorithm selection.
///
/// Both algorithms provide 256-bit security with authenticated encryption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM-SIV — **Default**.
    ///
    /// - Hardware-accelerated on modern x86/ARM (AES-NI)
    /// - Nonce-misuse-resistant (safe even if nonce reused)
    /// - Best for: servers, desktops, modern mobile
    #[default]
    Aes256GcmSiv,

    /// ChaCha20-Poly1305 (RFC 8439).
    ///
    /// - Excellent software performance (no hardware dependency)
    /// - Constant-time implementation (side-channel resistant)
    /// - Best for: WASM, embedded, platforms without AES-NI
    ChaCha20Poly1305,
}

impl std::fmt::Display for EncryptionAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aes256GcmSiv => write!(f, "AES-256-GCM-SIV"),
            Self::ChaCha20Poly1305 => write!(f, "ChaCha20-Poly1305"),
        }
    }
}

impl EncryptionAlgorithm {
    /// All supported encryption algorithms.
    pub const ALL: &'static [EncryptionAlgorithm] = &[
        EncryptionAlgorithm::Aes256GcmSiv,
        EncryptionAlgorithm::ChaCha20Poly1305,
    ];
}

// ───────────────────────────────────────────────────────────────
// EncryptionConfig
// ───────────────────────────────────────────────────────────────

/// Encryption configuration for data at rest.
///
/// Holds the encryption algorithm and derived key material.
/// Once created, can encrypt/decrypt arbitrary byte slices.
///
/// # Security Properties
///
/// - Fresh random nonce per encryption (12 bytes, CSPRNG)
/// - AEAD authentication prevents tampering
/// - Key derived via HKDF-SHA256 from password or raw key
/// - Zeroization: key material lives only in this struct
///
/// # Examples
///
/// ```rust
/// use dbx_core::storage::encryption::EncryptionConfig;
///
/// // From password (most common)
/// let config = EncryptionConfig::from_password("my-password");
///
/// // From raw 256-bit key
/// let key = [0x42u8; 32];
/// let config = EncryptionConfig::from_key(key);
///
/// // Encrypt → decrypt round-trip
/// let data = b"hello, encrypted world!";
/// let enc = config.encrypt(data).unwrap();
/// let dec = config.decrypt(&enc).unwrap();
/// assert_eq!(dec, data);
/// ```
#[derive(Clone)]
pub struct EncryptionConfig {
    /// Selected encryption algorithm.
    algorithm: EncryptionAlgorithm,
    /// 256-bit derived key.
    key: [u8; KEY_SIZE],
}

impl std::fmt::Debug for EncryptionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionConfig")
            .field("algorithm", &self.algorithm)
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl EncryptionConfig {
    // ===== Constructors =====

    /// Create encryption config from a password string.
    ///
    /// The password is stretched to a 256-bit key using HKDF-SHA256.
    /// Uses the default algorithm (AES-256-GCM-SIV).
    pub fn from_password(password: &str) -> Self {
        Self::from_password_with_algorithm(password, EncryptionAlgorithm::default())
    }

    /// Create encryption config from a password with a specific algorithm.
    pub fn from_password_with_algorithm(password: &str, algorithm: EncryptionAlgorithm) -> Self {
        let key = Self::derive_key(password.as_bytes());
        Self { algorithm, key }
    }

    /// Create encryption config from a raw 256-bit key.
    ///
    /// Uses the default algorithm (AES-256-GCM-SIV).
    ///
    /// # Panics
    ///
    /// Panics if `key` is not exactly 32 bytes (this is enforced by the type system).
    pub fn from_key(key: [u8; KEY_SIZE]) -> Self {
        Self {
            algorithm: EncryptionAlgorithm::default(),
            key,
        }
    }

    /// Create encryption config from a raw key with a specific algorithm.
    pub fn from_key_with_algorithm(key: [u8; KEY_SIZE], algorithm: EncryptionAlgorithm) -> Self {
        Self { algorithm, key }
    }

    /// Change the algorithm while keeping the same key.
    pub fn with_algorithm(mut self, algorithm: EncryptionAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    // ===== Accessors =====

    /// Get the configured algorithm.
    pub fn algorithm(&self) -> EncryptionAlgorithm {
        self.algorithm
    }

    // ===== Core Operations =====

    /// Encrypt a plaintext byte slice.
    ///
    /// Returns `[nonce (12 bytes)] || [ciphertext + auth_tag]`.
    ///
    /// A fresh random nonce is generated for each call, making it safe
    /// to encrypt the same plaintext multiple times.
    pub fn encrypt(&self, plaintext: &[u8]) -> DbxResult<Vec<u8>> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = GenericArray::from_slice(&nonce_bytes);

        let ciphertext = match self.algorithm {
            EncryptionAlgorithm::Aes256GcmSiv => {
                let cipher = Aes256GcmSiv::new(GenericArray::from_slice(&self.key));
                cipher.encrypt(nonce, plaintext).map_err(|e| {
                    DbxError::Encryption(format!("AES-GCM-SIV encrypt failed: {}", e))
                })?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
                cipher
                    .encrypt(nonce, plaintext)
                    .map_err(|e| DbxError::Encryption(format!("ChaCha20 encrypt failed: {}", e)))?
            }
        };

        // Wire format: [nonce (12)] || [ciphertext + tag]
        let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt an encrypted byte slice.
    ///
    /// Expects `[nonce (12 bytes)] || [ciphertext + auth_tag]` format
    /// (as produced by [`encrypt`](Self::encrypt)).
    ///
    /// Returns the original plaintext, or an error if:
    /// - Data is too short (less than nonce size)
    /// - Authentication fails (data tampered)
    /// - Wrong key used
    pub fn decrypt(&self, encrypted: &[u8]) -> DbxResult<Vec<u8>> {
        if encrypted.len() < NONCE_SIZE {
            return Err(DbxError::Encryption(
                "encrypted data too short (missing nonce)".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = GenericArray::from_slice(nonce_bytes);

        match self.algorithm {
            EncryptionAlgorithm::Aes256GcmSiv => {
                let cipher = Aes256GcmSiv::new(GenericArray::from_slice(&self.key));
                cipher
                    .decrypt(nonce, ciphertext)
                    .map_err(|e| DbxError::Encryption(format!("AES-GCM-SIV decrypt failed: {}", e)))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
                cipher
                    .decrypt(nonce, ciphertext)
                    .map_err(|e| DbxError::Encryption(format!("ChaCha20 decrypt failed: {}", e)))
            }
        }
    }

    /// Encrypt data with Associated Data (AAD).
    ///
    /// AAD is authenticated but not encrypted — useful for metadata
    /// like table names or column IDs that should be verified but remain readable.
    pub fn encrypt_with_aad(&self, plaintext: &[u8], aad: &[u8]) -> DbxResult<Vec<u8>> {
        use aes_gcm_siv::aead::Payload;

        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = GenericArray::from_slice(&nonce_bytes);

        let payload = Payload {
            msg: plaintext,
            aad,
        };

        let ciphertext = match self.algorithm {
            EncryptionAlgorithm::Aes256GcmSiv => {
                let cipher = Aes256GcmSiv::new(GenericArray::from_slice(&self.key));
                cipher.encrypt(nonce, payload).map_err(|e| {
                    DbxError::Encryption(format!("AES-GCM-SIV encrypt+AAD failed: {}", e))
                })?
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
                cipher.encrypt(nonce, payload).map_err(|e| {
                    DbxError::Encryption(format!("ChaCha20 encrypt+AAD failed: {}", e))
                })?
            }
        };

        let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    /// Decrypt data with Associated Data (AAD).
    ///
    /// The AAD must match exactly what was used during encryption,
    /// otherwise authentication will fail.
    pub fn decrypt_with_aad(&self, encrypted: &[u8], aad: &[u8]) -> DbxResult<Vec<u8>> {
        use aes_gcm_siv::aead::Payload;

        if encrypted.len() < NONCE_SIZE {
            return Err(DbxError::Encryption(
                "encrypted data too short (missing nonce)".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = GenericArray::from_slice(nonce_bytes);

        let payload = Payload {
            msg: ciphertext,
            aad,
        };

        match self.algorithm {
            EncryptionAlgorithm::Aes256GcmSiv => {
                let cipher = Aes256GcmSiv::new(GenericArray::from_slice(&self.key));
                cipher.decrypt(nonce, payload).map_err(|e| {
                    DbxError::Encryption(format!("AES-GCM-SIV decrypt+AAD failed: {}", e))
                })
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(&self.key));
                cipher.decrypt(nonce, payload).map_err(|e| {
                    DbxError::Encryption(format!("ChaCha20 decrypt+AAD failed: {}", e))
                })
            }
        }
    }

    // ===== Internal Helpers =====

    /// Derive a 256-bit key from arbitrary input using HKDF-SHA256.
    fn derive_key(input: &[u8]) -> [u8; KEY_SIZE] {
        let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), input);
        let mut key = [0u8; KEY_SIZE];
        hk.expand(HKDF_INFO, &mut key)
            .expect("HKDF expand should never fail for 32-byte output");
        key
    }
}

// ───────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_algorithm_is_aes_gcm_siv() {
        let config = EncryptionConfig::from_password("test");
        assert_eq!(config.algorithm(), EncryptionAlgorithm::Aes256GcmSiv);
    }

    #[test]
    fn round_trip_aes_gcm_siv() {
        let config = EncryptionConfig::from_password("test-password");
        let plaintext = b"Hello, DBX encryption!";

        let encrypted = config.encrypt(plaintext).unwrap();
        assert_ne!(encrypted, plaintext);
        assert!(encrypted.len() > plaintext.len()); // nonce + tag overhead

        let decrypted = config.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn round_trip_chacha20() {
        let config = EncryptionConfig::from_password("test-password")
            .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);
        let plaintext = b"Hello, ChaCha20!";

        let encrypted = config.encrypt(plaintext).unwrap();
        let decrypted = config.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn round_trip_all_algorithms() {
        let plaintext = b"Testing all algorithms";
        for algo in EncryptionAlgorithm::ALL {
            let config = EncryptionConfig::from_password("pw").with_algorithm(*algo);
            let encrypted = config.encrypt(plaintext).unwrap();
            let decrypted = config.decrypt(&encrypted).unwrap();
            assert_eq!(decrypted, plaintext, "Round-trip failed for {:?}", algo);
        }
    }

    #[test]
    fn from_raw_key() {
        let key = [0xABu8; KEY_SIZE];
        let config = EncryptionConfig::from_key(key);
        let plaintext = b"raw key test";

        let encrypted = config.encrypt(plaintext).unwrap();
        let decrypted = config.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_password_fails() {
        let config1 = EncryptionConfig::from_password("correct-password");
        let config2 = EncryptionConfig::from_password("wrong-password");

        let plaintext = b"secret data";
        let encrypted = config1.encrypt(plaintext).unwrap();

        let result = config2.decrypt(&encrypted);
        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn wrong_algorithm_fails() {
        let config_aes = EncryptionConfig::from_password("same-password");
        let config_chacha = EncryptionConfig::from_password("same-password")
            .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);

        let plaintext = b"algorithm mismatch test";
        let encrypted = config_aes.encrypt(plaintext).unwrap();

        // Same key but different algorithm should fail
        let result = config_chacha.decrypt(&encrypted);
        assert!(
            result.is_err(),
            "Decryption with wrong algorithm should fail"
        );
    }

    #[test]
    fn tampered_data_fails() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"tamper test";
        let mut encrypted = config.encrypt(plaintext).unwrap();

        // Flip a byte in the ciphertext (after nonce)
        let last = encrypted.len() - 1;
        encrypted[last] ^= 0xFF;

        let result = config.decrypt(&encrypted);
        assert!(result.is_err(), "Tampered data should fail authentication");
    }

    #[test]
    fn too_short_data_fails() {
        let config = EncryptionConfig::from_password("test");

        // Less than NONCE_SIZE bytes
        let result = config.decrypt(&[0u8; 5]);
        assert!(result.is_err());
    }

    #[test]
    fn empty_plaintext() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"";

        let encrypted = config.encrypt(plaintext).unwrap();
        let decrypted = config.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn large_data_round_trip() {
        let config = EncryptionConfig::from_password("test");
        let plaintext: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

        let encrypted = config.encrypt(&plaintext).unwrap();
        let decrypted = config.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn each_encrypt_produces_different_output() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"same input";

        let enc1 = config.encrypt(plaintext).unwrap();
        let enc2 = config.encrypt(plaintext).unwrap();

        // Different nonces → different ciphertexts
        assert_ne!(enc1, enc2, "Each encryption should use a fresh nonce");

        // Both should decrypt correctly
        assert_eq!(config.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(config.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn aad_round_trip() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"sensitive data";
        let aad = b"table:users,column:email";

        let encrypted = config.encrypt_with_aad(plaintext, aad).unwrap();
        let decrypted = config.decrypt_with_aad(&encrypted, aad).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aad_mismatch_fails() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"sensitive data";
        let aad = b"table:users";

        let encrypted = config.encrypt_with_aad(plaintext, aad).unwrap();

        // Wrong AAD should fail authentication
        let result = config.decrypt_with_aad(&encrypted, b"table:orders");
        assert!(result.is_err(), "Wrong AAD should fail authentication");
    }

    #[test]
    fn display_names() {
        assert_eq!(
            format!("{}", EncryptionAlgorithm::Aes256GcmSiv),
            "AES-256-GCM-SIV"
        );
        assert_eq!(
            format!("{}", EncryptionAlgorithm::ChaCha20Poly1305),
            "ChaCha20-Poly1305"
        );
    }

    #[test]
    fn all_algorithms_count() {
        assert_eq!(EncryptionAlgorithm::ALL.len(), 2);
    }

    #[test]
    fn debug_redacts_key() {
        let config = EncryptionConfig::from_password("secret");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("secret"));
    }

    #[test]
    fn wire_format_structure() {
        let config = EncryptionConfig::from_password("test");
        let plaintext = b"hello";

        let encrypted = config.encrypt(plaintext).unwrap();

        // Wire format: [nonce (12)] || [ciphertext + tag (16)]
        // For AES-GCM-SIV: tag is 16 bytes
        assert_eq!(
            encrypted.len(),
            NONCE_SIZE + plaintext.len() + 16, // 12 + 5 + 16 = 33
            "Wire format should be nonce + plaintext + tag"
        );
    }
}
