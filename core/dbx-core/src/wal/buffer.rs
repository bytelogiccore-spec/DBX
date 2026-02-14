use crate::wal::WalRecord;
use std::io;
use std::sync::{Arc, Mutex};

/// WAL 버퍼 - 디스크 I/O 빈도를 최소화하기 위한 메모리 버퍼
pub struct WALBuffer {
    buffer: Arc<Mutex<Vec<u8>>>,
    auto_flush_threshold: usize,
}

impl WALBuffer {
    /// 새 WAL 버퍼 생성
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            auto_flush_threshold: capacity * 3 / 4, // 75% 도달 시 자동 플러시
        }
    }

    /// 레코드를 버퍼에 추가
    pub fn append(&self, record: &WalRecord) -> io::Result<()> {
        let serialized = bincode::serialize(record)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut buf = self.buffer.lock().unwrap();
        buf.extend_from_slice(&serialized);

        // 자동 플러시 체크
        if buf.len() >= self.auto_flush_threshold {
            drop(buf); // 락 해제
            self.flush()?;
        }

        Ok(())
    }

    /// 버퍼를 디스크로 플러시
    pub fn flush(&self) -> io::Result<()> {
        let mut buf = self.buffer.lock().unwrap();
        if buf.is_empty() {
            return Ok(());
        }

        // TODO: 실제 WAL 파일에 쓰기 (비동기 I/O)
        // 현재는 버퍼만 비움
        buf.clear();
        Ok(())
    }

    /// 버퍼 사용량 확인
    pub fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// 버퍼가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.buffer.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_buffer_append() {
        let buffer = WALBuffer::new(1024);
        let record = WalRecord::Insert {
            table: "test".to_string(),
            key: vec![1, 2, 3],
            value: vec![4, 5, 6],
            ts: 0,
        };

        assert!(buffer.append(&record).is_ok());
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_wal_buffer_flush() {
        let buffer = WALBuffer::new(1024);
        let record = WalRecord::Insert {
            table: "test".to_string(),
            key: vec![1, 2, 3],
            value: vec![4, 5, 6],
            ts: 0,
        };

        buffer.append(&record).unwrap();
        assert!(buffer.flush().is_ok());
        assert!(buffer.is_empty());
    }
}
