//! Transaction Management — MVCC transaction methods

use crate::engine::Database;

impl Database {
    // No begin_transaction or begin here. The canonical `begin` is in api/transaction.rs
}
