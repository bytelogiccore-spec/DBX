//! 재시도 정책 + Job 히스토리
//!
//! - `RetryPolicy`: 지수 백오프 지원 재시도
//! - `JobHistoryEntry`: 작업 실행 이력

use std::time::{Duration, SystemTime};

use crate::error::{DbxError, DbxResult};

/// 재시도 정책 설정
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 최대 재시도 횟수 (0 = 재시도 없음)
    pub max_retries: u32,
    /// 첫 번째 재시도 전 대기 시간 (ms)
    pub backoff_ms: u64,
    /// true면 지수 백오프, false면 고정 백오프
    pub exponential: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            backoff_ms: 100,
            exponential: true,
        }
    }
}

impl RetryPolicy {
    /// 새 재시도 정책 생성
    pub fn new(max_retries: u32, backoff_ms: u64, exponential: bool) -> Self {
        Self {
            max_retries,
            backoff_ms,
            exponential,
        }
    }

    /// 재시도 없는 정책
    pub fn no_retry() -> Self {
        Self {
            max_retries: 0,
            backoff_ms: 0,
            exponential: false,
        }
    }

    /// 주어진 클로저를 재시도 정책에 따라 실행.
    /// max_retries 초과 시 마지막 Err 반환.
    pub fn execute<F, T>(&self, mut f: F) -> DbxResult<T>
    where
        F: FnMut() -> DbxResult<T>,
    {
        let mut last_err = None;
        let mut delay = self.backoff_ms;

        for attempt in 0..=self.max_retries {
            match f() {
                Ok(v) => return Ok(v),
                Err(e) => {
                    last_err = Some(e);
                    if attempt < self.max_retries && delay > 0 {
                        std::thread::sleep(Duration::from_millis(delay));
                        if self.exponential {
                            delay = delay.saturating_mul(2);
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| DbxError::InvalidArguments("no error recorded".into())))
    }

    /// 최대 시도 횟수 반환 (initial attempt + retries)
    pub fn max_attempts(&self) -> u32 {
        self.max_retries + 1
    }
}

/// Job 실행 이력 항목
#[derive(Debug, Clone)]
pub struct JobHistoryEntry {
    /// 작업 식별자
    pub job_id: String,
    /// 실행 시작 시각
    pub started_at: SystemTime,
    /// 실행 완료 시각 (실행 중이면 None)
    pub completed_at: Option<SystemTime>,
    /// 성공 여부
    pub success: bool,
    /// 오류 메시지 (실패 시)
    pub error: Option<String>,
    /// 총 시도 횟수
    pub attempts: u32,
}

impl JobHistoryEntry {
    /// 새 항목 생성 (실행 시작)
    pub fn start(job_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            started_at: SystemTime::now(),
            completed_at: None,
            success: false,
            error: None,
            attempts: 0,
        }
    }

    /// 성공으로 완료
    pub fn complete_success(&mut self, attempts: u32) {
        self.completed_at = Some(SystemTime::now());
        self.success = true;
        self.attempts = attempts;
    }

    /// 실패로 완료
    pub fn complete_failure(&mut self, error: String, attempts: u32) {
        self.completed_at = Some(SystemTime::now());
        self.success = false;
        self.error = Some(error);
        self.attempts = attempts;
    }

    /// 실행 시간 (완료된 경우)
    pub fn duration(&self) -> Option<Duration> {
        self.completed_at
            .and_then(|end| end.duration_since(self.started_at).ok())
    }
}

/// Job 실행 이력 저장소 (인메모리)
#[derive(Debug, Default)]
pub struct JobHistory {
    entries: Vec<JobHistoryEntry>,
}

impl JobHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, entry: JobHistoryEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[JobHistoryEntry] {
        &self.entries
    }

    pub fn entries_for(&self, job_id: &str) -> Vec<&JobHistoryEntry> {
        self.entries.iter().filter(|e| e.job_id == job_id).collect()
    }

    pub fn last_for(&self, job_id: &str) -> Option<&JobHistoryEntry> {
        self.entries.iter().rev().find(|e| e.job_id == job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_retry_executes_until_success() {
        let policy = RetryPolicy {
            max_retries: 3,
            backoff_ms: 0, // 테스트 속도를 위해 대기 없음
            exponential: false,
        };
        let attempts = Arc::new(AtomicUsize::new(0));
        let a = attempts.clone();

        let result = policy.execute(move || {
            let count = a.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err(DbxError::Storage("fail".into()))
            } else {
                Ok(())
            }
        });

        assert!(result.is_ok(), "3번째 시도에서 성공해야 함");
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_retry_fails_after_max() {
        let policy = RetryPolicy {
            max_retries: 2,
            backoff_ms: 0,
            exponential: false,
        };
        let result = policy.execute::<_, ()>(|| Err(DbxError::Storage("always fail".into())));
        assert!(result.is_err(), "max_retries 초과 시 실패해야 함");
    }

    #[test]
    fn test_retry_no_retry_on_success() {
        let policy = RetryPolicy::no_retry();
        let attempts = Arc::new(AtomicUsize::new(0));
        let a = attempts.clone();

        let result = policy.execute(move || {
            a.fetch_add(1, Ordering::SeqCst);
            Ok::<_, DbxError>(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "재시도 없이 1번만 실행");
    }

    #[test]
    fn test_retry_max_attempts() {
        let policy = RetryPolicy::new(3, 0, false);
        assert_eq!(policy.max_attempts(), 4);
    }

    #[test]
    fn test_job_history_entry() {
        let mut entry = JobHistoryEntry::start("backup");
        assert_eq!(entry.job_id, "backup");
        assert!(!entry.success);

        entry.complete_success(1);
        assert!(entry.success);
        assert!(entry.completed_at.is_some());
        assert!(entry.duration().is_some());
    }

    #[test]
    fn test_job_history_store() {
        let mut history = JobHistory::new();

        let mut e1 = JobHistoryEntry::start("backup");
        e1.complete_success(1);
        history.push(e1);

        let mut e2 = JobHistoryEntry::start("backup");
        e2.complete_failure("disk full".into(), 3);
        history.push(e2);

        let backup_entries = history.entries_for("backup");
        assert_eq!(backup_entries.len(), 2);

        let last = history.last_for("backup").unwrap();
        assert!(!last.success);
        assert_eq!(last.attempts, 3);
    }
}
