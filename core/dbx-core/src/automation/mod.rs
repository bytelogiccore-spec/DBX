//! Automation & Extensibility Framework
//!
//! 이 모듈은 DBX의 자동화 및 확장성을 위한 통합 프레임워크를 제공합니다.
//! - UDF (User-Defined Functions)
//! - 트리거 (Triggers)
//! - 스케줄러 (Scheduler)

pub mod callable;
pub mod executor;
pub mod scheduler;
pub mod trigger;
pub mod udf;

#[cfg(test)]
mod integration_tests;

pub use callable::{Callable, ExecutionContext, Signature};
pub use executor::ExecutionEngine;
pub use scheduler::{Schedule, ScheduleType, ScheduledJob, Scheduler};
pub use trigger::{Trigger, TriggerAction, TriggerCondition, TriggerEvent, TriggerEventType};
pub use udf::{AggregateState, AggregateUDF, ScalarUDF, TableUDF};
