//! Background Scheduler Thread
//!
//! Weak 포인터를 사용하여 순환 참조 해결

use crate::automation::Schedule;
use crate::engine::Database;
use crate::error::DbxResult;
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, RwLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 백그라운드 스케줄러 스레드
pub struct SchedulerThread {
    /// 스레드 핸들
    handle: Option<JoinHandle<()>>,
    /// Shutdown 신호 전송 채널
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl SchedulerThread {
    /// 새 SchedulerThread 생성
    pub fn new() -> Self {
        Self {
            handle: None,
            shutdown_tx: None,
        }
    }

    /// 백그라운드 스케줄러 스레드 시작
    ///
    /// # 인자
    ///
    /// * `db_weak` - Database Weak 포인터 (순환 참조 방지)
    /// * `schedules` - Schedule 목록 (Arc 공유)
    pub fn start(
        &mut self,
        db_weak: Weak<Database>,
        schedules: Arc<RwLock<HashMap<String, Schedule>>>,
    ) -> DbxResult<()> {
        if self.handle.is_some() {
            return Ok(()); // 이미 실행 중
        }

        let (tx, rx) = mpsc::channel();
        self.shutdown_tx = Some(tx);

        let handle = thread::spawn(move || {
            loop {
                // Shutdown 신호 확인 (non-blocking)
                if rx.try_recv().is_ok() {
                    break;
                }

                // Database Arc 업그레이드
                let db = match db_weak.upgrade() {
                    Some(db) => db,
                    None => {
                        // Database가 drop되면 종료
                        break;
                    }
                };

                // 모든 스케줄 체크 및 실행
                let schedule_list: Vec<(String, Schedule)> = {
                    let sched = schedules.read().unwrap();
                    sched.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                };

                for (schedule_name, schedule) in schedule_list {
                    // enabled 상태 확인
                    if !schedule.enabled {
                        continue;
                    }

                    // Cron 표현식 파싱 및 다음 실행 시간 계산
                    use std::str::FromStr;
                    match cron::Schedule::from_str(&schedule.cron_expr) {
                        Ok(cron_schedule) => {
                            use chrono::Utc;
                            let now = Utc::now();

                            // 다음 실행 시간 계산
                            if let Some(next) = cron_schedule.upcoming(Utc).next() {
                                // 현재 시간과 비교 (1초 오차 허용)
                                let diff = (next - now).num_seconds();
                                if diff.abs() <= 1 {
                                    // 실행 시간 도달
                                    let exec = db.schedule_executor.read().unwrap();
                                    if let Err(e) = exec.execute(&db, &schedule_name) {
                                        eprintln!(
                                            "[Scheduler] Failed to execute schedule '{}': {}",
                                            schedule_name, e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "[Scheduler] Invalid cron expression for '{}': {}",
                                schedule_name, e
                            );
                        }
                    }
                }

                // 1초 대기
                thread::sleep(Duration::from_secs(1));
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// 백그라운드 스케줄러 스레드 중지
    pub fn stop(&mut self) -> DbxResult<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    /// 스레드가 실행 중인지 확인
    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }
}

impl Default for SchedulerThread {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SchedulerThread {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
