//! Encrypted WOS (Write-Optimized Store) — transparent encryption wrapper.
//!
//! Wraps [`WosBackend`] to encrypt all values before storage and decrypt on read.
//! Keys remain unencrypted to support range scans and ordered iteration.
//!
//! # Architecture
//!
//! ```text
//! Application
//!     │ plaintext value
//!     ▼
//! EncryptedWosBackend
//!     │ encrypt(value) → ciphertext
//!     ▼
//! WosBackend (sled)
//!     │ store ciphertext
//!     ▼
//! Disk
//! ```
//!
//! # Security Properties
//!
//! - Values are encrypted with AEAD (confidentiality + integrity)
//! - Keys are stored in plaintext (trade-off for range scan support)
//! - AAD includes table name for cross-table attack prevention

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use crate::storage::encryption::EncryptionConfig;
use crate::storage::wos::WosBackend;
use std::ops::RangeBounds;
use std::path::Path;

/// Tier 3 with transparent encryption: sled-backed storage with AEAD encryption.
///
/// All values are encrypted before being written to sled and decrypted on read.
/// Keys remain in plaintext to preserve ordered iteration and range scans.
///
/// # Examples
///
/// ```rust,no_run
/// use dbx_core::storage::encrypted_wos::EncryptedWosBackend;
/// use dbx_core::storage::encryption::EncryptionConfig;
/// use std::path::Path;
///
/// let encryption = EncryptionConfig::from_password("my-secret");
/// let backend = EncryptedWosBackend::open(Path::new("./data"), encryption).unwrap();
/// ```
pub struct EncryptedWosBackend {
    inner: WosBackend,
    encryption: EncryptionConfig,
}

impl EncryptedWosBackend {
    /// Open an encrypted WOS at the given directory path.
    pub fn open(path: &Path, encryption: EncryptionConfig) -> DbxResult<Self> {
        let inner = WosBackend::open(path)?;
        Ok(Self { inner, encryption })
    }

    /// Open a temporary encrypted WOS (for testing).
    pub fn open_temporary(encryption: EncryptionConfig) -> DbxResult<Self> {
        let inner = WosBackend::open_temporary()?;
        Ok(Self { inner, encryption })
    }

    /// Get a reference to the encryption config.
    pub fn encryption_config(&self) -> &EncryptionConfig {
        &self.encryption
    }

    /// Re-key all data with a new encryption config.
    ///
    /// Reads all existing data, decrypts with the current key,
    /// and re-encrypts with the new key.
    ///
    /// # Warning
    ///
    /// This operation is NOT atomic — if interrupted, some data may be
    /// encrypted with the old key and some with the new key.
    /// Always checkpoint/backup before re-keying.
    pub fn rekey(&mut self, new_encryption: EncryptionConfig) -> DbxResult<usize> {
        let table_names = self.inner.table_names()?;
        let mut rekey_count = 0;

        for table_name in &table_names {
            // Read all entries with current key
            let entries: Vec<(Vec<u8>, Vec<u8>)> = self
                .inner
                .scan(table_name, ..)?
                .into_iter()
                .filter_map(|(key, encrypted_value)| {
                    // Decrypt with old key
                    let aad = table_name.as_bytes();
                    self.encryption
                        .decrypt_with_aad(&encrypted_value, aad)
                        .ok()
                        .map(|plain| (key, plain))
                })
                .collect();

            // Re-encrypt with new key and write back
            for (key, plaintext) in &entries {
                let aad = table_name.as_bytes();
                let new_ciphertext = new_encryption.encrypt_with_aad(plaintext, aad)?;
                self.inner.insert(table_name, key, &new_ciphertext)?;
                rekey_count += 1;
            }
        }

        self.encryption = new_encryption;
        self.inner.flush()?;
        Ok(rekey_count)
    }
}

