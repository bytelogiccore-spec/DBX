//! WosPage — 4KB 페이지 포맷으로 key-value 엔트리를 직렬화/역직렬화한다.
//!
//! 포맷:
//!   [magic: 4][version: 2][count: 4][entries...][crc32: 4]
//!
//! entry:
//!   [key_len: 4][key: N][val_len: 4][val: M][deleted: 1]

use crate::error::{DbxError, DbxResult};

const MAGIC: u32 = 0xD8B_B00F;
const VERSION: u16 = 1;

/// 페이지 내 단일 key-value 엔트리
#[derive(Clone)]
pub struct PageEntry {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub deleted: bool,
}

/// 직렬화 가능한 페이지 — 여러 엔트리의 컨테이너
pub struct WosPage {
    pub entries: Vec<PageEntry>,
}

impl WosPage {
    pub fn from_entries(entries: Vec<PageEntry>) -> Self {
        Self { entries }
    }

    /// 페이지를 바이트로 직렬화. 끝에 CRC32 체크섬 포함.
    pub fn serialize(&self) -> DbxResult<Vec<u8>> {
        let mut buf = Vec::new();

        buf.extend_from_slice(&MAGIC.to_le_bytes());
        buf.extend_from_slice(&VERSION.to_le_bytes());
        buf.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());

        for e in &self.entries {
            buf.extend_from_slice(&(e.key.len() as u32).to_le_bytes());
            buf.extend_from_slice(&e.key);
            buf.extend_from_slice(&(e.value.len() as u32).to_le_bytes());
            buf.extend_from_slice(&e.value);
            buf.push(e.deleted as u8);
        }

        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());

        Ok(buf)
    }

    /// 바이트에서 페이지 역직렬화. CRC32 불일치 시 에러 반환.
    pub fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
        if bytes.len() < 14 {
            return Err(DbxError::Storage("WosPage: too short".into()));
        }

        // CRC32 검증 (마지막 4바이트 제외한 나머지)
        let payload = &bytes[..bytes.len() - 4];
        let stored_crc = u32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap());
        if crc32fast::hash(payload) != stored_crc {
            return Err(DbxError::Storage("WosPage: checksum mismatch".into()));
        }

        let mut cur = 0usize;

        let _magic = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap());
        cur += 4;
        let _ver = u16::from_le_bytes(bytes[cur..cur + 2].try_into().unwrap());
        cur += 2;
        let count = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()) as usize;
        cur += 4;

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let klen = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()) as usize;
            cur += 4;
            let key = bytes[cur..cur + klen].to_vec();
            cur += klen;

            let vlen = u32::from_le_bytes(bytes[cur..cur + 4].try_into().unwrap()) as usize;
            cur += 4;
            let value = bytes[cur..cur + vlen].to_vec();
            cur += vlen;

            let deleted = bytes[cur] != 0;
            cur += 1;

            entries.push(PageEntry {
                key,
                value,
                deleted,
            });
        }

        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &[u8], value: &[u8]) -> PageEntry {
        PageEntry {
            key: key.to_vec(),
            value: value.to_vec(),
            deleted: false,
        }
    }

    fn deleted(key: &[u8]) -> PageEntry {
        PageEntry {
            key: key.to_vec(),
            value: vec![],
            deleted: true,
        }
    }

    #[test]
    fn page_roundtrip() {
        let entries = vec![entry(b"aaa", b"val1"), entry(b"bbb", b"val2")];
        let page = WosPage::from_entries(entries);
        let bytes = page.serialize().unwrap();
        let decoded = WosPage::deserialize(&bytes).unwrap();
        assert_eq!(decoded.entries.len(), 2);
        assert_eq!(decoded.entries[0].key, b"aaa");
        assert_eq!(decoded.entries[0].value, b"val1");
        assert_eq!(decoded.entries[1].key, b"bbb");
        assert_eq!(decoded.entries[1].value, b"val2");
    }

    #[test]
    fn page_checksum_detected() {
        let page = WosPage::from_entries(vec![entry(b"k", b"v")]);
        let mut bytes = page.serialize().unwrap();
        let len = bytes.len();
        bytes[len - 1] ^= 0xFF; // 체크섬 오염
        assert!(WosPage::deserialize(&bytes).is_err());
    }

    #[test]
    fn deleted_entry_roundtrip() {
        let entries = vec![entry(b"k1", b"v1"), deleted(b"k2")];
        let page = WosPage::from_entries(entries);
        let bytes = page.serialize().unwrap();
        let decoded = WosPage::deserialize(&bytes).unwrap();
        assert!(!decoded.entries[0].deleted);
        assert!(decoded.entries[1].deleted);
        assert!(decoded.entries[1].value.is_empty());
    }

    #[test]
    fn empty_page_roundtrip() {
        let page = WosPage::from_entries(vec![]);
        let bytes = page.serialize().unwrap();
        let decoded = WosPage::deserialize(&bytes).unwrap();
        assert_eq!(decoded.entries.len(), 0);
    }

    #[test]
    fn large_value_roundtrip() {
        let big_val = vec![0xABu8; 65536];
        let entries = vec![PageEntry {
            key: b"bigkey".to_vec(),
            value: big_val.clone(),
            deleted: false,
        }];
        let page = WosPage::from_entries(entries);
        let bytes = page.serialize().unwrap();
        let decoded = WosPage::deserialize(&bytes).unwrap();
        assert_eq!(decoded.entries[0].value, big_val);
    }
}
