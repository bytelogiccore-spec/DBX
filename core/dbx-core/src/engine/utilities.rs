//! Database Utility Methods — helper functions for database operations

use crate::engine::{Database, WosVariant};
use crate::error::{DbxError, DbxResult};
use crate::storage::StorageBackend;
use crate::storage::encrypted_wos::EncryptedWosBackend;
use crate::storage::encryption::EncryptionConfig;
use std::sync::Arc;

impl Database {
    /// 데이터베이스가 암호화되어 있는지 확인합니다.
    pub fn is_encrypted(&self) -> bool {
        self.encryption.read().unwrap().is_some()
    }

    /// 암호화 키를 교체합니다 (키 로테이션).
    ///
    /// 모든 저장 계층 (WOS, WAL)의 데이터를 현재 키로 복호화한 뒤
    /// 새 키로 재암호화합니다. Delta Store의 데이터는 먼저 WOS로
    /// flush된 후 재암호화됩니다.
    ///
    /// # 전제 조건
    ///
    /// - 데이터베이스가 암호화되어 있어야 합니다 (`is_encrypted() == true`).
    /// - 키 교체 중 다른 쓰기가 발생하지 않아야 합니다.
    ///
    /// # 반환값
    ///
    /// 재암호화된 레코드 수 (WOS + WAL).
    ///
    /// # 예제
    ///
    /// ```rust,no_run
    /// use dbx_core::Database;
    /// use dbx_core::storage::encryption::EncryptionConfig;
    /// use std::path::Path;
    ///
    /// let enc = EncryptionConfig::from_password("old-password");
    /// let db = Database::open_encrypted(Path::new("./data"), enc).unwrap();
    ///
    /// let new_enc = EncryptionConfig::from_password("new-password");
    /// let count = db.rotate_key(new_enc).unwrap();
    /// println!("Re-encrypted {} records", count);
    /// ```
    pub fn rotate_key(&self, new_encryption: EncryptionConfig) -> DbxResult<usize> {
        if !self.is_encrypted() {
            return Err(DbxError::Encryption(
                "cannot rotate key on unencrypted database".into(),
            ));
        }

        // Step 1: Flush Delta → WOS (ensure all data is in encrypted WOS)
        self.flush()?;

        let mut total = 0;

        // Step 2: Re-key WOS
        // We need mutable access to the EncryptedWosBackend.
        // Since WosVariant uses Arc, we use Arc::get_mut via try_unwrap workaround.
        // For now, use the rekey method through interior mutability pattern.
        match &self.wos {
            WosVariant::Encrypted(enc_wos) => {
                // Safety: We hold exclusive logical access during key rotation.
                // Use unsafe to get mutable reference — caller guarantees no concurrent writes.
                let wos_ptr = Arc::as_ptr(enc_wos) as *mut EncryptedWosBackend;
                // SAFETY: rotate_key documentation states no concurrent writes allowed
                let wos_mut = unsafe { &mut *wos_ptr };
                total += wos_mut.rekey(new_encryption.clone())?;
            }
            WosVariant::Plain(_) | WosVariant::InMemory(_) => {
                return Err(DbxError::Encryption(
                    "WOS is not encrypted — cannot rotate key".into(),
                ));
            }
        }

        // Step 3: Re-key encrypted WAL (if present)
        if let Some(enc_wal) = &self.encrypted_wal {
            let wal_ptr = Arc::as_ptr(enc_wal) as *mut crate::wal::encrypted_wal::EncryptedWal;
            // SAFETY: rotate_key documentation states no concurrent writes allowed
            let wal_mut = unsafe { &mut *wal_ptr };
            total += wal_mut.rekey(new_encryption.clone())?;
        }

        // Step 4: Update local encryption configuration
        let mut enc_lock = self.encryption.write().unwrap();
        *enc_lock = Some(new_encryption);

        Ok(total)
    }

    /// GPU Manager에 대한 참조를 반환합니다 (있는 경우).
    pub fn gpu_manager(&self) -> Option<&crate::storage::gpu::GpuManager> {
        self.gpu_manager.as_ref().map(|v| &**v)
    }

    /// Delta Store의 모든 데이터를 WOS로 flush합니다.
    pub fn flush(&self) -> DbxResult<()> {
        match &self.delta {
            crate::engine::DeltaVariant::RowBased(_) => {
                let drained = self.delta.drain_all();
                for (table, entries) in drained {
                    let rows: Vec<_> = entries.into_iter().collect();
                    self.wos.insert_batch(&table, rows)?;
                }
                self.wos.flush()
            }
            crate::engine::DeltaVariant::Columnar(_) => {
                // Get all table names
                let table_names = self.delta.table_names()?;
                for table in table_names {
                    crate::engine::compaction::Compactor::bypass_flush(self, &table)?;
                }
                Ok(())
            }
        }
    }

    /// Get the total entry count (Delta + WOS) for a table.
    pub fn count(&self, table: &str) -> DbxResult<usize> {
        let delta_count = self.delta.count(table)?;
        let wos_count = self.wos.count(table)?;
        Ok(delta_count + wos_count)
    }

    /// Get all table names across all tiers.
    pub fn table_names(&self) -> DbxResult<Vec<String>> {
        let mut names: Vec<String> = self.delta.table_names()?;
        for name in self.wos.table_names()? {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    /// Get the Delta Store entry count (diagnostic).
    pub fn delta_entry_count(&self) -> usize {
        self.delta.entry_count()
    }

    // ════════════════════════════════════════════
    // MVCC Garbage Collection
    // ════════════════════════════════════════════

    /// Run garbage collection to clean up old MVCC versions.
    ///
    /// This removes versions that are no longer visible to any active transaction.
    /// Returns the number of versions deleted.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dbx_core::Database;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// // Run GC
    /// let deleted = db.gc()?;
    /// println!("Deleted {} old versions", deleted);
    /// # Ok(())
    /// # }
    /// ```
    pub fn gc(&self) -> DbxResult<usize> {
        use crate::transaction::gc::GarbageCollector;

        let gc = GarbageCollector::new();

        // Use min_active_ts as the watermark, or current_ts if no active transactions
        let watermark = self
            .tx_manager
            .min_active_ts()
            .unwrap_or_else(|| self.tx_manager.current_ts());

        gc.collect(self, watermark)
    }

    /// Estimate the number of versions that would be deleted by GC.
    pub fn gc_estimate(&self) -> DbxResult<usize> {
        use crate::transaction::gc::GarbageCollector;

        let gc = GarbageCollector::new();
        let watermark = self
            .tx_manager
            .min_active_ts()
            .unwrap_or_else(|| self.tx_manager.current_ts());

        gc.estimate_garbage(self, watermark)
    }

    /// Get the number of active transactions.
    pub fn active_transaction_count(&self) -> usize {
        self.tx_manager.active_count()
    }
}
