use crate::error::{DbxError, DbxResult};
use std::convert::TryInto;

/// A versioned key for MVCC storage.
///
/// Encodes a user key and a timestamp into a single byte array,
/// allowing for efficient range scans and version retrieval.
///
/// Format: `[UserKey bytes] + [!Timestamp (8 bytes, Big Endian)]`
///
/// The timestamp is stored as bit-inverted (`!ts`) big-endian to ensure
/// that newer versions (higher timestamps) appear first in lexicographical order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedKey {
    pub user_key: Vec<u8>,
    pub commit_ts: u64,
}

impl PartialOrd for VersionedKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionedKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.user_key.cmp(&other.user_key) {
            std::cmp::Ordering::Equal => {
                // Descending order for timestamp: higher TS comes first
                other.commit_ts.cmp(&self.commit_ts)
            }
            ord => ord,
        }
    }
}

impl VersionedKey {
    /// Create a new versioned key.
    pub fn new(user_key: Vec<u8>, commit_ts: u64) -> Self {
        Self {
            user_key,
            commit_ts,
        }
    }

    /// Encode into bytes for storage.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.user_key.len() + 8);
        bytes.extend_from_slice(&self.user_key);
        // Invert timestamp for descending order (latest first)
        let inverted_ts = !self.commit_ts;
        bytes.extend_from_slice(&inverted_ts.to_be_bytes());
        bytes
    }

    /// Decode from storage bytes.
    pub fn decode(bytes: &[u8]) -> DbxResult<Self> {
        if bytes.len() < 8 {
            return Err(DbxError::Storage("Invalid versioned key length".into()));
        }

        let split_idx = bytes.len() - 8;
        let user_key = bytes[..split_idx].to_vec();

        let ts_bytes: [u8; 8] = bytes[split_idx..].try_into().unwrap(); // Safe due to len check
        let inverted_ts = u64::from_be_bytes(ts_bytes);
        let commit_ts = !inverted_ts;

        Ok(Self {
            user_key,
            commit_ts,
        })
    }

    /// Extract user key from encoded bytes without full decoding.
    pub fn extract_user_key(bytes: &[u8]) -> Option<&[u8]> {
        if bytes.len() < 8 {
            None
        } else {
            Some(&bytes[..bytes.len() - 8])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_versioned_key_encoding() {
        let key = b"key1".to_vec();
        let ts = 100;
        let vk = VersionedKey::new(key.clone(), ts);
        let encoded = vk.encode();

        assert_eq!(encoded.len(), key.len() + 8);

        let decoded = VersionedKey::decode(&encoded).unwrap();
        assert_eq!(decoded, vk);
    }

    #[test]
    fn test_version_ordering() {
        let key = b"key".to_vec();
        // Newer version (higher TS)
        let v2 = VersionedKey::new(key.clone(), 200);
        // Older version (lower TS)
        let v1 = VersionedKey::new(key.clone(), 100);

        // Lexicographical order: encoded v2 < encoded v1 because of inverted TS
        // !200 < !100
        assert!(v2.encode() < v1.encode());
    }
}
