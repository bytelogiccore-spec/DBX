//! Trigger System
//!
//! 이벤트 기반 자동화 트리거 시스템

pub mod event;
pub mod trigger;

pub use event::{TriggerEvent, TriggerEventType};
pub use trigger::{Trigger, TriggerAction, TriggerCondition};
