//! Sharding Module — Hash 기반 수평 분할 MVP
//!
//! # 모듈 구성
//!
//! - [`router::ShardRouter`]: FNV1a 해시 기반 키→샤드 라우팅
//! - [`scatter_gather::ScatterGather`]: 분산 쿼리 실행 (scatter) + 결과 병합 (gather)

pub mod router;
pub mod scatter_gather;
pub mod node_ring;
pub mod rebalancer;
pub mod two_phase;

pub use router::{ShardNode, ShardRouter};
pub use scatter_gather::ScatterGather;
pub use node_ring::NodeRing;
pub use rebalancer::{Rebalancer, MigrationTask, rebalancer_on_add, rebalancer_on_remove};
