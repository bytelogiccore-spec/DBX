//! Trigger System
//!
//! 이벤트 기반 자동화 트리거 시스템

pub mod core;
pub mod event;

pub use core::{Trigger, TriggerAction, TriggerCondition};
pub use event::{TriggerEvent, TriggerEventType};
