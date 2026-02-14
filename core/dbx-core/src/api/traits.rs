//! API 트레이트 정의

use crate::error::DbxResult;
use arrow::array::{Array, ArrayRef, RecordBatch};
use arrow::datatypes::DataType;

/// RecordBatch에서 구조체로 변환하는 트레이트
pub trait FromRow: Sized {
    fn from_row(batch: &RecordBatch, row_idx: usize) -> DbxResult<Self>;
}

/// Rust 타입을 Arrow DataType으로 변환하는 트레이트
pub trait IntoArrowType {
    fn arrow_type() -> DataType;
    fn is_nullable() -> bool;
}

/// Arrow Array에서 Rust 타입으로 변환하는 트레이트
pub trait FromColumn: Sized {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self>;
}

// 기본 타입 구현
impl IntoArrowType for i32 {
    fn arrow_type() -> DataType {
        DataType::Int32
    }
    fn is_nullable() -> bool {
        false
    }
}

impl FromColumn for i32 {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        use arrow::array::AsArray;
        Ok(column
            .as_primitive::<arrow::datatypes::Int32Type>()
            .value(row_idx))
    }
}

impl IntoArrowType for i64 {
    fn arrow_type() -> DataType {
        DataType::Int64
    }
    fn is_nullable() -> bool {
        false
    }
}

impl FromColumn for i64 {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        use arrow::array::AsArray;
        Ok(column
            .as_primitive::<arrow::datatypes::Int64Type>()
            .value(row_idx))
    }
}

impl IntoArrowType for f64 {
    fn arrow_type() -> DataType {
        DataType::Float64
    }
    fn is_nullable() -> bool {
        false
    }
}

impl FromColumn for f64 {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        use arrow::array::AsArray;
        Ok(column
            .as_primitive::<arrow::datatypes::Float64Type>()
            .value(row_idx))
    }
}

impl IntoArrowType for String {
    fn arrow_type() -> DataType {
        DataType::Utf8
    }
    fn is_nullable() -> bool {
        false
    }
}

impl FromColumn for String {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        use arrow::array::AsArray;
        Ok(column.as_string::<i32>().value(row_idx).to_string())
    }
}

impl IntoArrowType for bool {
    fn arrow_type() -> DataType {
        DataType::Boolean
    }
    fn is_nullable() -> bool {
        false
    }
}

impl FromColumn for bool {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        use arrow::array::AsArray;
        Ok(column.as_boolean().value(row_idx))
    }
}

// Option<T> 구현
impl<T: IntoArrowType> IntoArrowType for Option<T> {
    fn arrow_type() -> DataType {
        T::arrow_type()
    }
    fn is_nullable() -> bool {
        true
    }
}

impl<T: FromColumn> FromColumn for Option<T> {
    fn from_column(column: &ArrayRef, row_idx: usize) -> DbxResult<Self> {
        if column.is_null(row_idx) {
            Ok(None)
        } else {
            Ok(Some(T::from_column(column, row_idx)?))
        }
    }
}
