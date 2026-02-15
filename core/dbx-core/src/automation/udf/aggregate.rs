//! Aggregate UDF
//!
//! 다중 값을 단일 값으로 집계하는 UDF (SUM, AVG, COUNT 등)

use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// Aggregate UDF 상태
pub trait AggregateState: Send + Sync {
    /// 새 값 추가
    fn accumulate(&mut self, value: &Value) -> DbxResult<()>;

    /// 최종 결과 계산
    fn finalize(&self) -> DbxResult<Value>;

    /// 상태 초기화
    fn reset(&mut self);
}

/// Aggregate UDF
pub struct AggregateUDF {
    name: String,
    signature: Signature,
    create_state: Box<dyn Fn() -> Box<dyn AggregateState> + Send + Sync>,
}

impl AggregateUDF {
    /// 새 Aggregate UDF 생성
    pub fn new<F, S>(name: impl Into<String>, signature: Signature, create_state: F) -> Self
    where
        F: Fn() -> S + Send + Sync + 'static,
        S: AggregateState + 'static,
    {
        Self {
            name: name.into(),
            signature,
            create_state: Box::new(move || Box::new(create_state())),
        }
    }

    /// 값 배열에 대해 집계 수행
    pub fn aggregate(&self, values: &[Value]) -> DbxResult<Value> {
        let mut state = (self.create_state)();

        for value in values {
            state.accumulate(value)?;
        }

        state.finalize()
    }
}

impl Callable for AggregateUDF {
    fn call(&self, _ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
        // args는 집계할 값들의 배열
        self.aggregate(args)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::callable::DataType;

    // SUM 집계 상태
    struct SumState {
        sum: i64,
    }

    impl AggregateState for SumState {
        fn accumulate(&mut self, value: &Value) -> DbxResult<()> {
            self.sum += value.as_i64()?;
            Ok(())
        }

        fn finalize(&self) -> DbxResult<Value> {
            Ok(Value::Int(self.sum))
        }

        fn reset(&mut self) {
            self.sum = 0;
        }
    }

    // COUNT 집계 상태
    struct CountState {
        count: i64,
    }

    impl AggregateState for CountState {
        fn accumulate(&mut self, _value: &Value) -> DbxResult<()> {
            self.count += 1;
            Ok(())
        }

        fn finalize(&self) -> DbxResult<Value> {
            Ok(Value::Int(self.count))
        }

        fn reset(&mut self) {
            self.count = 0;
        }
    }

    // AVG 집계 상태
    struct AvgState {
        sum: f64,
        count: i64,
    }

    impl AggregateState for AvgState {
        fn accumulate(&mut self, value: &Value) -> DbxResult<()> {
            self.sum += value.as_i64()? as f64;
            self.count += 1;
            Ok(())
        }

        fn finalize(&self) -> DbxResult<Value> {
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

    #[test]
    fn test_aggregate_udf_sum() {
        let sum_udf = AggregateUDF::new(
            "sum",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: true,
            },
            || SumState { sum: 0 },
        );

        let values = vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)];
        let result = sum_udf.aggregate(&values).unwrap();

        assert_eq!(result.as_i64().unwrap(), 10);
    }

    #[test]
    fn test_aggregate_udf_count() {
        let count_udf = AggregateUDF::new(
            "count",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: true,
            },
            || CountState { count: 0 },
        );

        let values = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let result = count_udf.aggregate(&values).unwrap();

        assert_eq!(result.as_i64().unwrap(), 3);
    }

    #[test]
    fn test_aggregate_udf_avg() {
        let avg_udf = AggregateUDF::new(
            "avg",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Float,
                is_variadic: true,
            },
            || AvgState { sum: 0.0, count: 0 },
        );

        let values = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
        let result = avg_udf.aggregate(&values).unwrap();

        assert_eq!(result.as_f64().unwrap(), 20.0);
    }

    #[test]
    fn test_aggregate_udf_empty() {
        let avg_udf = AggregateUDF::new(
            "avg",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Float,
                is_variadic: true,
            },
            || AvgState { sum: 0.0, count: 0 },
        );

        let values = vec![];
        let result = avg_udf.aggregate(&values).unwrap();

        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn test_aggregate_udf_with_engine() {
        use crate::automation::ExecutionEngine;
        use crate::engine::Database;
        use std::sync::Arc;

        let engine = ExecutionEngine::new();

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

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let result = engine
            .execute(
                "sum",
                &ctx,
                &[Value::Int(5), Value::Int(10), Value::Int(15)],
            )
            .unwrap();

        assert_eq!(result.as_i64().unwrap(), 30);
    }
}
