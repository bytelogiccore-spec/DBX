//! Native WOS — self-implemented Write-Optimized Store
//!
//! sled를 제거하고 WAL + SSTable 방식으로 직접 구현한 Tier 3 스토리지.
//! DashMap으로 테이블별 독립 락을 사용하여 동시성을 개선한다.

pub mod backend;
pub mod page;
pub mod table_store;

pub use backend::NativeWosBackend;
