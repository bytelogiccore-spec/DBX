//! Vectorized UDF implementation
//!
//! Vectorized UDF: 다수 값(콜럼/배열) → 다수 값(콜럼/배열) 변환 함수

use crate::automation::callable::{Callable, DataType, ExecutionContext, Signature, Value};
use crate::error::{DbxError, DbxResult};

/// Type alias for vectorized UDF function
type VectorizedFn = Box<dyn Fn(&[Value]) -> DbxResult<Value> + Send + Sync>;

/// Vectorized UDF (배열 단위 처리)
pub struct VectorizedUDF {
    name: String,
    signature: Signature,
    func: VectorizedFn,
}

impl VectorizedUDF {
    /// 새 Vectorized UDF 생성
    pub fn new<F>(name: impl Into<String>, signature: Signature, func: F) -> Self
    where
        F: Fn(&[Value]) -> DbxResult<Value> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            signature,
            func: Box::new(func),
        }
    }
}

impl Callable for VectorizedUDF {
    fn call(&self, _ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
        // 1. Signature 인자 개수 및 Array 타입 검증
        self.signature.validate_args(args)?;

        // 2. 추가 검증: 입력된 모든 인자가 Array인지 재확인 (방어적 프로그래밍)
        // (Signature.params에 DataType::Array로 명시되어 있다면 validate_args에서 이미 검증됨)
        for arg in args {
            if arg.data_type() != DataType::Array {
                return Err(DbxError::TypeMismatch {
                    expected: "Array".to_string(),
                    actual: format!("{:?}", arg.data_type()),
                });
            }
        }

        // 3. 배열들의 길이가 모두 일치하는지 검증
        let mut expected_len = None;
        for arg in args {
            let arr = arg.as_array()?;
            match expected_len {
                None => expected_len = Some(arr.len()),
                Some(len) => {
                    if len != arr.len() {
                        return Err(DbxError::InvalidArguments(
                            "Vectorized UDF arguments must have the same array length".to_string()
                        ));
                    }
                }
            }
        }

        // 4. 함수 실행
        let result = (self.func)(args)?;

        // 5. 결과 타입 검증 (반드시 Array를 반환해야 함)
        if result.data_type() != DataType::Array {
            return Err(DbxError::TypeMismatch {
                expected: "Array".to_string(),
                actual: format!("{:?}", result.data_type()),
            });
        }

        // 6. 반환 배열 길이 검증 (입력 길이와 같아야 함, 단 입력이 없었던 경우는 예외)
        if let Some(expected) = expected_len {
            let res_arr = result.as_array()?;
            if res_arr.len() != expected {
                return Err(DbxError::InvalidArguments(
                    "Vectorized UDF result array length must match input array lengths".to_string()
                ));
            }
        }

        Ok(result)
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
    use crate::automation::executor::ExecutionEngine;
    use crate::engine::Database;
    use std::sync::Arc;

    #[test]
    fn test_vectorized_udf_basic() {
        // UDF: 배열의 각 요소에 대해 x * 2를 수행
        let udf = VectorizedUDF::new(
            "vec_double",
            Signature {
                params: vec![DataType::Array],
                return_type: DataType::Array,
                is_variadic: false,
            },
            |args| {
                let input_array = args[0].as_array()?;
                let mut result = Vec::with_capacity(input_array.len());
                for val in input_array {
                    // 내부 요소가 Int라고 가정
                    let x = val.as_i64()?;
                    result.push(Value::Int(x * 2));
                }
                Ok(Value::Array(result))
            },
        );

        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        
        let input_vals = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
        let result = udf.call(&ctx, &[Value::Array(input_vals)]).unwrap();

        let out_arr = result.as_array().unwrap();
        assert_eq!(out_arr.len(), 3);
        assert_eq!(out_arr[0].as_i64().unwrap(), 20);
        assert_eq!(out_arr[1].as_i64().unwrap(), 40);
        assert_eq!(out_arr[2].as_i64().unwrap(), 60);
    }

    #[test]
    fn test_vectorized_udf_multiple_args() {
        // UDF: 배열 콜럼 2개를 받아 x + y를 수행
        let udf = VectorizedUDF::new(
            "vec_add",
            Signature {
                params: vec![DataType::Array, DataType::Array],
                return_type: DataType::Array,
                is_variadic: false,
            },
            |args| {
                let x_arr = args[0].as_array()?;
                let y_arr = args[1].as_array()?;
                
                let mut result = Vec::with_capacity(x_arr.len());
                for i in 0..x_arr.len() {
                    let x = x_arr[i].as_i64()?;
                    let y = y_arr[i].as_i64()?;
                    result.push(Value::Int(x + y));
                }
                Ok(Value::Array(result))
            },
        );

        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        
        let x_vals = vec![Value::Int(1), Value::Int(2), Value::Int(3)];
        let y_vals = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
        
        let result = udf.call(&ctx, &[Value::Array(x_vals), Value::Array(y_vals)]).unwrap();

        let out_arr = result.as_array().unwrap();
        assert_eq!(out_arr.len(), 3);
        assert_eq!(out_arr[0].as_i64().unwrap(), 11);
        assert_eq!(out_arr[1].as_i64().unwrap(), 22);
        assert_eq!(out_arr[2].as_i64().unwrap(), 33);
    }

    #[test]
    fn test_vectorized_udf_mismatch_lengths() {
        // UDF: 길이가 다른 두 콜럼 전달 시 에러를 반환해야 함
        let udf = VectorizedUDF::new(
            "vec_add",
            Signature {
                params: vec![DataType::Array, DataType::Array],
                return_type: DataType::Array,
                is_variadic: false,
            },
            | _args | {
                Ok(Value::Array(vec![])) // 실제 실행 전 프레임워크 레벨에서 걸러져야 함 
            },
        );

        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        
        let x_vals = vec![Value::Int(1), Value::Int(2)];
        let y_vals = vec![Value::Int(10), Value::Int(20), Value::Int(30)];
        
        // 길이가 다르므로 InvalidArguments 에러가 나야 함
        let result = udf.call(&ctx, &[Value::Array(x_vals), Value::Array(y_vals)]);
        assert!(result.is_err());
        match result.unwrap_err() {
            DbxError::InvalidArguments(msg) => assert!(msg.contains("same array length")),
            _ => panic!("Expected InvalidArguments error"),
        }
    }

    #[test]
    fn test_vectorized_udf_with_engine() {
        let engine = ExecutionEngine::new();

        let udf = Arc::new(VectorizedUDF::new(
            "vec_triple",
            Signature {
                params: vec![DataType::Array],
                return_type: DataType::Array,
                is_variadic: false,
            },
            |args| {
                let input_array = args[0].as_array()?;
                let mut result = Vec::with_capacity(input_array.len());
                for val in input_array {
                    let x = val.as_i64()?;
                    result.push(Value::Int(x * 3));
                }
                Ok(Value::Array(result))
            },
        ));

        engine.register(udf).unwrap();

        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        let result = engine.execute(
            "vec_triple", 
            &ctx, 
            &[Value::Array(vec![Value::Int(5), Value::Int(10)])]
        ).unwrap();

        let out_arr = result.as_array().unwrap();
        assert_eq!(out_arr[0].as_i64().unwrap(), 15);
        assert_eq!(out_arr[1].as_i64().unwrap(), 30);
    }
}
