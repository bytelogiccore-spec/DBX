//! SQL Schedule Executor
//!
//! SQL 스케줄 실행 엔진

use crate::automation::{Schedule, SchedulerThread};
use crate::engine::Database;
use crate::error::DbxResult;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

/// SQL 스케줄 실행 엔진
pub struct ScheduleExecutor {
    /// 등록된 스케줄들
    schedules: Arc<RwLock<HashMap<String, Schedule>>>,
    /// 백그라운드 스케줄러 스레드
    scheduler_thread: RwLock<Option<SchedulerThread>>,
}

impl ScheduleExecutor {
    /// 새 ScheduleExecutor 생성
    pub fn new() -> Self {
        Self {
            schedules: Arc::new(RwLock::new(HashMap::new())),
            scheduler_thread: RwLock::new(None),
        }
    }

    /// 스케줄 등록
    pub fn register(&self, schedule: Schedule) -> DbxResult<()> {
        let mut schedules = self.schedules.write().unwrap();
        schedules.insert(schedule.name.clone(), schedule);
        Ok(())
    }

    /// 스케줄 해제
    pub fn unregister(&self, name: &str) -> DbxResult<()> {
        let mut schedules = self.schedules.write().unwrap();
        schedules.remove(name);
        Ok(())
    }

    /// 등록된 모든 스케줄 목록
    pub fn list_schedules(&self) -> Vec<String> {
        let schedules = self.schedules.read().unwrap();
        schedules.keys().cloned().collect()
    }

    /// 특정 스케줄 조회
    pub fn get_schedule(&self, name: &str) -> Option<Schedule> {
        let schedules = self.schedules.read().unwrap();
        schedules.get(name).cloned()
    }

    /// 스케줄 활성화
    pub fn enable(&self, name: &str) -> DbxResult<()> {
        let mut schedules = self.schedules.write().unwrap();
        if let Some(schedule) = schedules.get_mut(name) {
            schedule.enable();
        }
        Ok(())
    }

    /// 스케줄 비활성화
    pub fn disable(&self, name: &str) -> DbxResult<()> {
        let mut schedules = self.schedules.write().unwrap();
        if let Some(schedule) = schedules.get_mut(name) {
            schedule.disable();
        }
        Ok(())
    }

    /// 스케줄 실행
    ///
    /// 지정된 스케줄의 SQL body를 실행합니다.
    ///
    /// # 인자
    ///
    /// * `db` - Database 인스턴스
    /// * `name` - 실행할 스케줄 이름
    ///
    /// # 에러
    ///
    /// - 스케줄이 존재하지 않으면 에러 반환
    /// - 스케줄이 비활성화 상태면 실행하지 않음
    /// - SQL 실행 실패 시 에러 반환
    pub fn execute(&self, db: &crate::engine::Database, name: &str) -> DbxResult<()> {
        // 스케줄 조회
        let schedule = {
            let schedules = self.schedules.read().unwrap();
            schedules.get(name).cloned()
        };

        let schedule = schedule.ok_or_else(|| crate::error::DbxError::InvalidOperation {
            message: format!("Schedule '{}' not found", name),
            context: "EXECUTE SCHEDULE".to_string(),
        })?;

        // 비활성화된 스케줄은 실행하지 않음
        if !schedule.enabled {
            #[cfg(debug_assertions)]
            println!("[Schedule] Skipping disabled schedule '{}'", name);
            return Ok(());
        }

        // Schedule body의 각 SQL 문장 실행
        for sql in &schedule.sql_body {
            match db.execute_sql(sql) {
                Ok(_) => {
                    #[cfg(debug_assertions)]
                    println!("[Schedule] Successfully executed '{}': {}", name, sql);
                }
                Err(e) => {
                    // Schedule 실행 실패 시 에러 반환
                    return Err(crate::error::DbxError::InvalidOperation {
                        message: format!("Failed to execute schedule '{}': {}", name, e),
                        context: sql.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 백그라운드 스케줄러 시작
    ///
    /// # 인자
    ///
    /// * `db_weak` - Database Weak 포인터 (순환 참조 방지)
    pub fn start_scheduler(&self, db_weak: Weak<Database>) -> DbxResult<()> {
        let mut thread = self.scheduler_thread.write().unwrap();

        if thread.is_some() {
            return Ok(()); // 이미 실행 중
        }

        let mut scheduler = SchedulerThread::new();
        scheduler.start(db_weak, Arc::clone(&self.schedules))?;

        *thread = Some(scheduler);
        Ok(())
    }

    /// 백그라운드 스케줄러 중지
    pub fn stop_scheduler(&self) -> DbxResult<()> {
        let mut thread = self.scheduler_thread.write().unwrap();

        if let Some(mut scheduler) = thread.take() {
            scheduler.stop()?;
        }

        Ok(())
    }

    /// 스케줄러가 실행 중인지 확인
    pub fn is_scheduler_running(&self) -> bool {
        let thread = self.scheduler_thread.read().unwrap();
        thread.as_ref().map(|t| t.is_running()).unwrap_or(false)
    }
}

impl Default for ScheduleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list() {
        let executor = ScheduleExecutor::new();
        let schedule = Schedule::new("test", "0 0 * * *", vec![]);

        executor.register(schedule).unwrap();

        let schedules = executor.list_schedules();
        assert_eq!(schedules.len(), 1);
        assert!(schedules.contains(&"test".to_string()));
    }

    #[test]
    fn test_unregister() {
        let executor = ScheduleExecutor::new();
        let schedule = Schedule::new("test", "0 0 * * *", vec![]);

        executor.register(schedule).unwrap();
        executor.unregister("test").unwrap();

        let schedules = executor.list_schedules();
        assert_eq!(schedules.len(), 0);
    }

    #[test]
    fn test_enable_disable() {
        let executor = ScheduleExecutor::new();
        let schedule = Schedule::new("test", "0 0 * * *", vec![]);

        executor.register(schedule).unwrap();

        executor.disable("test").unwrap();
        let schedule = executor.get_schedule("test").unwrap();
        assert!(!schedule.enabled);

        executor.enable("test").unwrap();
        let schedule = executor.get_schedule("test").unwrap();
        assert!(schedule.enabled);
    }
}