impl StorageBackend for EncryptedWosBackend {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        let aad = table.as_bytes();
        let encrypted = self.encryption.encrypt_with_aad(value, aad)?;
        self.inner.insert(table, key, &encrypted)
    }

    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        let aad = table.as_bytes();
        let encrypted_rows: Vec<(Vec<u8>, Vec<u8>)> = rows
            .into_iter()
            .map(|(key, value)| {
                let encrypted = self.encryption.encrypt_with_aad(&value, aad)?;
                Ok((key, encrypted))
            })
            .collect::<DbxResult<Vec<_>>>()?;

        self.inner.insert_batch(table, encrypted_rows)
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        match self.inner.get(table, key)? {
            Some(encrypted) => {
                let aad = table.as_bytes();
                let decrypted = self.encryption.decrypt_with_aad(&encrypted, aad)?;
                Ok(Some(decrypted))
            }
            None => Ok(None),
        }
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        self.inner.delete(table, key)
    }

    fn scan<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let encrypted_entries = self.inner.scan(table, range)?;
        let aad = table.as_bytes();

        encrypted_entries
            .into_iter()
            .map(|(key, encrypted)| {
                let decrypted = self.encryption.decrypt_with_aad(&encrypted, aad)?;
                Ok((key, decrypted))
            })
            .collect()
    }

    fn scan_one<R: RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        let aad = table.as_bytes();
        match self.inner.scan_one(table, range)? {
            Some((key, encrypted)) => {
                let decrypted = self.encryption.decrypt_with_aad(&encrypted, aad)?;
                Ok(Some((key, decrypted)))
            }
            None => Ok(None),
        }
    }

    fn flush(&self) -> DbxResult<()> {
        self.inner.flush()
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        self.inner.count(table)
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        self.inner.table_names()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::encryption::EncryptionAlgorithm;

    fn encrypted_wos() -> EncryptedWosBackend {
        let enc = EncryptionConfig::from_password("test-password");
        EncryptedWosBackend::open_temporary(enc).unwrap()
    }

    #[test]
    fn insert_and_get_round_trip() {
        let wos = encrypted_wos();
        wos.insert("users", b"key1", b"Alice").unwrap();
        let result = wos.get("users", b"key1").unwrap();
        assert_eq!(result, Some(b"Alice".to_vec()));
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let wos = encrypted_wos();
        assert_eq!(wos.get("users", b"missing").unwrap(), None);
    }

    #[test]
    fn delete_existing() {
        let wos = encrypted_wos();
        wos.insert("users", b"key1", b"Alice").unwrap();
        assert!(wos.delete("users", b"key1").unwrap());
        assert_eq!(wos.get("users", b"key1").unwrap(), None);
    }

    #[test]
    fn upsert_overwrites() {
        let wos = encrypted_wos();
        wos.insert("t", b"k", b"v1").unwrap();
        wos.insert("t", b"k", b"v2").unwrap();
        assert_eq!(wos.get("t", b"k").unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn scan_all_decrypted() {
        let wos = encrypted_wos();
        wos.insert("t", b"a", b"1").unwrap();
        wos.insert("t", b"b", b"2").unwrap();
        wos.insert("t", b"c", b"3").unwrap();

        let all = wos.scan("t", ..).unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], (b"a".to_vec(), b"1".to_vec()));
        assert_eq!(all[1], (b"b".to_vec(), b"2".to_vec()));
        assert_eq!(all[2], (b"c".to_vec(), b"3".to_vec()));
    }

    #[test]
    fn count_accuracy() {
        let wos = encrypted_wos();
        assert_eq!(wos.count("t").unwrap(), 0);
        wos.insert("t", b"a", b"1").unwrap();
        wos.insert("t", b"b", b"2").unwrap();
        assert_eq!(wos.count("t").unwrap(), 2);
    }

    #[test]
    fn table_names_tracks_tables() {
        let wos = encrypted_wos();
        wos.insert("users", b"a", b"1").unwrap();
        wos.insert("orders", b"b", b"2").unwrap();
        let mut names = wos.table_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["orders".to_string(), "users".to_string()]);
    }

    #[test]
    fn wrong_password_cannot_decrypt() {
        let enc1 = EncryptionConfig::from_password("correct");
        let enc2 = EncryptionConfig::from_password("wrong");

        let wos = EncryptedWosBackend::open_temporary(enc1).unwrap();
        wos.insert("t", b"k", b"secret").unwrap();

        // Read raw encrypted value from inner sled
        let raw = wos.inner.get("t", b"k").unwrap().unwrap();

        // Trying to decrypt with wrong key should fail
        let result = enc2.decrypt_with_aad(&raw, b"t");
        assert!(result.is_err());
    }

    #[test]
    fn cross_table_aad_prevents_swap() {
        let wos = encrypted_wos();
        wos.insert("table_a", b"k", b"data_a").unwrap();

        // Read raw encrypted value from table_a
        let raw = wos.inner.get("table_a", b"k").unwrap().unwrap();

        // Write it to table_b as if it were valid
        wos.inner.insert("table_b", b"k", &raw).unwrap();

        // Reading from table_b should fail (wrong AAD)
        let result = wos.get("table_b", b"k");
        assert!(result.is_err(), "Cross-table AAD should prevent decryption");
    }

    #[test]
    fn insert_batch_encrypted() {
        let wos = encrypted_wos();
        let rows = vec![
            (b"k1".to_vec(), b"v1".to_vec()),
            (b"k2".to_vec(), b"v2".to_vec()),
            (b"k3".to_vec(), b"v3".to_vec()),
        ];
        wos.insert_batch("t", rows).unwrap();

        assert_eq!(wos.get("t", b"k1").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(wos.get("t", b"k2").unwrap(), Some(b"v2".to_vec()));
        assert_eq!(wos.get("t", b"k3").unwrap(), Some(b"v3".to_vec()));
    }

    #[test]
    fn rekey_preserves_data() {
        let enc_old = EncryptionConfig::from_password("old-password");
        let enc_new = EncryptionConfig::from_password("new-password")
            .with_algorithm(EncryptionAlgorithm::ChaCha20Poly1305);

        let mut wos = EncryptedWosBackend::open_temporary(enc_old).unwrap();
        wos.insert("users", b"alice", b"Alice Data").unwrap();
        wos.insert("users", b"bob", b"Bob Data").unwrap();
        wos.insert("orders", b"order1", b"Order Data").unwrap();

        let rekeyed = wos.rekey(enc_new).unwrap();
        assert_eq!(rekeyed, 3);

        // Verify data still readable with new key
        assert_eq!(
            wos.get("users", b"alice").unwrap(),
            Some(b"Alice Data".to_vec())
        );
        assert_eq!(
            wos.get("users", b"bob").unwrap(),
            Some(b"Bob Data".to_vec())
        );
        assert_eq!(
            wos.get("orders", b"order1").unwrap(),
            Some(b"Order Data".to_vec())
        );
    }

    #[test]
    fn flush_persists() {
        let wos = encrypted_wos();
        wos.insert("t", b"key", b"val").unwrap();
        wos.flush().unwrap();
        assert_eq!(wos.get("t", b"key").unwrap(), Some(b"val".to_vec()));
    }

    #[test]
    fn multiple_tables_isolation() {
        let wos = encrypted_wos();
        wos.insert("t1", b"k", b"v1").unwrap();
        wos.insert("t2", b"k", b"v2").unwrap();
        assert_eq!(wos.get("t1", b"k").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(wos.get("t2", b"k").unwrap(), Some(b"v2".to_vec()));
    }

    #[test]
    fn large_value_round_trip() {
        let wos = encrypted_wos();
        let large_value: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        wos.insert("t", b"big", &large_value).unwrap();
        let result = wos.get("t", b"big").unwrap().unwrap();
        assert_eq!(result, large_value);
    }
}
