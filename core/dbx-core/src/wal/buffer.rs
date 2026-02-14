use crate::wal::WalRecord;
use std::io;
use std::sync::Mutex;

/// WAL Buffer - Memory buffer to minimize disk I/O frequency
pub struct WalBuffer {
    buffer: Mutex<Vec<WalRecord>>,
    auto_flush_threshold: usize,
}

impl WalBuffer {
    /// Creates a new WAL buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Mutex::new(Vec::with_capacity(capacity)),
            auto_flush_threshold: capacity * 3 / 4, // Auto-flush at 75%
        }
    }

    /// Adds a record to the buffer
    pub fn push(&self, record: WalRecord) {
        let mut buf = self.buffer.lock().unwrap();
        buf.push(record);

        // 자동 플러시 체크
        if buf.len() >= self.auto_flush_threshold {
            drop(buf); // 락 해제
            let _ = self.flush();
        }
    }

    /// Flushes buffer to disk
    pub fn flush(&self) -> std::io::Result<()> {
        let mut buf = self.buffer.lock().unwrap();
        if buf.is_empty() {
            return Ok(());
        }

        // TODO: 실제 WAL 파일에 쓰기 (비동기 I/O)
        // 현재는 버퍼만 비움
        buf.clear();
        Ok(())
    }

    /// Checks buffer usage
    pub fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Checks if buffer is empty
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
