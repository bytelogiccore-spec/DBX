//! # DBX — High-Performance Embedded Database
//!
//! DBX는 5-Tier Hybrid Storage 아키텍처 기반의 고성능 임베디드 데이터베이스입니다.
//! 순수 Rust로 구현되었으며, Apache Arrow와 Parquet를 활용한 컬럼형 스토리지를 지원합니다.
//!
//! ## 주요 특징
//!
//! - **5-Tier Hybrid Storage**: Delta → Cache → WOS → Index → ROS
//! - **Apache Arrow 기반**: 컬럼형 스토리지 및 벡터화 연산
//! - **SQL 지원**: SELECT, WHERE, JOIN, GROUP BY, ORDER BY
//! - **ACID 트랜잭션**: Typestate 패턴 기반 타입 안전 트랜잭션
//! - **성능 최적화**: LRU Cache, Bloom Filter, SIMD 벡터화
//!
//! ## 빠른 시작
//!
//! ### 기본 사용 (CRUD)
//!
//! ```rust
//! use dbx_core::Database;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! // 데이터베이스 열기
//! let db = Database::open_in_memory()?;
//!
//! // 데이터 삽입
//! db.insert("users", b"user:1", b"Alice")?;
//!
//! // 데이터 조회
//! let value = db.get("users", b"user:1")?;
//! assert_eq!(value, Some(b"Alice".to_vec()));
//!
//! // 데이터 삭제
//! db.delete("users", b"user:1")?;
//! # Ok(())
//! # }
//! ```
//!
//! ### 트랜잭션
//!
//! ```rust
//! use dbx_core::Database;
//!
//! # fn main() -> dbx_core::DbxResult<()> {
//! let db = Database::open_in_memory()?;
//!
//! // 트랜잭션 시작
//! let _tx = db.begin()?;
//!
//! // 기본 CRUD는 Database 직접 사용
//! db.insert("users", b"user:2", b"Bob")?;
//!
//! // 트랜잭션은 Query Builder와 함께 사용 (Phase 6)
//! # Ok(())
//! # }
//! ```
//!
//! ## 아키텍처
//!
//! ### 5-Tier Hybrid Storage
//!
//! 1. **Delta Store** (DashMap) — 인메모리 쓰기 버퍼, 락 프리 동시성
//! 2. **Cache** (LRU) — 읽기 캐시, 자주 접근하는 데이터
//! 3. **WOS** (sled) — Write-Optimized Store, 영구 저장소
//! 4. **Index** (Bloom Filter) — 빠른 존재 확인
//! 5. **ROS** (Parquet) — Read-Optimized Store, 컬럼형 압축
//!
//! ### SQL 실행 파이프라인
//!
//! ```text
//! SQL 문자열 → Parser → AST → Planner → LogicalPlan
//!          → Optimizer → PhysicalPlan → Executor → RecordBatch
//! ```
//!
//! ## 모듈 구조

//! - [`engine`] — 데이터베이스 엔진 ([`Database`])
//! - [`sql`] — SQL 파서, 플래너, 최적화기, 실행기
//! - [`storage`] — 5-Tier 스토리지 백엔드
//! - [`transaction`] — MVCC 트랜잭션 관리
//! - [`index`] — Hash Index
//! - [`wal`] — Write-Ahead Log

pub mod api;
pub mod engine;
pub mod error;
pub mod index;
pub mod sql;
pub mod storage;
pub mod transaction;
pub mod wal;

// Hash index for fast key lookups
// This comment was for `pub mod index;` but the instruction moved `index` up without its comment.
// Keeping the comment here as it was not explicitly removed.

// Write-Ahead Logging for crash recovery
// This comment was for `pub mod wal;` but the instruction moved `wal` up without its comment.
// Keeping the comment here as it was not explicitly removed.

// ===== Re-exports =====
#[cfg(feature = "simd")]
pub mod simd;

// Logging utilities
pub mod logging;

// Re-export commonly used types
pub use engine::{Database, DurabilityLevel};
pub use error::{DbxError, DbxResult};

// Re-export derive macros
pub use dbx_derive::Table;
