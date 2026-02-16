//! Automation & Extensibility Framework
//!
//! 이 모듈은 DBX의 자동화 및 확장성을 위한 통합 프레임워크를 제공합니다.
//! - UDF (User-Defined Functions)
//! - 트리거 (Triggers)
//! - 스케줄러 (Scheduler)
//! - Stored Procedures

pub mod callable;
pub mod event_hook;
pub mod executor;
pub mod function_parser;
pub mod procedure;
pub mod procedure_executor;
pub mod procedure_parser;
pub mod schedule;
pub mod schedule_executor;
pub mod schedule_parser;
pub mod scheduler;
pub mod scheduler_thread; // 새로 추가
pub mod trigger;
pub mod trigger_executor;
pub mod trigger_parser;
pub mod udf;

#[cfg(test)]
mod integration_tests;

pub use callable::{Callable, ExecutionContext, Signature};
pub use event_hook::{
    EventHook, EventHookAction, EventHookCondition, EventHookEvent, EventHookEventType,
};
pub use executor::ExecutionEngine;
pub use function_parser::{parse_create_function, parse_drop_function};
pub use procedure::{ProcedureParameter, StoredProcedure};
pub use procedure_executor::ProcedureExecutor;
pub use procedure_parser::{parse_call_procedure, parse_create_procedure, parse_drop_procedure};
pub use schedule::Schedule;
pub use schedule_executor::ScheduleExecutor;
pub use schedule_parser::{parse_create_schedule, parse_drop_schedule};
pub use scheduler::{Schedule as EventSchedule, ScheduleType, ScheduledJob, Scheduler};
pub use scheduler_thread::SchedulerThread; // 새로 추가
pub use trigger::{ForEachType, Trigger, TriggerOperation, TriggerTiming};
pub use trigger_executor::TriggerExecutor;
pub use trigger_parser::{parse_create_trigger, parse_drop_trigger};
pub use udf::{AggregateState, AggregateUDF, ScalarUDF, TableUDF, UdfMetadata, UdfType};
