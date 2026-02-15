//! Integration Tests
//!
//! UDF, 트리거, 스케줄러 통합 테스트

#[cfg(test)]
mod integration_tests {
    use crate::automation::callable::{DataType, ExecutionContext, Signature, Value};
    use crate::automation::{
        AggregateState, AggregateUDF, ExecutionEngine, ScalarUDF, Schedule, ScheduleType,
        ScheduledJob, Scheduler, Trigger, TriggerAction, TriggerCondition, TriggerEvent,
        TriggerEventType,
    };
    use crate::engine::Database;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// 테스트 1: 트리거에서 Scalar UDF 호출
    #[test]
    fn test_trigger_calls_scalar_udf() {
        let engine = Arc::new(ExecutionEngine::new());

        // Scalar UDF 등록: 값을 2배로
        let double_udf = Arc::new(ScalarUDF::new(
            "double",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| {
                let x = args[0].as_i64()?;
                Ok(Value::Int(x * 2))
            },
        ));
        engine.register(double_udf).unwrap();

        // 트리거 액션: UDF 호출 결과를 저장
        let result = Arc::new(Mutex::new(0i64));
        let result_clone = Arc::clone(&result);
        let engine_clone = Arc::clone(&engine);

        let trigger = Trigger::new(
            "double_trigger",
            TriggerEventType::AfterInsert,
            "users",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |ctx, event| {
                if let Some(value) = event.data.get("id") {
                    // UDF 호출
                    let doubled = engine_clone.execute("double", ctx, &[value.clone()])?;
                    *result_clone.lock().unwrap() = doubled.as_i64()?;
                }
                Ok(())
            })),
        );

        // 트리거 실행
        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "users")
            .with_data("id", Value::Int(21));

        trigger.execute_action(&ctx, &event).unwrap();

        // 결과 확인: 21 * 2 = 42
        assert_eq!(*result.lock().unwrap(), 42);
    }

    /// 테스트 2: 트리거에서 Aggregate UDF 호출
    #[test]
    fn test_trigger_calls_aggregate_udf() {
        let engine = Arc::new(ExecutionEngine::new());

        // Aggregate UDF 등록: SUM
        struct SumState {
            sum: i64,
        }
        impl AggregateState for SumState {
            fn accumulate(&mut self, value: &Value) -> crate::error::DbxResult<()> {
                self.sum += value.as_i64()?;
                Ok(())
            }
            fn finalize(&self) -> crate::error::DbxResult<Value> {
                Ok(Value::Int(self.sum))
            }
            fn reset(&mut self) {
                self.sum = 0;
            }
        }

        let sum_udf = Arc::new(AggregateUDF::new(
            "sum",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: true,
            },
            || SumState { sum: 0 },
        ));
        engine.register(sum_udf).unwrap();

        // 트리거: 여러 값의 합계 계산
        let result = Arc::new(Mutex::new(0i64));
        let result_clone = Arc::clone(&result);
        let engine_clone = Arc::clone(&engine);

        let trigger = Trigger::new(
            "sum_trigger",
            TriggerEventType::AfterInsert,
            "orders",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |ctx, _event| {
                let values = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
                let sum = engine_clone.execute("sum", ctx, &values)?;
                *result_clone.lock().unwrap() = sum.as_i64()?;
                Ok(())
            })),
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "orders");

        trigger.execute_action(&ctx, &event).unwrap();

        assert_eq!(*result.lock().unwrap(), 60);
    }

    /// 테스트 3: 스케줄러에서 Scalar UDF 호출
    #[test]
    fn test_scheduler_calls_scalar_udf() {
        let engine = Arc::new(ExecutionEngine::new());

        // Scalar UDF 등록
        let triple_udf = Arc::new(ScalarUDF::new(
            "triple",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| {
                let x = args[0].as_i64()?;
                Ok(Value::Int(x * 3))
            },
        ));
        engine.register(triple_udf).unwrap();

        // 스케줄러 생성
        let scheduler = Scheduler::new(Arc::clone(&engine));

        // 즉시 실행되는 작업 등록
        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(0)));
        let job = ScheduledJob::new("triple_job", schedule, "triple", vec![Value::Int(14)]);
        scheduler.register(job).unwrap();

        // 실행
        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        std::thread::sleep(Duration::from_millis(100));
        let executed = scheduler.tick(&ctx).unwrap();

        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0], "triple_job");

        // UDF 메트릭 확인
        let metrics = engine.metrics().unwrap();
        assert_eq!(metrics.call_counts.get("triple"), Some(&1));
    }

    /// 테스트 4: 스케줄러에서 Aggregate UDF 호출
    #[test]
    fn test_scheduler_calls_aggregate_udf() {
        let engine = Arc::new(ExecutionEngine::new());

        // Aggregate UDF: AVG
        struct AvgState {
            sum: f64,
            count: i64,
        }
        impl AggregateState for AvgState {
            fn accumulate(&mut self, value: &Value) -> crate::error::DbxResult<()> {
                self.sum += value.as_i64()? as f64;
                self.count += 1;
                Ok(())
            }
            fn finalize(&self) -> crate::error::DbxResult<Value> {
                if self.count == 0 {
                    Ok(Value::Null)
                } else {
                    Ok(Value::Float(self.sum / self.count as f64))
                }
            }
            fn reset(&mut self) {
                self.sum = 0.0;
                self.count = 0;
            }
        }

        let avg_udf = Arc::new(AggregateUDF::new(
            "avg",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Float,
                is_variadic: true,
            },
            || AvgState { sum: 0.0, count: 0 },
        ));
        engine.register(avg_udf).unwrap();

        let scheduler = Scheduler::new(Arc::clone(&engine));

        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(0)));
        let job = ScheduledJob::new(
            "avg_job",
            schedule,
            "avg",
            vec![Value::Int(10), Value::Int(20), Value::Int(30)],
        );
        scheduler.register(job).unwrap();

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        std::thread::sleep(Duration::from_millis(100));
        let executed = scheduler.tick(&ctx).unwrap();

        assert_eq!(executed.len(), 1);
    }

    /// 테스트 5: 복합 시나리오 - 트리거가 스케줄러 작업을 트리거
    #[test]
    fn test_complex_trigger_scheduler_integration() {
        let engine = Arc::new(ExecutionEngine::new());

        // UDF: 로그 카운터
        let counter = Arc::new(Mutex::new(0i64));
        let counter_clone = Arc::clone(&counter);

        let log_udf = Arc::new(ScalarUDF::new(
            "log_event",
            Signature {
                params: vec![DataType::String],
                return_type: DataType::Null,
                is_variadic: false,
            },
            move |_args| {
                *counter_clone.lock().unwrap() += 1;
                Ok(Value::Null)
            },
        ));
        engine.register(log_udf).unwrap();

        // 트리거: 이벤트 발생 시 로그
        let engine_clone = Arc::clone(&engine);
        let trigger = Trigger::new(
            "log_trigger",
            TriggerEventType::AfterInsert,
            "events",
            TriggerCondition::Always,
            TriggerAction::Custom(Box::new(move |ctx, _event| {
                engine_clone.execute("log_event", ctx, &[Value::String("event".to_string())])?;
                Ok(())
            })),
        );

        // 스케줄러: 주기적으로 로그
        let scheduler = Scheduler::new(Arc::clone(&engine));
        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(0)));
        let job = ScheduledJob::new(
            "periodic_log",
            schedule,
            "log_event",
            vec![Value::String("scheduled".to_string())],
        );
        scheduler.register(job).unwrap();

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // 트리거 실행
        let event = TriggerEvent::new(TriggerEventType::AfterInsert, "events");
        trigger.execute_action(&ctx, &event).unwrap();

        // 스케줄러 실행
        std::thread::sleep(Duration::from_millis(100));
        scheduler.tick(&ctx).unwrap();

        // 총 2번 호출되어야 함 (트리거 1번 + 스케줄러 1번)
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    /// 테스트 6: 모든 UDF 타입이 ExecutionEngine에서 동작
    #[test]
    fn test_all_udf_types_in_engine() {
        let engine = ExecutionEngine::new();

        // Scalar UDF
        let scalar = Arc::new(ScalarUDF::new(
            "scalar",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| Ok(Value::Int(args[0].as_i64()? + 1)),
        ));

        // Aggregate UDF
        struct CountState {
            count: i64,
        }
        impl AggregateState for CountState {
            fn accumulate(&mut self, _value: &Value) -> crate::error::DbxResult<()> {
                self.count += 1;
                Ok(())
            }
            fn finalize(&self) -> crate::error::DbxResult<Value> {
                Ok(Value::Int(self.count))
            }
            fn reset(&mut self) {
                self.count = 0;
            }
        }

        let aggregate = Arc::new(AggregateUDF::new(
            "count",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: true,
            },
            || CountState { count: 0 },
        ));

        engine.register(scalar).unwrap();
        engine.register(aggregate).unwrap();

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // Scalar 실행
        let r1 = engine.execute("scalar", &ctx, &[Value::Int(5)]).unwrap();
        assert_eq!(r1.as_i64().unwrap(), 6);

        // Aggregate 실행
        let r2 = engine
            .execute(
                "count",
                &ctx,
                &[Value::Int(1), Value::Int(2), Value::Int(3)],
            )
            .unwrap();
        assert_eq!(r2.as_i64().unwrap(), 3);

        // 메트릭 확인
        let metrics = engine.metrics().unwrap();
        assert_eq!(metrics.call_counts.get("scalar"), Some(&1));
        assert_eq!(metrics.call_counts.get("count"), Some(&1));
    }
}
