use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A source of monotonically increasing timestamps for MVCC.
///
/// This oracle ensures that every transaction gets a unique logical timestamp
/// for determining visibility and commit ordering.
///
/// - `read_ts`: The point in time a transaction reads from (snapshot).
/// - `commit_ts`: The point in time a transaction writes its changes.
#[derive(Debug)]
pub struct TimestampOracle {
    /// The next available timestamp. Starts at 1.
    next_ts: AtomicU64,
}

impl TimestampOracle {
    /// Create a new TimestampOracle starting at the given timestamp.
    pub fn new(start_ts: u64) -> Self {
        Self {
            next_ts: AtomicU64::new(start_ts),
        }
    }

    /// Allocate and return the next timestamp.
    /// Returns the NEW value after incrementing.
    pub fn next(&self) -> u64 {
        self.next_ts.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Read the current timestamp without incrementing (e.g., for status checks).
    pub fn read(&self) -> u64 {
        self.next_ts.load(Ordering::SeqCst)
    }
}

impl Default for TimestampOracle {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Managing active transactions and cleanup.
#[derive(Debug)]
pub struct TransactionManager {
    oracle: Arc<TimestampOracle>,
    /// Active transactions: tx_id -> read_ts
    active_txs: Arc<DashMap<u64, u64>>,
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            oracle: Arc::new(TimestampOracle::default()),
            active_txs: Arc::new(DashMap::new()),
        }
    }

    /// Start a new transaction, allocating a read timestamp.
    pub fn begin_transaction(&self) -> u64 {
        let read_ts = self.oracle.next();
        let tx_id = read_ts; // Use read_ts as tx_id for simplicity
        self.active_txs.insert(tx_id, read_ts);
        read_ts
    }

    /// Allocate a commit timestamp.
    pub fn allocate_commit_ts(&self) -> u64 {
        self.oracle.next()
    }

    /// Read the current max timestamp (for non-transactional reads).
    pub fn current_ts(&self) -> u64 {
        self.oracle.read()
    }

    /// Mark a transaction as completed (committed or rolled back).
    pub fn end_transaction(&self, tx_id: u64) {
        self.active_txs.remove(&tx_id);
    }

    /// Get the minimum active read timestamp.
    ///
    /// This is the watermark below which versions can be safely garbage collected.
    /// Returns None if there are no active transactions.
    pub fn min_active_ts(&self) -> Option<u64> {
        self.active_txs.iter().map(|entry| *entry.value()).min()
    }

    /// Get the number of active transactions.
    pub fn active_count(&self) -> usize {
        self.active_txs.len()
    }
}
