//! WAL (Write-Ahead Log) 레코드 직렬화 / 역직렬화
//!
//! ## 레코드 포맷
//! ```text
//! [key_len: 4LE][key][val_len: 4LE][val][deleted: 1][crc32: 4LE]
//! ```
//!
//! - CRC32는 key_len..deleted 전체에 대해 계산한다.
//! - 파일 끝(EOF)나 CRC 불일치는 replaying 시 gracefully 무시한다.

use std::io::{self, Read, Write};

/// WAL 단일 레코드
pub struct WalRecord {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub deleted: bool,
}

impl WalRecord {
    /// 레코드를 바이트로 인코딩 (append용)
    pub fn encode(&self) -> Vec<u8> {
        let mut buf =
            Vec::with_capacity(4 + self.key.len() + 4 + self.value.len() + 1 + 4);
        buf.extend_from_slice(&(self.key.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.key);
        buf.extend_from_slice(&(self.value.len() as u32).to_le_bytes());
        buf.extend_from_slice(&self.value);
        buf.push(self.deleted as u8);
        let crc = crc32fast::hash(&buf);
        buf.extend_from_slice(&crc.to_le_bytes());
        buf
    }

    /// reader에서 레코드 하나를 디코딩.
    /// - `Ok(Some(r))`: 정상 레코드
    /// - `Ok(None)`:   EOF (정상 종료)
    /// - `Err(...)`:   CRC 불일치 또는 I/O 에러 (replay 시 해당 레코드부터 잘린 것으로 처리)
    pub fn decode_from(reader: &mut impl Read) -> io::Result<Option<Self>> {
        // key_len
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        let klen = u32::from_le_bytes(len_buf) as usize;

        let mut key = vec![0u8; klen];
        reader.read_exact(&mut key)?;

        // val_len
        reader.read_exact(&mut len_buf)?;
        let vlen = u32::from_le_bytes(len_buf) as usize;

        let mut value = vec![0u8; vlen];
        reader.read_exact(&mut value)?;

        // deleted
        let mut flag = [0u8; 1];
        reader.read_exact(&mut flag)?;
        let deleted = flag[0] != 0;

        // CRC32 검증
        let mut crc_buf = [0u8; 4];
        reader.read_exact(&mut crc_buf)?;
        let stored_crc = u32::from_le_bytes(crc_buf);

        // CRC 재계산
        let mut check = Vec::with_capacity(4 + klen + 4 + vlen + 1);
        check.extend_from_slice(&(klen as u32).to_le_bytes());
        check.extend_from_slice(&key);
        check.extend_from_slice(&(vlen as u32).to_le_bytes());
        check.extend_from_slice(&value);
        check.push(deleted as u8);

        if crc32fast::hash(&check) != stored_crc {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "WAL record CRC32 mismatch",
            ));
        }

        Ok(Some(WalRecord { key, value, deleted }))
    }
}

/// WAL 파일 전체를 replay해 레코드 목록 반환.
/// 불완전한 레코드(크래시로 인한 부분 쓰기)는 그 지점에서 중단하고 이전까지만 반환.
pub fn replay_wal(reader: &mut impl Read) -> Vec<WalRecord> {
    let mut records = Vec::new();
    loop {
        match WalRecord::decode_from(reader) {
            Ok(Some(r)) => records.push(r),
            Ok(None) => break,        // EOF
            Err(_) => break,          // 부분 쓰기 또는 손상 — 여기서 중단 (WAL truncate 필요)
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn roundtrip_put() {
        let r = WalRecord { key: b"hello".to_vec(), value: b"world".to_vec(), deleted: false };
        let enc = r.encode();
        let mut cur = Cursor::new(&enc);
        let dec = WalRecord::decode_from(&mut cur).unwrap().unwrap();
        assert_eq!(dec.key, b"hello");
        assert_eq!(dec.value, b"world");
        assert!(!dec.deleted);
    }

    #[test]
    fn roundtrip_delete() {
        let r = WalRecord { key: b"key".to_vec(), value: vec![], deleted: true };
        let enc = r.encode();
        let mut cur = Cursor::new(&enc);
        let dec = WalRecord::decode_from(&mut cur).unwrap().unwrap();
        assert_eq!(dec.key, b"key");
        assert!(dec.deleted);
    }

    #[test]
    fn eof_returns_none() {
        let mut cur = Cursor::new(b"");
        assert!(WalRecord::decode_from(&mut cur).unwrap().is_none());
    }

    #[test]
    fn crc_mismatch_returns_err() {
        let r = WalRecord { key: b"k".to_vec(), value: b"v".to_vec(), deleted: false };
        let mut enc = r.encode();
        let n = enc.len();
        enc[n - 1] ^= 0xFF; // CRC 오염
        let mut cur = Cursor::new(&enc);
        assert!(WalRecord::decode_from(&mut cur).is_err());
    }

    #[test]
    fn replay_stops_at_corruption() {
        let r1 = WalRecord { key: b"a".to_vec(), value: b"1".to_vec(), deleted: false };
        let r2 = WalRecord { key: b"b".to_vec(), value: b"2".to_vec(), deleted: false };
        let mut data = r1.encode();
        let mut bad = r2.encode();
        let bad_len = bad.len();
        bad[bad_len - 1] ^= 0xFF; // 두 번째 레코드 CRC 오염
        data.extend_from_slice(&bad);
        let mut cur = Cursor::new(&data);
        let records = replay_wal(&mut cur);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].key, b"a");
    }
}
