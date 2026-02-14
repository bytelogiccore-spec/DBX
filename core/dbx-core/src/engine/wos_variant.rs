//! WOS (Write-Optimized Store) Variant — supports both plain and encrypted implementations

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use crate::storage::encrypted_wos::EncryptedWosBackend;
use crate::storage::memory_wos::InMemoryWosBackend;
use crate::storage::wos::WosBackend;
use std::sync::Arc;

/// WOS variant — supports both plain and encrypted implementations.
///
/// Note: `StorageBackend::scan` has a generic parameter `R: RangeBounds`
/// which prevents dyn compatibility.
pub enum WosVariant {
    Plain(Arc<WosBackend>),
    Encrypted(Arc<EncryptedWosBackend>),
    InMemory(Arc<InMemoryWosBackend>),
}

impl WosVariant {
    #[allow(dead_code)]
    pub fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => wos.as_ref().insert(table, key, value),
            Self::Encrypted(wos) => wos.as_ref().insert(table, key, value),
            Self::InMemory(wos) => wos.as_ref().insert(table, key, value),
        }
    }

    pub fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => wos.as_ref().insert_batch(table, rows),
            Self::Encrypted(wos) => wos.as_ref().insert_batch(table, rows),
            Self::InMemory(wos) => wos.as_ref().insert_batch(table, rows),
        }
    }

    pub fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        match self {
            Self::Plain(wos) => wos.as_ref().get(table, key),
            Self::Encrypted(wos) => wos.as_ref().get(table, key),
            Self::InMemory(wos) => wos.as_ref().get(table, key),
        }
    }

    pub fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        match self {
            Self::Plain(wos) => wos.as_ref().delete(table, key),
            Self::Encrypted(wos) => wos.as_ref().delete(table, key),
            Self::InMemory(wos) => wos.as_ref().delete(table, key),
        }
    }

    #[allow(dead_code)]
    pub fn scan<R: std::ops::RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        match self {
            Self::Plain(wos) => wos.as_ref().scan(table, range),
            Self::Encrypted(wos) => wos.as_ref().scan(table, range),
            Self::InMemory(wos) => wos.as_ref().scan(table, range),
        }
    }

    pub fn scan_one<R: std::ops::RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        match self {
            Self::Plain(wos) => wos.as_ref().scan_one(table, range),
            Self::Encrypted(wos) => wos.as_ref().scan_one(table, range),
            Self::InMemory(wos) => wos.as_ref().scan_one(table, range),
        }
    }

    pub fn flush(&self) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => wos.as_ref().flush(),
            Self::Encrypted(wos) => wos.as_ref().flush(),
            Self::InMemory(wos) => wos.as_ref().flush(),
        }
    }

    pub fn count(&self, table: &str) -> DbxResult<usize> {
        match self {
            Self::Plain(wos) => wos.as_ref().count(table),
            Self::Encrypted(wos) => wos.as_ref().count(table),
            Self::InMemory(wos) => wos.as_ref().count(table),
        }
    }

    pub fn table_names(&self) -> DbxResult<Vec<String>> {
        match self {
            Self::Plain(wos) => wos.as_ref().table_names(),
            Self::Encrypted(wos) => wos.as_ref().table_names(),
            Self::InMemory(wos) => wos.as_ref().table_names(),
        }
    }

    // ════════════════════════════════════════════
    // Metadata Persistence Helpers
    // ════════════════════════════════════════════

    /// Save schema metadata (only for Plain WOS, no-op for encrypted/in-memory)
    pub fn save_schema_metadata(&self, table: &str, schema: &arrow::datatypes::Schema) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => {
                crate::engine::metadata::save_schema(wos.as_ref(), table, schema)
            }
            Self::Encrypted(_) | Self::InMemory(_) => {
                // Encrypted and in-memory WOS don't persist metadata
                Ok(())
            }
        }
    }

    /// Delete schema metadata (only for Plain WOS)
    pub fn delete_schema_metadata(&self, table: &str) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => {
                crate::engine::metadata::delete_schema(wos.as_ref(), table)
            }
            Self::Encrypted(_) | Self::InMemory(_) => Ok(()),
        }
    }

    /// Save index metadata (only for Plain WOS)
    pub fn save_index_metadata(&self, index_name: &str, table: &str, column: &str) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => {
                crate::engine::metadata::save_index(wos.as_ref(), index_name, table, column)
            }
            Self::Encrypted(_) | Self::InMemory(_) => Ok(()),
        }
    }

    /// Delete index metadata (only for Plain WOS)
    pub fn delete_index_metadata(&self, index_name: &str) -> DbxResult<()> {
        match self {
            Self::Plain(wos) => {
                crate::engine::metadata::delete_index(wos.as_ref(), index_name)
            }
            Self::Encrypted(_) | Self::InMemory(_) => Ok(()),
        }
    }
}
