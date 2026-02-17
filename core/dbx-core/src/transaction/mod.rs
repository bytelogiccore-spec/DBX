pub mod api;
pub mod mvcc;

// Re-export MVCC modules for backward compatibility
pub use mvcc::gc;
pub use mvcc::manager;
pub use mvcc::snapshot;
pub use mvcc::version;
pub use mvcc::version_manager;
pub use mvcc::versionable;

// Re-export MVCC types
pub use mvcc::{TimestampOracle, VersionManager, Versionable};

// Re-export API types
pub use api::{Active, Committed, RolledBack, Transaction, TxState};
