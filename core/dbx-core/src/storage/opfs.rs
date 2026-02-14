use crate::error::DbxResult;

/// StorageBackend extension trait for OPFS (Origin Private File System) support
pub trait OpfsBackend {
    /// Reads data from OPFS
    fn opfs_read(&self, key: &[u8]) -> DbxResult<Option<Vec<u8>>>;

    /// Writes data to OPFS
    fn opfs_write(&self, key: &[u8], value: &[u8]) -> DbxResult<()>;

    /// Deletes data from OPFS
    fn opfs_delete(&self, key: &[u8]) -> DbxResult<()>;

    /// Syncs OPFS (fsync)
    fn opfs_sync(&self) -> DbxResult<()>;
}

// TODO: Actual implementation using FileSystemSyncAccessHandle in WASM environment
// Currently only trait definition is provided
