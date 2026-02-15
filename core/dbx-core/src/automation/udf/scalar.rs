//! Scalar UDF implementation
//!
//! Scalar UDF: 단일 값 → 단일 값 변환 함수

use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// Scalar UDF (단일 값 → 단일 값)
pub struct ScalarUDF {
    name: String,
    signature: Signature,
    func: Box<dyn Fn(&[Value]) -> DbxResult<Value> + Send + Sync>,
}

impl ScalarUDF {
    /// 새 Scalar UDF 생성
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

impl Callable for ScalarUDF {
    fn call(&self, _ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
        // 타입 검증
        self.signature.validate_args(args)?;
        
        // 함수 실행
        (self.func)(args)
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
    use crate::automation::executor::ExecutionEngine;
    use crate::engine::Database;
    use std::sync::Arc;
    
    #[test]
    fn test_scalar_udf_basic() {
        // UDF: x * 2
        let udf = ScalarUDF::new(
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
        );
        
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        let result = udf.call(&ctx, &[Value::Int(21)]).unwrap();
        
        assert_eq!(result.as_i64().unwrap(), 42);
    }
    
    #[test]
    fn test_scalar_udf_string() {
        // UDF: 문자열 대문자 변환
        let udf = ScalarUDF::new(
            "upper",
            Signature {
                params: vec![DataType::String],
                return_type: DataType::String,
                is_variadic: false,
            },
            |args| {
                let s = args[0].as_str()?;
                Ok(Value::String(s.to_uppercase()))
            },
        );
        
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        let result = udf.call(&ctx, &[Value::String("hello".to_string())]).unwrap();
        
        assert_eq!(result.as_str().unwrap(), "HELLO");
    }
    
    #[test]
    fn test_scalar_udf_multiple_args() {
        // UDF: x + y
        let udf = ScalarUDF::new(
            "add",
            Signature {
                params: vec![DataType::Int, DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| {
                let x = args[0].as_i64()?;
                let y = args[1].as_i64()?;
                Ok(Value::Int(x + y))
            },
        );
        
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        let result = udf.call(&ctx, &[Value::Int(10), Value::Int(32)]).unwrap();
        
        assert_eq!(result.as_i64().unwrap(), 42);
    }
    
    #[test]
    fn test_scalar_udf_type_validation() {
        // UDF: x * 2 (Int만 허용)
        let udf = ScalarUDF::new(
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
        );
        
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        
        // String 전달 시 에러
        let result = udf.call(&ctx, &[Value::String("hello".to_string())]);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_scalar_udf_with_engine() {
        let engine = ExecutionEngine::new();
        
        // UDF 등록
        let udf = Arc::new(ScalarUDF::new(
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
        
        engine.register(udf).unwrap();
        
        // 실행
        let ctx = ExecutionContext::new(Arc::new(Database::open_in_memory().unwrap()));
        let result = engine.execute("triple", &ctx, &[Value::Int(14)]).unwrap();
        
        assert_eq!(result.as_i64().unwrap(), 42);
    }
}
