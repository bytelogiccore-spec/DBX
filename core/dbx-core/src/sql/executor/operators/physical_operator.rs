//! Physical Operator Trait — Volcano Execution Model

use crate::error::DbxResult;
use arrow::array::RecordBatch;
use arrow::datatypes::Schema;

/// 물리 연산자 트레이트 — Volcano 실행 모델 (Pull 기반)
pub trait PhysicalOperator: Send {
    /// 출력 스키마 반환
    fn schema(&self) -> &Schema;

    /// 다음 RecordBatch 반환 (None이면 끝)
    fn next(&mut self) -> DbxResult<Option<RecordBatch>>;

    /// 연산자 상태 초기화 (재실행용)
    fn reset(&mut self) -> DbxResult<()>;
}
