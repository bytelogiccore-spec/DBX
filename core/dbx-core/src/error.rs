//! Error types for the DBX database engine.
//!
//! All public APIs return `DbxResult<T>` — no panics in library code.

use thiserror::Error;

/// Unified error type for all DBX operations.
#[derive(Debug, Error)]
pub enum DbxError {
    /// Storage layer error (I/O, corruption, etc.)
    #[error("storage error: {0}")]
    Storage(String),

    /// Schema definition or validation error
    #[error("schema error: {0}")]
    Schema(String),

    /// Apache Arrow error (RecordBatch operations)
    #[error("arrow error: {source}")]
    Arrow {
        #[from]
        source: arrow::error::ArrowError,
    },

    /// Apache Parquet error (file I/O)
    #[error("parquet error: {source}")]
    Parquet {
        #[from]
        source: parquet::errors::ParquetError,
    },

    /// sled embedded database error
    #[error("sled error: {source}")]
    Sled {
        #[from]
        source: sled::Error,
    },

    /// Standard I/O error
    #[error("io error: {source}")]
    Io {
        #[from]
        source: std::io::Error,
    },

    /// Requested table does not exist
    #[error("table '{0}' not found")]
    TableNotFound(String),

    /// Requested key does not exist
    #[error("key not found")]
    KeyNotFound,

    /// Type mismatch between expected and actual values
    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// Constraint violation (PK, FK, CHECK, etc.)
    #[error("constraint violation: {0}")]
    ConstraintViolation(String),

    /// Serialization/deserialization error
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Feature not yet implemented
    #[error("not implemented: {0}")]
    NotImplemented(String),

    /// SQL parsing error
    #[error("SQL parse error: {message}\nSQL: {sql}")]
    SqlParse { message: String, sql: String },

    /// SQL execution error
    #[error("SQL execution error: {message}\nContext: {context}")]
    SqlExecution { message: String, context: String },

    /// Unsupported SQL feature
    #[error("SQL feature not supported: {feature}\nHint: {hint}")]
    SqlNotSupported { feature: String, hint: String },

    /// Transaction conflict
    #[error("transaction conflict: {message}")]
    TransactionConflict { message: String },

    /// Transaction aborted
    #[error("transaction aborted: {reason}")]
    TransactionAborted { reason: String },

    /// Invalid operation
    #[error("invalid operation: {message}\nContext: {context}")]
    InvalidOperation { message: String, context: String },

    /// Index already exists
    #[error("index already exists on table '{table}', column '{column}'")]
    IndexAlreadyExists { table: String, column: String },

    /// Index not found
    #[error("index not found on table '{table}', column '{column}'")]
    IndexNotFound { table: String, column: String },

    /// WAL error
    #[error("WAL error: {0}")]
    Wal(String),

    /// Checkpoint failed
    #[error("checkpoint failed: {0}")]
    CheckpointFailed(String),

    /// Recovery failed
    #[error("recovery failed: {0}")]
    RecoveryFailed(String),

    /// Encryption/decryption error
    #[error("encryption error: {0}")]
    Encryption(String),

    /// GPU operation error
    #[error("GPU error: {0}")]
    Gpu(String),

    /// Callable not found
    #[error("callable '{0}' not found")]
    CallableNotFound(String),

    /// Duplicate callable registration
    #[error("callable '{0}' already registered")]
    DuplicateCallable(String),

    /// Invalid arguments
    #[error("invalid arguments: {0}")]
    InvalidArguments(String),

    /// Lock poisoned
    #[error("lock poisoned")]
    LockPoisoned,

    /// Performance regression detected
    #[error("performance regression detected for '{name}': baseline={baseline:.2}ms, current={current:.2}ms, ratio={ratio:.2}x")]
    PerformanceRegression {
        name: String,
        baseline: f64,
        current: f64,
        ratio: f64,
    },
}

/// Result type alias for all DBX operations.
pub type DbxResult<T> = Result<T, DbxError>;

// From 구현들
impl From<serde_json::Error> for DbxError {
    fn from(err: serde_json::Error) -> Self {
        DbxError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_storage() {
        let err = DbxError::Storage("disk full".to_string());
        assert_eq!(err.to_string(), "storage error: disk full");
    }

    #[test]
    fn error_display_table_not_found() {
        let err = DbxError::TableNotFound("users".to_string());
        assert_eq!(err.to_string(), "table 'users' not found");
    }

    #[test]
    fn error_display_type_mismatch() {
        let err = DbxError::TypeMismatch {
            expected: "Int32".to_string(),
            actual: "Utf8".to_string(),
        };
        assert_eq!(err.to_string(), "type mismatch: expected Int32, got Utf8");
    }

    #[test]
    fn dbx_result_ok() {
        let result: DbxResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn dbx_result_err() {
        let result: DbxResult<i32> = Err(DbxError::KeyNotFound);
        assert!(result.is_err());
    }

    #[test]
    fn error_display_sql_parse() {
        let err = DbxError::SqlParse {
            message: "unexpected token".to_string(),
            sql: "SELECT * FORM users".to_string(),
        };
        assert!(err.to_string().contains("SQL parse error"));
        assert!(err.to_string().contains("FORM users"));
    }

    #[test]
    fn error_display_sql_not_supported() {
        let err = DbxError::SqlNotSupported {
            feature: "WINDOW functions".to_string(),
            hint: "Use subqueries instead".to_string(),
        };
        assert!(err.to_string().contains("not supported"));
        assert!(err.to_string().contains("WINDOW functions"));
        assert!(err.to_string().contains("subqueries"));
    }

    #[test]
    fn error_display_transaction_conflict() {
        let err = DbxError::TransactionConflict {
            message: "write-write conflict on key 'user:123'".to_string(),
        };
        assert!(err.to_string().contains("transaction conflict"));
        assert!(err.to_string().contains("user:123"));
    }

    #[test]
    fn error_display_invalid_operation() {
        let err = DbxError::InvalidOperation {
            message: "cannot query after commit".to_string(),
            context: "Transaction is in Committed state".to_string(),
        };
        assert!(err.to_string().contains("invalid operation"));
        assert!(err.to_string().contains("after commit"));
    }
}
