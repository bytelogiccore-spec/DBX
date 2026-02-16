//! API 모듈 — Fluent 스타일 API
//!
//! FromRow, IntoArrowType, FromColumn 트레이트 제공

pub mod traits;
pub mod transaction;

// Re-export from sql::builder for backward compatibility
pub use crate::sql::builder::{Execute, FromScalar, IntoParam, Query, QueryOne, QueryOptional, QueryScalar};
pub use traits::{FromColumn, FromRow, IntoArrowType};
pub use transaction::{Active, Committed, RolledBack, Transaction, TxState};
