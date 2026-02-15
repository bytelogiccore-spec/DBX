//! UDF API for Database
//!
//! UDF 등록 및 호출 API

use crate::automation::callable::{DataType, ExecutionContext, Signature, Value};
use crate::automation::ScalarUDF;
use crate::engine::Database;
use crate::error::DbxResult;
use std::sync::Arc;

impl Database {
    /// Scalar UDF 등록
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::automation::callable::{DataType, Signature, Value};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// // UDF: x * 2
    /// db.register_scalar_udf(
    ///     "double",
    ///     Signature {
    ///         params: vec![DataType::Int],
    ///         return_type: DataType::Int,
    ///         is_variadic: false,
    ///     },
    ///     |args| {
    ///         let x = args[0].as_i64()?;
    ///         Ok(Value::Int(x * 2))
    ///     },
    /// )?;
    ///
    /// // UDF 호출
    /// let result = db.call_udf("double", &[Value::Int(21)])?;
    /// assert_eq!(result.as_i64()?, 42);
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_scalar_udf<F>(
        &self,
        name: impl Into<String>,
        signature: Signature,
        func: F,
    ) -> DbxResult<()>
    where
        F: Fn(&[Value]) -> DbxResult<Value> + Send + Sync + 'static,
    {
        let udf = Arc::new(ScalarUDF::new(name, signature, func));
        self.automation_engine.register(udf)
    }

    /// UDF 호출
    ///
    /// # 예제
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use dbx_core::automation::callable::{DataType, Signature, Value};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// db.register_scalar_udf(
    ///     "add",
    ///     Signature {
    ///         params: vec![DataType::Int, DataType::Int],
    ///         return_type: DataType::Int,
    ///         is_variadic: false,
    ///     },
    ///     |args| {
    ///         let x = args[0].as_i64()?;
    ///         let y = args[1].as_i64()?;
    ///         Ok(Value::Int(x + y))
    ///     },
    /// )?;
    ///
    /// let result = db.call_udf("add", &[Value::Int(10), Value::Int(32)])?;
    /// assert_eq!(result.as_i64()?, 42);
    /// # Ok(())
    /// # }
    /// ```
    pub fn call_udf(&self, name: &str, args: &[Value]) -> DbxResult<Value> {
        // Use a temporary in-memory DB for ExecutionContext
        // Note: Ideally ExecutionContext would accept Option<Arc<Database>>
        // to avoid this allocation, but that requires a broader refactor.
        // For now, this is sufficient as UDFs rarely need the DB context.
        let temp_db = Arc::new(Database::open_in_memory()?);
        let ctx = ExecutionContext::new(temp_db);
        self.automation_engine.execute(name, &ctx, args)
    }

    /// 등록된 UDF 목록 조회
    pub fn list_udfs(&self) -> DbxResult<Vec<String>> {
        self.automation_engine.list()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::callable::{DataType, Signature, Value};

    #[test]
    fn test_register_and_call_udf() {
        let db = Database::open_in_memory().unwrap();

        // UDF 등록
        db.register_scalar_udf(
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
        )
        .unwrap();

        // UDF 호출
        let result = db.call_udf("triple", &[Value::Int(14)]).unwrap();
        assert_eq!(result.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_multiple_udfs() {
        let db = Database::open_in_memory().unwrap();

        // UDF 1: double
        db.register_scalar_udf(
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
        )
        .unwrap();

        // UDF 2: add
        db.register_scalar_udf(
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
        )
        .unwrap();

        // 호출
        let r1 = db.call_udf("double", &[Value::Int(21)]).unwrap();
        let r2 = db.call_udf("add", &[Value::Int(10), Value::Int(32)]).unwrap();

        assert_eq!(r1.as_i64().unwrap(), 42);
        assert_eq!(r2.as_i64().unwrap(), 42);
    }

    #[test]
    fn test_list_udfs() {
        let db = Database::open_in_memory().unwrap();

        db.register_scalar_udf(
            "func1",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |args| Ok(args[0].clone()),
        )
        .unwrap();

        db.register_scalar_udf(
            "func2",
            Signature {
                params: vec![DataType::String],
                return_type: DataType::String,
                is_variadic: false,
            },
            |args| Ok(args[0].clone()),
        )
        .unwrap();

        let udfs = db.list_udfs().unwrap();
        assert_eq!(udfs.len(), 2);
        assert!(udfs.contains(&"func1".to_string()));
        assert!(udfs.contains(&"func2".to_string()));
    }
}
