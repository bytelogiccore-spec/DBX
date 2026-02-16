//! Database CRUD Operations — Create, Read, Update, Delete methods

use crate::engine::Database;
use crate::engine::types::{BackgroundJob, DurabilityLevel};
use crate::error::{DbxError, DbxResult};
use crate::storage::StorageBackend;

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
    // WAL Helper
    // ════════════════════════════════════════════

    /// Append a WAL record if durability is enabled and a WAL backend exists.
    #[inline]
    fn append_to_wal(&self, record: &crate::wal::WalRecord) -> DbxResult<()> {
        if self.durability == DurabilityLevel::None {
            return Ok(());
        }
        if let Some(wal) = &self.wal {
            wal.append(record)?;
            if self.durability == DurabilityLevel::Full {
                if let Some(tx) = &self.job_sender {
                    let _ = tx.send(BackgroundJob::WalSync);
                } else {
                    wal.sync()?;
                }
            }
        } else if let Some(encrypted_wal) = &self.encrypted_wal {
            encrypted_wal.append(record)?;
            if self.durability == DurabilityLevel::Full {
                if let Some(tx) = &self.job_sender {
                    let _ = tx.send(BackgroundJob::EncryptedWalSync);
                } else {
                    encrypted_wal.sync()?;
                }
            }
        }
        Ok(())
    }

    // ════════════════════════════════════════════
    // CRUD Operations
    // ════════════════════════════════════════════

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
        // Log to WAL first — only allocate record if WAL exists
        #[cfg(feature = "wal")]
        if self.durability != DurabilityLevel::None
            && (self.wal.is_some() || self.encrypted_wal.is_some())
        {
            self.append_to_wal(&crate::wal::WalRecord::Insert {
                table: table.to_string(),
                key: key.to_vec(),
                value: value.to_vec(),
                ts: 0,
            })?;
        }

        // 데이터 삽입
        self.delta.insert(table, key, value)?;

        // O(1) row_id 계산 + 인덱스 업데이트 — only when index exists
        #[cfg(feature = "index")]
        if self.has_index(table, "key") {
            let counter = self
                .row_counters
                .entry(table.to_string())
                .or_insert_with(|| std::sync::atomic::AtomicUsize::new(0));
            let row_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        #[cfg(feature = "wal")]
        if self.durability != DurabilityLevel::None
            && (self.wal.is_some() || self.encrypted_wal.is_some())
        {
            self.append_to_wal(&crate::wal::WalRecord::Batch {
                table: table.to_string(),
                rows: rows.clone(),
                ts: 0,
            })?;
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
    ///
    /// 성능 최적화: MVCC feature가 비활성화되면 Fast-path만 사용하여
    /// 최대 성능을 달성합니다.
    #[inline(always)]
    pub fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // Fast-path: Delta → WOS 직접 조회 (MVCC 오버헤드 없음)
        // MVCC feature가 활성화되어도 Fast-path를 우선 사용
        // 일반 insert()로 저장된 데이터는 여기서 조회됨
        if let Some(value) = self.delta.get(table, key)? {
            return Ok(Some(value));
        }
        if let Some(value) = self.wos.get(table, key)? {
            return Ok(Some(value));
        }

        // ════════════════════════════════════════════
        // MVCC Fallback: Transaction Commit 후 데이터 조회
        // ════════════════════════════════════════════
        // Transaction::commit()은 insert_versioned()와 insert()를 모두 호출하므로
        // 일반적으로 위의 Fast-path에서 데이터를 찾을 수 있습니다.
        // 
        // 하지만 다음 경우에 이 Fallback이 필요합니다:
        // 1. insert_versioned()만 호출된 경우 (일반 key 없음)
        // 2. 향후 MVCC 전용 모드 지원 시
        // 3. Snapshot isolation 구현 시
        //
        // 현재는 최신 타임스탬프로 조회하지만, 향후 snapshot_ts를 인자로 받아
        // 특정 시점의 데이터를 조회할 수 있도록 확장 가능합니다.
        let current_ts = self.tx_manager.allocate_commit_ts();
        let vk = crate::transaction::version::VersionedKey::new(key.to_vec(), current_ts);
        let encoded_key = vk.encode();
        
        // Delta에서 versioned key 조회
        if let Some(value) = self.delta.get(table, &encoded_key)? {
            return Ok(Self::decode_mvcc_value(value));
        }
        
        // WOS에서 versioned key 조회
        if let Some(value) = self.wos.get(table, &encoded_key)? {
            return Ok(Self::decode_mvcc_value(value));
        }

        Ok(None)
    }

    /// MVCC 값 디코딩 (Tombstone 필터링)
    #[inline(always)]
    fn decode_mvcc_value(v: Vec<u8>) -> Option<Vec<u8>> {
        if v.len() < MVCC_PREFIX_LEN || v[0] != 0x00 {
            return Some(v); // Legacy value
        }

        match v[1] {
            0x01 => Some(v[MVCC_PREFIX_LEN..].to_vec()), // Value
            0x02 => None,                                // Tombstone
            _ => Some(v),                                // Unknown tag
        }
    }

    /// VersionedKey 디코딩
    #[inline(always)]
    fn decode_versioned_key(k: Vec<u8>) -> Vec<u8> {
        if k.len() <= 8 {
            return k;
        }

        crate::transaction::version::VersionedKey::decode(&k)
            .map(|vk| vk.user_key)
            .unwrap_or(k)
    }

    /// 테이블의 모든 키-값 쌍을 스캔합니다.
    pub fn scan(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Fast-path: Delta가 비어있으면 WOS 직접 스캔 (merge 오버헤드 제거)
        let delta_entries = self.delta.scan(table, ..)?;
        if delta_entries.is_empty() {
            return self.wos.scan(table, ..);
        }

        // 1. Collect from Delta Store and WOS
        let wos_entries = self.wos.scan(table, ..)?;

        // 2. Direct 2-way merge (both are already sorted)
        let mut result = Vec::with_capacity(delta_entries.len() + wos_entries.len());

        let mut i = 0;
        let mut j = 0;

        while i < delta_entries.len() && j < wos_entries.len() {
            match delta_entries[i].0.cmp(&wos_entries[j].0) {
                std::cmp::Ordering::Less => {
                    // Delta key is smaller
                    if let Some(decoded_v) = Self::decode_mvcc_value(delta_entries[i].1.clone()) {
                        let user_key = Self::decode_versioned_key(delta_entries[i].0.clone());
                        result.push((user_key, decoded_v));
                    }
                    i += 1;
                }
                std::cmp::Ordering::Equal => {
                    // Same key - Delta takes priority
                    if let Some(decoded_v) = Self::decode_mvcc_value(delta_entries[i].1.clone()) {
                        let user_key = Self::decode_versioned_key(delta_entries[i].0.clone());
                        result.push((user_key, decoded_v));
                    }
                    i += 1;
                    j += 1; // Skip WOS entry
                }
                std::cmp::Ordering::Greater => {
                    // WOS key is smaller
                    if let Some(decoded_v) = Self::decode_mvcc_value(wos_entries[j].1.clone()) {
                        let user_key = Self::decode_versioned_key(wos_entries[j].0.clone());
                        result.push((user_key, decoded_v));
                    }
                    j += 1;
                }
            }
        }

        // 3. Process remaining Delta entries
        while i < delta_entries.len() {
            if let Some(decoded_v) = Self::decode_mvcc_value(delta_entries[i].1.clone()) {
                let user_key = Self::decode_versioned_key(delta_entries[i].0.clone());
                result.push((user_key, decoded_v));
            }
            i += 1;
        }

        // 4. Process remaining WOS entries
        while j < wos_entries.len() {
            if let Some(decoded_v) = Self::decode_mvcc_value(wos_entries[j].1.clone()) {
                let user_key = Self::decode_versioned_key(wos_entries[j].0.clone());
                result.push((user_key, decoded_v));
            }
            j += 1;
        }

        Ok(result)
    }

    /// 테이블의 키 범위를 스캔합니다.
    pub fn range(
        &self,
        table: &str,
        start_key: &[u8],
        end_key: &[u8],
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let range = start_key.to_vec()..end_key.to_vec();

        // Scan both Delta Store and WOS with range bounds
        let mut merged = std::collections::BTreeMap::new();
        for (k, v) in self.delta.scan(table, range.clone())? {
            merged.insert(k, v);
        }
        for (k, v) in self.wos.scan(table, range)? {
            merged.entry(k).or_insert(v);
        }

        Ok(merged.into_iter().collect())
    }

    /// 테이블의 행 개수를 반환합니다.
    pub fn table_row_count(&self, table: &str) -> DbxResult<usize> {
        self.count(table)
    }

    // ════════════════════════════════════════════
    // DELETE Operations
    // ════════════════════════════════════════════

    /// 키를 삭제합니다.
    pub fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        #[cfg(feature = "index")]
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
        #[cfg(feature = "mvcc")]
        {
            let commit_ts = self.tx_manager.allocate_commit_ts();
            self.insert_versioned(table, key, None, commit_ts)?;
        }

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

// ════════════════════════════════════════════
// DatabaseCore Trait Implementation
// ════════════════════════════════════════════

impl crate::traits::DatabaseCore for Database {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        // Reuse existing implementation
        Database::insert(self, table, key, value)
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        // Reuse existing implementation
        Database::get(self, table, key)
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<()> {
        // Reuse existing implementation
        Database::delete(self, table, key).map(|_| ())
    }

    fn scan(&self, table: &str) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        // Reuse existing implementation
        Database::scan(self, table)
    }

    fn flush(&self) -> DbxResult<()> {
        // Reuse existing implementation
        Database::flush(self)
    }

    fn insert_batch(&self, table: &str, entries: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        // Reuse existing implementation
        Database::insert_batch(self, table, entries)
    }
}

