//! Partitioned WAL Writer — Phase 2: Section 5.1
//!
//! 테이블별 독립 WAL 세그먼트로 병렬 쓰기를 가능하게 함

use crate::error::{DbxError, DbxResult};
use crate::wal::WalRecord;
use dashmap::DashMap;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// 파티션 기반 WAL 쓰기 엔진
///
/// 각 테이블이 독립적인 WAL 세그먼트를 가짐으로써
/// 서로 다른 테이블의 쓰기가 동시에 진행될 수 있습니다.
pub struct PartitionedWalWriter {
    /// 파티션별 WAL 버퍼: table_name → records
    partitions: DashMap<String, Vec<WalRecord>>,
    /// WAL 디렉토리
    wal_dir: PathBuf,
    /// 글로벌 시퀀스 번호 (원자적 증가)
    sequence: AtomicU64,
    /// 버퍼 플러시 임계값 (레코드 수)
    flush_threshold: usize,
}

impl PartitionedWalWriter {
    /// 새 파티션 WAL 생성
    pub fn new(wal_dir: PathBuf, flush_threshold: usize) -> DbxResult<Self> {
        if !wal_dir.exists() {
            std::fs::create_dir_all(&wal_dir).map_err(|source| DbxError::Io { source })?;
        }
        Ok(Self {
            partitions: DashMap::new(),
            wal_dir,
            sequence: AtomicU64::new(0),
            flush_threshold,
        })
    }

    /// 기본 설정 (플러시 임계값: 100)
    pub fn with_defaults(wal_dir: PathBuf) -> DbxResult<Self> {
        Self::new(wal_dir, 100)
    }

    /// WAL 레코드 추가 (테이블별 파티션에 버퍼링)
    pub fn append(&self, record: WalRecord) -> DbxResult<u64> {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let table = Self::extract_table(&record);

        let mut partition = self.partitions.entry(table).or_default();
        partition.push(record);

        // 임계값 도달 시 자동 플러시
        if partition.len() >= self.flush_threshold {
            let records = std::mem::take(&mut *partition);
            drop(partition); // DashMap lock 해제
            self.flush_records(&Self::extract_table(&records[0]), &records)?;
        }

        Ok(seq)
    }

    /// 여러 레코드를 한번에 추가 (배치 쓰기)
    pub fn append_batch(&self, records: Vec<WalRecord>) -> DbxResult<Vec<u64>> {
        let sequences: Vec<u64> = records
            .iter()
            .map(|_| self.sequence.fetch_add(1, Ordering::SeqCst))
            .collect();

        // 테이블별로 그룹화
        let mut grouped: std::collections::HashMap<String, Vec<WalRecord>> =
            std::collections::HashMap::new();
        for record in records {
            let table = Self::extract_table(&record);
            grouped.entry(table).or_default().push(record);
        }

        // 병렬로 각 파티션에 추가
        let results: Vec<DbxResult<()>> = grouped
            .into_par_iter()
            .map(|(table, partition_records)| {
                let mut partition = self.partitions.entry(table.clone()).or_default();
                partition.extend(partition_records);

                if partition.len() >= self.flush_threshold {
                    let records = std::mem::take(&mut *partition);
                    drop(partition);
                    self.flush_records(&table, &records)?;
                }
                Ok(())
            })
            .collect();

        // 에러 체크
        for result in results {
            result?;
        }

        Ok(sequences)
    }

    /// 모든 파티션 플러시 (병렬)
    pub fn flush_all(&self) -> DbxResult<usize> {
        let tables: Vec<String> = self.partitions.iter().map(|e| e.key().clone()).collect();

        let flushed: Vec<DbxResult<usize>> = tables
            .par_iter()
            .map(|table| {
                if let Some(mut partition) = self.partitions.get_mut(table) {
                    if partition.is_empty() {
                        return Ok(0);
                    }
                    let records = std::mem::take(&mut *partition);
                    let count = records.len();
                    drop(partition);
                    self.flush_records(table, &records)?;
                    Ok(count)
                } else {
                    Ok(0)
                }
            })
            .collect();

        let mut total = 0;
        for result in flushed {
            total += result?;
        }
        Ok(total)
    }

    /// 파티션 수 조회
    pub fn partition_count(&self) -> usize {
        self.partitions.len()
    }

    /// 버퍼에 있는 총 레코드 수
    pub fn buffered_count(&self) -> usize {
        self.partitions.iter().map(|e| e.value().len()).sum()
    }

    /// 현재 시퀀스 번호
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    // ─── Internal helpers ───────────────────────────────

    /// 레코드에서 테이블 이름 추출
    fn extract_table(record: &WalRecord) -> String {
        match record {
            WalRecord::Insert { table, .. } => table.clone(),
            WalRecord::Delete { table, .. } => table.clone(),
            WalRecord::Batch { table, .. } => table.clone(),
            WalRecord::Checkpoint { .. } => "__checkpoint__".to_string(),
            WalRecord::Commit { .. } => "__tx__".to_string(),
            WalRecord::Rollback { .. } => "__tx__".to_string(),
        }
    }

