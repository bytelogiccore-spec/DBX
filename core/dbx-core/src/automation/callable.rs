//! Callable trait and execution context
//!
//! 모든 실행 가능한 객체(UDF, 트리거, 스케줄 작업)의 공통 인터페이스

use crate::error::{DbxError, DbxResult};
use crate::engine::Database;
use std::collections::HashMap;
use std::sync::Arc;

/// 실행 가능한 모든 객체의 공통 인터페이스
pub trait Callable: Send + Sync {
    /// 함수 실행
    fn call(&self, ctx: &ExecutionContext, args: &[Value]) -> DbxResult<Value>;
    
    /// 함수 이름
    fn name(&self) -> &str;
    
    /// 함수 시그니처
    fn signature(&self) -> &Signature;
}

/// 실행 컨텍스트
pub struct ExecutionContext {
    /// DBX 인스턴스 참조
    pub dbx: Arc<Database>,
    /// 트랜잭션 ID (옵션)
    pub tx_id: Option<u64>,
    /// 메타데이터
    pub metadata: HashMap<String, Value>,
}

impl ExecutionContext {
    pub fn new(dbx: Arc<Database>) -> Self {
        Self {
            dbx,
            tx_id: None,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_tx(mut self, tx_id: u64) -> Self {
        self.tx_id = Some(tx_id);
        self
    }
}

/// 함수 시그니처
#[derive(Debug, Clone)]
pub struct Signature {
    pub params: Vec<DataType>,
    pub return_type: DataType,
    pub is_variadic: bool,
}

impl Signature {
    /// 인자 타입 검증
    pub fn validate_args(&self, args: &[Value]) -> DbxResult<()> {
        if !self.is_variadic && args.len() != self.params.len() {
            return Err(DbxError::InvalidArguments(format!(
                "Expected {} arguments, got {}",
                self.params.len(),
                args.len()
            )));
        }
        
        if self.is_variadic && args.len() < self.params.len() {
            return Err(DbxError::InvalidArguments(format!(
                "Expected at least {} arguments, got {}",
                self.params.len(),
                args.len()
            )));
        }
        
        // 타입 검증
        for (i, (expected, actual)) in self.params.iter().zip(args.iter()).enumerate() {
            if !actual.matches_type(expected) {
                return Err(DbxError::TypeMismatch {
                    expected: format!("{:?}", expected),
                    actual: format!("{:?}", actual.data_type()),
                });
            }
        }
        
        Ok(())
    }
}

/// 데이터 타입
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Null,
    Boolean,
    Int,
    Float,
    String,
    Bytes,
    Row,
    RecordBatch,
    Table,
}

/// 값
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    /// 테이블 결과 (행 × 열)
    Table(Vec<Vec<Value>>),
}

impl Value {
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Null => DataType::Null,
            Value::Boolean(_) => DataType::Boolean,
            Value::Int(_) => DataType::Int,
            Value::Float(_) => DataType::Float,
            Value::String(_) => DataType::String,
            Value::Bytes(_) => DataType::Bytes,
            Value::Table(_) => DataType::Table,
        }
    }
    
    pub fn matches_type(&self, expected: &DataType) -> bool {
        match (self, expected) {
            (Value::Null, _) => true,  // Null은 모든 타입과 호환
            _ => &self.data_type() == expected,
        }
    }
    
    // 타입 변환 헬퍼
    pub fn as_bool(&self) -> DbxResult<bool> {
        match self {
            Value::Boolean(b) => Ok(*b),
            _ => Err(DbxError::TypeMismatch {
                expected: "Boolean".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }
    
    pub fn as_i64(&self) -> DbxResult<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err(DbxError::TypeMismatch {
                expected: "Int".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }
    
    pub fn as_f64(&self) -> DbxResult<f64> {
        match self {
            Value::Float(f) => Ok(*f),
            _ => Err(DbxError::TypeMismatch {
                expected: "Float".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }
    
    pub fn as_str(&self) -> DbxResult<&str> {
        match self {
            Value::String(s) => Ok(s),
            _ => Err(DbxError::TypeMismatch {
                expected: "String".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }
    
    pub fn as_bytes(&self) -> DbxResult<&[u8]> {
        match self {
            Value::Bytes(b) => Ok(b),
            _ => Err(DbxError::TypeMismatch {
                expected: "Bytes".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }

    /// truthy 판단 (트리거 조건 평가용)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Boolean(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Bytes(b) => !b.is_empty(),
            Value::Table(t) => !t.is_empty(),
        }
    }

    /// 테이블 데이터 반환
    pub fn as_table(&self) -> DbxResult<&Vec<Vec<Value>>> {
        match self {
            Value::Table(t) => Ok(t),
            _ => Err(DbxError::TypeMismatch {
                expected: "Table".to_string(),
                actual: format!("{:?}", self.data_type()),
            }),
        }
    }
}
