use std::sync::Arc;
use crate::error::{DbxError, DbxResult};
use crate::sql::executor::operators::PhysicalOperator;
use crate::sql::planner::types::ShuffleSalting;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;
use tokio::sync::mpsc;
use rand::Rng;

/// Opaque wrapper struct that can be converted from an arrow Compute Error
/// We will keep hash logic extremely simple.
pub struct GridShuffleWriterOperator {
    input: Box<dyn PhysicalOperator>,
    hash_params: Vec<usize>,
    exchange_id: usize,
    salting: ShuffleSalting,
    /// Outgoing channels for each target node
    target_senders: Vec<mpsc::Sender<DbxResult<Option<Vec<u8>>>>>,
}

impl GridShuffleWriterOperator {
    pub fn new(
        input: Box<dyn PhysicalOperator>,
        hash_params: Vec<usize>,
        exchange_id: usize,
        salting: ShuffleSalting,
        target_senders: Vec<mpsc::Sender<DbxResult<Option<Vec<u8>>>>>,
    ) -> Self {
        Self {
            input,
            hash_params,
            exchange_id,
            salting,
            target_senders,
        }
    }

    /// Arrow IPC serialization utility
    fn serialize_batch(&self, batch: &RecordBatch) -> DbxResult<Vec<u8>> {
        crate::grid::protocol::serialize_batch_to_ipc(batch)
    }
}

impl PhysicalOperator for GridShuffleWriterOperator {
    fn schema(&self) -> &Schema {
        self.input.schema()
    }

    fn next(&mut self) -> DbxResult<Option<RecordBatch>> {
        // Fetch next batch from input
        let batch_opt = self.input.next()?;
        
        let batch = match batch_opt {
            Some(b) => b,
            None => return Ok(None),
        };

        let num_targets = self.target_senders.len();
        if num_targets == 0 || batch.num_rows() == 0 {
            return Ok(Some(batch)); // Pass through if no targets
        }

        // Basic implementation for Phase 4 Track C
        // Later we'll partition the Arrow Batch, but for now we format it
        // and handle ReplicateProbe and RandomDistributed
        
        match &self.salting {
            ShuffleSalting::ReplicateProbe { factor } => {
                // Broadcast essentially
                let bytes = self.serialize_batch(&batch)?;
                for sender in &self.target_senders {
                    let _ = sender.blocking_send(Ok(Some(bytes.clone())));
                }
            }
            ShuffleSalting::RandomDistributed { factor } => {
                // Random round robin for now instead of complex arrow hashing
                // Real DBs split the RecordBatch into N tiny batches using take()
                // For simplicity, we can send the whole batch to a random node
                let target_idx = rand::thread_rng().gen_range(0..num_targets);
                let bytes = self.serialize_batch(&batch)?;
                let _ = self.target_senders[target_idx].blocking_send(Ok(Some(bytes)));
            }
            ShuffleSalting::None => {
                // If hash_params are empty, single node. If not, naive hash.
                let target_idx = 0; // TODO: Implement Arrow array hashing logic using compute::kernels
                let bytes = self.serialize_batch(&batch)?;
                let _ = self.target_senders[target_idx].blocking_send(Ok(Some(bytes)));
            }
        }

        // We return an empty batch so local execution doesn't process these rows locally
        // Because the data was SENT to other nodes.
        Ok(Some(RecordBatch::new_empty(std::sync::Arc::new(self.input.schema().clone()))))
    }

    fn reset(&mut self) -> DbxResult<()> {
        self.input.reset()
    }
}
