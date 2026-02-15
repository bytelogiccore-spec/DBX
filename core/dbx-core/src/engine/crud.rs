//! Database CRUD Operations — Create, Read, Update, Delete methods

use crate::engine::Database;
use crate::engine::types::{BackgroundJob, DurabilityLevel};
use crate::error::{DbxError, DbxResult};
use crate::storage::StorageBackend;
use std::collections::HashMap;

// ════════════════════════════════════════════
// ⚠️ MVCC Value Encoding Constants
// ════════════════════════════════════════════
// MVCC 버전 관리를 위한 매직 헤더.
// 반드시 2바이트 [0x00, tag]를 사용하여 일반 사용자 데이터와 충돌을 방지한다.
// 일반 UTF-8 텍스트나 바이너리 데이터는 0x00으로 시작하지 않으므로 안전하다.
// 이 상수를 변경하면 crud.rs, snapshot.rs 양쪽 모두 동기화해야 한다.

/// MVCC 값이 존재함을 나타내는 2바이트 매직 헤더: [0x00, 0x01]
pub(crate) const MVCC_VALUE_PREFIX: [u8; 2] = [0x00, 0x01];
/// MVCC 삭제(tombstone)를 나타내는 2바이트 매직 헤더: [0x00, 0x02]
pub(crate) const MVCC_TOMBSTONE_PREFIX: [u8; 2] = [0x00, 0x02];
/// MVCC 매직 헤더 길이
pub(crate) const MVCC_PREFIX_LEN: usize = 2;

impl Database {
    // ════════════════════════════════════════════
    // CREATE Operations
    // ════════════════════════════════════════════

    /// 키-값 쌍을 삽입합니다.
    ///
    /// 데이터는 먼저 Delta Store (Tier 1)에 쓰여집니다.
    /// Flush 임계값을 초과하면 자동으로 WOS로 이동합니다.
    ///
    /// # 인자
    ///
    /// * `table` - 테이블 이름
    /// * `key` - 키 (바이트 배열)
    /// * `value` - 값 (바이트 배열)
    pub fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        if self.durability != DurabilityLevel::None {
            // Log to WAL first (Write-Ahead Logging)
            let wal_record = crate::wal::WalRecord::Insert {
                table: table.to_string(),
                key: key.to_vec(),
                value: value.to_vec(),
                ts: 0,
            };
            if let Some(wal) = &self.wal {
                wal.append(&wal_record)?;
                if self.durability == DurabilityLevel::Full {
                    if let Some(tx) = &self.job_sender {
                        let _ = tx.send(BackgroundJob::WalSync);
                    } else {
                        wal.sync()?;
                    }
                }
            } else if let Some(encrypted_wal) = &self.encrypted_wal {
                encrypted_wal.append(&wal_record)?;
                if self.durability == DurabilityLevel::Full {
                    if let Some(tx) = &self.job_sender {
                        let _ = tx.send(BackgroundJob::EncryptedWalSync);
                    } else {
                        encrypted_wal.sync()?;
                    }
                }
            }
        }

        // O(1) row_id 계산 (증분 카운터 사용)
        let counter = self
            .row_counters
            .entry(table.to_string())
            .or_insert_with(|| std::sync::atomic::AtomicUsize::new(0));
        let row_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // 데이터 삽입
        self.delta.insert(table, key, value)?;

        // 비동기 인덱스 업데이트 예약
        if self.has_index(table, "key") {
            if let Some(tx) = &self.job_sender {
                let _ = tx.send(BackgroundJob::IndexUpdate {
                    table: table.to_string(),
                    column: "key".to_string(),
                    key: key.to_vec(),
                    row_id,
                });
            } else {
                self.index.update_on_insert(table, "key", key, row_id)?;
            }
        }

        // Auto-flush if threshold exceeded
        if self.delta.should_flush() {
            self.flush()?;
        }

