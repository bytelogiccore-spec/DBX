//! Replication Module — WAL 기반 Master-Slave 복제 MVP
//!
//! # 아키텍처
//!
//! - [`master::ReplicationMaster`]: WAL append 시 Broadcast 채널로 전송
//! - [`slave::ReplicationSlave`]: 채널에서 수신 → 로컬 적용
//! - [`protocol::ReplicationMessage`]: 복제 프로토콜 메시지
//!
//! # MVP 범위
//!
//! TCP 구현 전에 `tokio::sync::broadcast` 인메모리 채널로 먼저 테스트합니다.

pub mod master;
pub mod protocol;
pub mod slave;
pub mod node;
pub mod vector_clock;
pub mod transport;

pub use master::ReplicationMaster;
pub use protocol::ReplicationMessage;
pub use slave::ReplicationSlave;
pub use node::{ReplicationNode, NodeRole, NodeError};
pub use vector_clock::{VectorClock, VectorClockOrder};
