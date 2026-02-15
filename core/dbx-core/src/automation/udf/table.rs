//! Table UDF
//!
//! 테이블(RecordBatch)을 반환하는 UDF

use crate::automation::callable::{Callable, ExecutionContext, Signature, Value};
use crate::error::DbxResult;

/// Type alias for table UDF function
type TableFn = Box<dyn Fn(&ExecutionContext, &[Value]) -> DbxResult<Vec<Vec<Value>>> + Send + Sync>;

/// Table UDF
pub struct TableUDF {
    name: String,
    signature: Signature,
    func: TableFn,
}

impl TableUDF {
    /// 새 Table UDF 생성
    pub fn new<F>(name: impl Into<String>, signature: Signature, func: F) -> Self
    where
        F: Fn(&ExecutionContext, &[Value]) -> DbxResult<Vec<Vec<Value>>> + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            signature,
            func: Box::new(func),
        }
    }
}

impl Callable for TableUDF {
    fn call(&self, ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value> {
        let rows = (self.func)(ctx, args)?;
        Ok(Value::Table(rows))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }
}

impl TableUDF {
    /// 테이블 데이터 반환 (실제 구현용)
    pub fn execute(&self, ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Vec<Vec<Value>>> {
        (self.func)(ctx, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::callable::DataType;
    use crate::engine::Database;
    use std::sync::Arc;

    #[test]
    fn test_table_udf_basic() {
        let table_udf = TableUDF::new(
            "generate_series",
            Signature {
                params: vec![DataType::Int, DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |_ctx, args| {
                let start = args[0].as_i64()?;
                let end = args[1].as_i64()?;

                let mut rows = Vec::new();
                for i in start..=end {
                    rows.push(vec![Value::Int(i)]);
                }

                Ok(rows)
            },
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let rows = table_udf
            .execute(&ctx, &[Value::Int(1), Value::Int(5)])
            .unwrap();

        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0][0].as_i64().unwrap(), 1);
        assert_eq!(rows[4][0].as_i64().unwrap(), 5);
    }

    #[test]
    fn test_table_udf_multi_column() {
        let table_udf = TableUDF::new(
            "user_data",
            Signature {
                params: vec![],
                return_type: DataType::String,
                is_variadic: false,
            },
            |_ctx, _args| {
                Ok(vec![
                    vec![Value::Int(1), Value::String("Alice".to_string())],
                    vec![Value::Int(2), Value::String("Bob".to_string())],
                    vec![Value::Int(3), Value::String("Charlie".to_string())],
                ])
            },
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let rows = table_udf.execute(&ctx, &[]).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[1][1].as_str().unwrap(), "Bob");
    }

    #[test]
    fn test_table_udf_with_filter() {
        let table_udf = TableUDF::new(
            "filtered_range",
            Signature {
                params: vec![DataType::Int, DataType::Int, DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |_ctx, args| {
                let start = args[0].as_i64()?;
                let end = args[1].as_i64()?;
                let step = args[2].as_i64()?;

                let mut rows = Vec::new();
                let mut current = start;
                while current <= end {
                    rows.push(vec![Value::Int(current)]);
                    current += step;
                }

                Ok(rows)
            },
        );

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        let rows = table_udf
            .execute(&ctx, &[Value::Int(0), Value::Int(10), Value::Int(2)])
            .unwrap();

        assert_eq!(rows.len(), 6); // 0, 2, 4, 6, 8, 10
        assert_eq!(rows[3][0].as_i64().unwrap(), 6);
    }

    #[test]
    fn test_table_udf_with_engine() {
        use crate::automation::ExecutionEngine;

        let engine = ExecutionEngine::new();

        let table_udf = Arc::new(TableUDF::new(
            "range",
            Signature {
                params: vec![DataType::Int],
                return_type: DataType::Int,
                is_variadic: false,
            },
            |_ctx, args| {
                let n = args[0].as_i64()?;
                let mut rows = Vec::new();
                for i in 0..n {
                    rows.push(vec![Value::Int(i)]);
                }
                Ok(rows)
            },
        ));

        engine.register(table_udf).unwrap();

        let db = Database::open_in_memory().unwrap();
        let ctx = ExecutionContext::new(Arc::new(db));

        // Callable::call을 통한 호출 → Value::Table 반환
        let result = engine.execute("range", &ctx, &[Value::Int(3)]).unwrap();

        // 전체 테이블 결과 확인
        let table = result.as_table().unwrap();
        assert_eq!(table.len(), 3);
        assert_eq!(table[0][0].as_i64().unwrap(), 0);
    }
}
