//! Delta Store Variant — supports both row-based and columnar implementations

use crate::error::DbxResult;
use crate::storage::StorageBackend;
use crate::storage::columnar_delta::ColumnarDelta;
use crate::storage::delta_store::DeltaStore;
use std::sync::Arc;

/// Delta Store variant — supports both row-based and columnar implementations.
pub enum DeltaVariant {
    RowBased(Arc<DeltaStore>),
    Columnar(Arc<ColumnarDelta>),
}

impl DeltaVariant {
    pub fn should_flush(&self) -> bool {
        match self {
            Self::RowBased(delta) => delta.should_flush(),
            Self::Columnar(delta) => delta.should_flush(),
        }
    }

    pub fn entry_count(&self) -> usize {
        match self {
            Self::RowBased(delta) => delta.entry_count(),
            Self::Columnar(delta) => delta.row_count(),
        }
    }

    /// Drain all data from the store.
    /// Returns table→entries mapping for flushing to WOS.
    #[allow(clippy::type_complexity)]
    pub fn drain_all(&self) -> Vec<(String, Vec<(Vec<u8>, Vec<u8>)>)> {
        match self {
            Self::RowBased(delta) => delta.drain_all(),
            Self::Columnar(delta) => {
                use crate::storage::kv_adapter::{batch_to_kv, merge_batches};

                // Get all table names
                let table_names = delta.table_names();
                let mut result = Vec::new();

                for table in table_names {
                    // Drain all batches from this table
                    let batches = delta.drain_table(&table);
                    if !batches.is_empty() {
                        // Merge all batches
                        let batch_refs: Vec<Arc<arrow::record_batch::RecordBatch>> =
                            batches.iter().map(|vb| Arc::clone(&vb.data)).collect();

                        if let Ok(merged) = merge_batches(batch_refs) {
                            // Convert to key-value pairs
                            if let Ok(rows) = batch_to_kv(&merged) {
                                result.push((table, rows));
                            }
                        }
                    }
                }

                result
            }
        }
    }
}

impl StorageBackend for DeltaVariant {
    fn insert(&self, table: &str, key: &[u8], value: &[u8]) -> DbxResult<()> {
        match self {
            Self::RowBased(delta) => delta.insert(table, key, value),
            Self::Columnar(delta) => delta.insert(table, key, value),
        }
    }

    fn insert_batch(&self, table: &str, rows: Vec<(Vec<u8>, Vec<u8>)>) -> DbxResult<()> {
        match self {
            Self::RowBased(delta) => delta.insert_batch(table, rows),
            Self::Columnar(delta) => delta.insert_batch(table, rows),
        }
    }

    fn get(&self, table: &str, key: &[u8]) -> DbxResult<Option<Vec<u8>>> {
        match self {
            Self::RowBased(delta) => delta.get(table, key),
            Self::Columnar(delta) => delta.get(table, key),
        }
    }

    fn delete(&self, table: &str, key: &[u8]) -> DbxResult<bool> {
        match self {
            Self::RowBased(delta) => delta.delete(table, key),
            Self::Columnar(delta) => delta.delete(table, key),
        }
    }

    fn scan<R: std::ops::RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Vec<(Vec<u8>, Vec<u8>)>> {
        match self {
            Self::RowBased(delta) => delta.scan(table, range),
            Self::Columnar(delta) => delta.scan(table, range),
        }
    }

    fn scan_one<R: std::ops::RangeBounds<Vec<u8>> + Clone>(
        &self,
        table: &str,
        range: R,
    ) -> DbxResult<Option<(Vec<u8>, Vec<u8>)>> {
        match self {
            Self::RowBased(delta) => delta.scan_one(table, range),
            Self::Columnar(delta) => delta.scan_one(table, range),
        }
    }

    fn flush(&self) -> DbxResult<()> {
        match self {
            Self::RowBased(delta) => delta.flush(),
            Self::Columnar(delta) => delta.flush(),
        }
    }

    fn count(&self, table: &str) -> DbxResult<usize> {
        match self {
            Self::RowBased(delta) => delta.count(table),
            Self::Columnar(delta) => delta.count(table),
        }
    }

    fn table_names(&self) -> DbxResult<Vec<String>> {
        match self {
            Self::RowBased(delta) => delta.table_names(),
            Self::Columnar(delta) => Ok(delta.table_names()),
        }
    }
}