    /// 레코드를 디스크에 플러시
    fn flush_records(&self, table: &str, records: &[WalRecord]) -> DbxResult<()> {
        let safe_name = table.replace(['/', '\\', ':'], "_");
        let path = self.wal_dir.join(format!("{safe_name}.wal"));

        let serialized: Vec<u8> = records
            .iter()
            .flat_map(|r| {
                let mut buf = bincode::serialize(r).unwrap_or_default();
                let len = buf.len() as u32;
                let mut frame = len.to_le_bytes().to_vec();
                frame.append(&mut buf);
                frame
            })
            .collect();

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| DbxError::Io { source })?;
        file.write_all(&serialized)
            .map_err(|source| DbxError::Io { source })?;
        file.flush().map_err(|source| DbxError::Io { source })?;

        Ok(())
    }
}

/// 병렬 체크포인트 관리자
///
/// 여러 테이블의 체크포인트를 동시에 생성합니다.
pub struct ParallelCheckpointManager {
    wal_dir: PathBuf,
}

impl ParallelCheckpointManager {
    pub fn new(wal_dir: PathBuf) -> Self {
        Self { wal_dir }
    }

    /// 여러 테이블을 병렬로 체크포인트
    pub fn checkpoint_tables(&self, tables: &[String]) -> DbxResult<usize> {
        let results: Vec<DbxResult<()>> = tables
            .par_iter()
            .map(|table| {
                // 각 테이블의 WAL 파일을 체크포인트
                let safe_name = table.replace(['/', '\\', ':'], "_");
                let wal_path = self.wal_dir.join(format!("{safe_name}.wal"));
                let checkpoint_path = self.wal_dir.join(format!("{safe_name}.checkpoint"));

                if wal_path.exists() {
                    // WAL 내용을 체크포인트로 이동
                    std::fs::rename(&wal_path, &checkpoint_path)
                        .map_err(|source| DbxError::Io { source })?;
                }
                Ok(())
            })
            .collect();

        let mut count = 0;
        for result in results {
            result?;
            count += 1;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn insert_record(table: &str, key: &[u8], value: &[u8]) -> WalRecord {
        WalRecord::Insert {
            table: table.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
            ts: 0,
        }
    }

    #[test]
    fn test_partitioned_wal_basic() {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100).unwrap();

        let seq = wal.append(insert_record("users", b"k1", b"v1")).unwrap();
        assert_eq!(seq, 0);

        let seq2 = wal.append(insert_record("orders", b"k2", b"v2")).unwrap();
        assert_eq!(seq2, 1);

        assert_eq!(wal.partition_count(), 2);
        assert_eq!(wal.buffered_count(), 2);
    }

    #[test]
    fn test_partitioned_wal_batch() {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100).unwrap();

        let records = vec![
            insert_record("users", b"k1", b"v1"),
            insert_record("users", b"k2", b"v2"),
            insert_record("orders", b"k3", b"v3"),
        ];

        let seqs = wal.append_batch(records).unwrap();
        assert_eq!(seqs.len(), 3);
        assert_eq!(wal.partition_count(), 2);
    }

    #[test]
    fn test_partitioned_wal_flush() {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100).unwrap();

        for i in 0..10 {
            wal.append(insert_record("users", format!("k{i}").as_bytes(), b"v"))
                .unwrap();
        }

        let flushed = wal.flush_all().unwrap();
        assert_eq!(flushed, 10);
        assert_eq!(wal.buffered_count(), 0);

        // WAL 파일 생성 확인
        assert!(dir.path().join("users.wal").exists());
    }

    #[test]
    fn test_partitioned_wal_auto_flush() {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 5).unwrap();

        // 5개 추가 시 자동 플러시
        for i in 0..5 {
            wal.append(insert_record("users", format!("k{i}").as_bytes(), b"v"))
                .unwrap();
        }

        // 자동 플러시 후 버퍼는 비어 있어야 함
        assert_eq!(wal.buffered_count(), 0);
        assert!(dir.path().join("users.wal").exists());
    }

    #[test]
    fn test_parallel_checkpoint() {
        let dir = tempdir().unwrap();
        let wal = PartitionedWalWriter::new(dir.path().to_path_buf(), 100).unwrap();

        // 데이터 추가 후 플러시
        wal.append(insert_record("users", b"k1", b"v1")).unwrap();
        wal.append(insert_record("orders", b"k2", b"v2")).unwrap();
        wal.flush_all().unwrap();

        // 체크포인트
        let checkpoint_mgr = ParallelCheckpointManager::new(dir.path().to_path_buf());
        let count = checkpoint_mgr
            .checkpoint_tables(&["users".to_string(), "orders".to_string()])
            .unwrap();
        assert_eq!(count, 2);

        // 체크포인트 파일 확인
        assert!(dir.path().join("users.checkpoint").exists());
        assert!(dir.path().join("orders.checkpoint").exists());
    }
}
