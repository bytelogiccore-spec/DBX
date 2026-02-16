//! Trigger Execution Engine
//!
//! Trigger 실행 엔진: 이벤트 발생 시 SQL Trigger 자동 실행

use crate::automation::{Trigger, TriggerOperation, TriggerTiming};
use crate::engine::Database;
use crate::error::DbxResult;

/// Trigger 실행 엔진
pub struct TriggerExecutor {
    /// 등록된 Trigger 목록
    triggers: Vec<Trigger>,
}

impl TriggerExecutor {
    /// 새 Trigger 실행 엔진 생성
    pub fn new() -> Self {
        Self {
            triggers: Vec::new(),
        }
    }

    /// Trigger 등록
    pub fn register(&mut self, trigger: Trigger) {
        self.triggers.push(trigger);
    }

    /// 모든 Trigger 등록
    pub fn register_all(&mut self, triggers: Vec<Trigger>) {
        self.triggers.extend(triggers);
    }

    /// Trigger 제거
    pub fn unregister(&mut self, name: &str) -> bool {
        if let Some(pos) = self.triggers.iter().position(|t| t.name == name) {
            self.triggers.remove(pos);
            true
        } else {
            false
        }
    }

    /// 등록된 모든 Trigger 조회
    pub fn list_triggers(&self) -> &[Trigger] {
        &self.triggers
    }

    /// BEFORE 이벤트 처리
    pub fn fire_before(
        &self,
        db: &Database,
        operation: TriggerOperation,
        table: &str,
    ) -> DbxResult<()> {
        self.fire_triggers(db, TriggerTiming::Before, operation, table)
    }

    /// AFTER 이벤트 처리
    pub fn fire_after(
        &self,
        db: &Database,
        operation: TriggerOperation,
        table: &str,
    ) -> DbxResult<()> {
        self.fire_triggers(db, TriggerTiming::After, operation, table)
    }

    /// WHEN 조건 결과 평가
    ///
    /// RecordBatch 결과를 받아서 true/false로 평가합니다.
    /// - 결과가 비어있으면 false
    /// - 첨번째 행의 첨번째 열이 true/1/non-zero면 true
    fn evaluate_condition_result(result: &[arrow::record_batch::RecordBatch]) -> bool {
        use arrow::array::AsArray;

        if result.is_empty() {
            return false;
        }

        let batch = &result[0];
        if batch.num_rows() == 0 || batch.num_columns() == 0 {
            return false;
        }

        // 첨번째 열 가져오기
        let column = batch.column(0);

        // 타입에 따라 평가
        use arrow::datatypes::DataType;
        match column.data_type() {
            DataType::Boolean => {
                let bool_array = column.as_boolean();
                bool_array.value(0)
            }
            DataType::Int64 | DataType::Int32 | DataType::Int16 | DataType::Int8 => {
                let int_array = column.as_primitive::<arrow::datatypes::Int64Type>();
                int_array.value(0) != 0
            }
            DataType::UInt64 | DataType::UInt32 | DataType::UInt16 | DataType::UInt8 => {
                let uint_array = column.as_primitive::<arrow::datatypes::UInt64Type>();
                uint_array.value(0) != 0
            }
            DataType::Float64 | DataType::Float32 => {
                let float_array = column.as_primitive::<arrow::datatypes::Float64Type>();
                float_array.value(0) != 0.0
            }
            DataType::Utf8 => {
                let str_array = column.as_string::<i32>();
                let val = str_array.value(0);
                !val.is_empty() && val != "0" && val.to_lowercase() != "false"
            }
            _ => false, // 기타 타입은 false
        }
    }

    /// Trigger 실행
    fn fire_triggers(
        &self,
        db: &Database,
        timing: TriggerTiming,
        operation: TriggerOperation,
        table: &str,
    ) -> DbxResult<()> {
        // 조건에 맞는 Trigger 필터링
        let matching_triggers: Vec<&Trigger> = self
            .triggers
            .iter()
            .filter(|t| t.timing == timing && t.operation == operation && t.table == table)
            .collect();

        // 각 Trigger의 SQL 문장 실행
        for trigger in matching_triggers {
            // WHEN 조건 평가
            if let Some(ref condition) = trigger.condition {
                // 조건 SQL 실행 (SELECT 문으로 평가)
                match db.execute_sql(condition) {
                    Ok(result) => {
                        // 결과가 true인지 확인
                        if !Self::evaluate_condition_result(&result) {
                            #[cfg(debug_assertions)]
                            println!(
                                "[Trigger] Skipping '{}' - WHEN condition evaluated to false",
                                trigger.name
                            );
                            continue; // 조건 false면 스킵
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[Trigger] Failed to evaluate WHEN condition for '{}': {}",
                            trigger.name, e
                        );
                        continue; // 조건 평가 실패 시 스킵
                    }
                }
            }

            // Trigger body의 각 SQL 문장 실행
            for sql in &trigger.body {
                // 실제 SQL 실행
                match db.execute_sql(sql) {
                    Ok(_) => {
                        #[cfg(debug_assertions)]
                        println!(
                            "[Trigger] Successfully executed '{}' on {:?} {:?}: {}",
                            trigger.name, timing, operation, sql
                        );
                    }
                    Err(e) => {
                        // Trigger 실행 실패 시 경고 로그 출력 (에러 전파 안 함)
                        eprintln!(
                            "[Trigger] Failed to execute '{}': {} (SQL: {})",
                            trigger.name, e, sql
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for TriggerExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::ForEachType;

    #[test]
    fn test_trigger_executor_register() {
        let mut executor = TriggerExecutor::new();

        let trigger = Trigger::new(
            "test_trigger",
            TriggerTiming::After,
            TriggerOperation::Insert,
            "users",
            ForEachType::Row,
            None,
            vec!["INSERT INTO audit_logs VALUES (1, 'test')".to_string()],
        );

        executor.register(trigger);
        assert_eq!(executor.list_triggers().len(), 1);
    }

    #[test]
    fn test_trigger_executor_unregister() {
        let mut executor = TriggerExecutor::new();

        let trigger = Trigger::new(
            "test_trigger",
            TriggerTiming::After,
            TriggerOperation::Insert,
            "users",
            ForEachType::Row,
            None,
            vec!["INSERT INTO audit_logs VALUES (1, 'test')".to_string()],
        );

        executor.register(trigger);
        assert_eq!(executor.list_triggers().len(), 1);

        let removed = executor.unregister("test_trigger");
        assert!(removed);
        assert_eq!(executor.list_triggers().len(), 0);
    }

    #[test]
    fn test_trigger_executor_filter() {
        let mut executor = TriggerExecutor::new();

        // AFTER INSERT trigger
        executor.register(Trigger::new(
            "after_insert",
            TriggerTiming::After,
            TriggerOperation::Insert,
            "users",
            ForEachType::Row,
            None,
            vec!["INSERT INTO logs VALUES (1)".to_string()],
        ));

        // BEFORE UPDATE trigger
        executor.register(Trigger::new(
            "before_update",
            TriggerTiming::Before,
            TriggerOperation::Update,
            "users",
            ForEachType::Row,
            None,
            vec!["UPDATE stats SET count = count + 1".to_string()],
        ));

        assert_eq!(executor.list_triggers().len(), 2);
    }
}
