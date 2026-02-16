//! EventHook System
//!
//! 이벤트 기반 자동화 훅 시스템 (Rust 클로저 기반)

pub mod core;
pub mod event;

pub use core::{EventHook, EventHookAction, EventHookCondition};
pub use event::{EventHookEvent, EventHookEventType};