        Ok(())
    }

    /// 여러 키-값 쌍을 일괄 삽입합니다 (최적화됨).
    pub fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        if self.durability != DurabilityLevel::None {
            // Log to WAL first
            let wal_record = crate::wal::WalRecord::Batch {
                table: table.to_string(),
                rows: rows.clone(),
                ts: 0,
            };

            if let Some(wal) = &self.wal {
                wal.append(&wal_record)?;
                if self.durability == DurabilityLevel::Full {
                    if let Some(tx) = &self.job_sender {
                        let _ = tx.send(BackgroundJob::WalSync);
                    } else {
                        wal.sync()?;
                    }
                }
            } else if let Some(encrypted_wal) = &self.encrypted_wal {
                encrypted_wal.append(&wal_record)?;
                if self.durability == DurabilityLevel::Full {
                    if let Some(tx) = &self.job_sender {
                        let _ = tx.send(BackgroundJob::EncryptedWalSync);
                    } else {
                        encrypted_wal.sync()?;
                    }
                }
            }
        }

        self.delta.insert_batch(table, rows)?;

        // Auto-flush if threshold exceeded
        if self.delta.should_flush() {
            self.flush()?;
        }

        Ok(())
    }

    /// Insert a versioned key-value pair for MVCC.
    pub fn insert_versioned(
        &self,
        table: &str,
        key: &[u8],
        value: Option<&[u8]>,
        commit_ts: u64,
    ) -> DbxResult<()> {
        let vk = crate::transaction::version::VersionedKey::new(key.to_vec(), commit_ts);
        let encoded_key = vk.encode();

        // Encode value with prefix
        // ⚠️ MVCC 매직 헤더 인코딩 — MVCC_VALUE_PREFIX / MVCC_TOMBSTONE_PREFIX 사용
        let encoded_value = match value {
            Some(v) => {
                let mut bytes = Vec::with_capacity(v.len() + MVCC_PREFIX_LEN);
                bytes.extend_from_slice(&MVCC_VALUE_PREFIX);
                bytes.extend_from_slice(v);
                bytes
            }
            None => MVCC_TOMBSTONE_PREFIX.to_vec(),
        };

        // Write to Delta Store
        self.delta.insert(table, &encoded_key, &encoded_value)?;

        Ok(())
    }

    // ════════════════════════════════════════════
    // READ Operations
    // ════════════════════════════════════════════

    /// Read a specific version of a key (Snapshot Read).
    pub fn get_snapshot(
        &self,
        table: &str,
        key: &[u8],
        read_ts: u64,
    ) -> DbxResult<Option<Option<Vec<u8>>>> {
        let start_vk = crate::transaction::version::VersionedKey::new(key.to_vec(), read_ts);
        let start_bytes = start_vk.encode();

        // Helper: returns Some(Some(v)), Some(None) (tombstone), or None (mismatch)
        let check_entry = |entry_key: &[u8], entry_val: &[u8]| -> Option<Option<Vec<u8>>> {
            let decoded = crate::transaction::version::VersionedKey::decode(entry_key).ok()?;
            if decoded.user_key != key {
                return None;
            }
            if decoded.commit_ts > read_ts {
                return None;
            }
            if entry_val.is_empty() {
                return Some(Some(entry_val.to_vec())); // Legacy empty value
            }
            // ⚠️ MVCC 매직 헤더 디코딩 — 2바이트 [0x00, tag] 확인
            if entry_val.len() >= MVCC_PREFIX_LEN && entry_val[0] == 0x00 {
                match entry_val[1] {
                    0x01 => return Some(Some(entry_val[MVCC_PREFIX_LEN..].to_vec())),
                    0x02 => return Some(None), // Tombstone
                    _ => {}
                }
            }
            // Legacy non-prefixed value
            Some(Some(entry_val.to_vec()))
        };

        // 1. Check Delta Store
        if let Some((k, v)) = self.delta.scan_one(table, start_bytes.clone()..)?
            && let Some(result) = check_entry(&k, &v)
        {
            return Ok(Some(result));
        }

        // 2. Check WOS
        if let Some((k, v)) = self.wos.scan_one(table, start_bytes..)?
            && let Some(result) = check_entry(&k, &v)
        {
            return Ok(Some(result));
        }

        Ok(None)
    }

    /// Helper method for Snapshot: scan all versioned entries from Delta Store.
    pub(crate) fn scan_delta_versioned(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        StorageBackend::scan(&self.delta, table, ..)
    }

    /// Helper method for Snapshot: scan all versioned entries from WOS.
    pub(crate) fn scan_wos_versioned(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        self.wos.scan(table, ..)
    }

    /// Get the current timestamp from the transaction manager.
    pub fn current_timestamp(&self) -> u64 {
        self.tx_manager.current_ts()
    }

    /// Allocate a new commit timestamp for a transaction.
    /// This increments the timestamp oracle and returns a unique timestamp.
    pub fn allocate_commit_ts(&self) -> u64 {
        self.tx_manager.allocate_commit_ts()
    }

    /// 키로 값을 조회합니다.
    pub fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // 1. Prioritize MVCC Snapshot Read — versioned keys
        let current_ts = self.tx_manager.current_ts();
        if let Some(result) = self.get_snapshot(table, key, current_ts)? {
            return Ok(result); // Some(val) or None (tombstone)
        }

        // 2. Fallback to Legacy Read (Delta → WOS) — non-versioned keys
        if let Some(value) = self.delta.get(table, key)? {
            return Ok(Some(value));
        }
        if let Some(value) = self.wos.get(table, key)? {
            return Ok(Some(value));
        }

        Ok(None)
    }

    /// 테이블의 모든 키-값 쌍을 스캔합니다.
    pub fn scan(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // 1. Collect from Delta Store
        let delta_entries = self.delta.scan(table, ..)?;

        // 2. Collect from WOS
        let wos_entries = self.wos.scan(table, ..)?;

        // 3. Merge (Delta overrides WOS for same keys)
        let mut merged = HashMap::new();

        // WOS first (lower priority)
        for (key, value) in wos_entries {
            merged.insert(key, value);
        }

        // Delta second (higher priority - overrides WOS)
        for (key, value) in delta_entries {
            merged.insert(key, value);
        }

        // 4. Convert to sorted Vec and handle prefixes
        let mut result = Vec::new();
        for (k, v) in merged {
            // Decode value
            // ⚠️ MVCC 매직 헤더 디코딩 — [0x00, tag] 확인
            let decoded_v = if v.is_empty() {
                v
            } else if v.len() >= MVCC_PREFIX_LEN && v[0] == 0x00 {
                match v[1] {
                    0x01 => v[MVCC_PREFIX_LEN..].to_vec(), // Value
                    0x02 => continue,                      // Tombstone
                    _ => v,                                // Unknown tag → legacy
                }
            } else {
                v
            };

            // Decode key if versioned
            let user_key = if k.len() > 8 {
                if let Ok(vk) = crate::transaction::version::VersionedKey::decode(&k) {
                    vk.user_key
                } else {
                    k
                }
            } else {
                k
            };

            result.push((user_key, decoded_v));
        }
        result.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(result)
    }

    /// 테이블의 키 범위를 스캔합니다.
    pub fn range(
        &self,
        table: &str,
        start_key: &[u8],
        end_key: &[u8],
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Use full scan and filter for now (simpler than range bound conversion)
        let all = self.scan(table)?;
        Ok(all
            .into_iter()
            .filter(|(k, _)| k.as_slice() >= start_key && k.as_slice() < end_key)
            .collect())
    }

    /// 테이블의 행 개수를 반환합니다.
    pub fn table_row_count(&self, table: &str) -> DbxResult<usize> {
        let all = self.scan(table)?;
        Ok(all.len())
    }

    // ════════════════════════════════════════════
    // DELETE Operations
    // ════════════════════════════════════════════

    /// 키를 삭제합니다.
    pub fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        if self.has_index(table, "key") {
            let row_ids = self.index.lookup(table, "key", key)?;
            for row_id in row_ids {
                self.index.update_on_delete(table, "key", key, row_id)?;
            }
        }

        // 1. Delete from legacy
        let delta_deleted = self.delta.delete(table, key)?;
        let wos_deleted = self.wos.delete(table, key)?;

        // 2. Add versioned tombstone if it was a versioned key
        let commit_ts = self.tx_manager.allocate_commit_ts();
        self.insert_versioned(table, key, None, commit_ts)?;

        Ok(delta_deleted || wos_deleted)
    }

    // ════════════════════════════════════════════
    // Helper Methods
    // ════════════════════════════════════════════

    /// Synchronize the Columnar Cache with the latest data from Delta Store.
    pub fn sync_columnar_cache(&self, table: &str) -> DbxResult<usize> {
        self.columnar_cache.sync_from_delta(&self.delta, table)
    }

    /// Sync data from multiple tiers (Delta and ROS) to GPU for merge operations.
    pub fn sync_gpu_cache_multi_tier(&self, table: &str) -> DbxResult<()> {
        let gpu = self
            .gpu_manager
            .as_ref()
            .ok_or_else(|| DbxError::NotImplemented("GPU manager not available".to_string()))?;

        // 1. Sync Delta data (Tier 1)
        let delta_batches = self.columnar_cache.get_batches(table, None)?;
        if let Some(batches) = delta_batches {
            for batch in batches {
                gpu.upload_batch_pinned(&format!("{}_delta", table), &batch)?;
            }
        }

        // 2. Sync ROS data (Tier 5) - simplified: assuming ROS is already in SQL tables for now
        let tables = self.tables.read().unwrap();
        if let Some(batches) = tables.get(table) {
            for batch in batches {
                gpu.upload_batch_pinned(&format!("{}_ros", table), batch)?;
            }
        }

        Ok(())
    }

    /// Legacy method to sync data from Columnar Cache to GPU.
    pub fn sync_gpu_cache(&self, table: &str) -> DbxResult<()> {
        self.sync_gpu_cache_multi_tier(table)
    }

    /// Execute an operation on GPU with automatic fallback to CPU on any error.
    pub fn gpu_exec_with_fallback<T, F, C>(&self, gpu_op: F, cpu_op: C) -> DbxResult<T>
    where
        F: FnOnce(&crate::storage::gpu::GpuManager) -> DbxResult<T>,
        C: FnOnce() -> DbxResult<T>,
    {
        if let Some(gpu) = &self.gpu_manager {
            match gpu_op(gpu) {
                Ok(val) => Ok(val),
                Err(e) => {
                    tracing::warn!("GPU execution failed, falling back to CPU: {:?}", e);
                    cpu_op()
                }
            }
        } else {
            cpu_op()
        }
    }
}
