//! Schedule types and definitions
//!
//! 스케줄 타입 정의

use std::time::Duration;

/// 스케줄 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleType {
    /// 한 번만 실행 (지연 시간)
    Once(Duration),
    
    /// 주기적 실행 (간격)
    Interval(Duration),
    
    /// 매일 특정 시간 (시, 분)
    Daily { hour: u8, minute: u8 },
    
    /// 매주 특정 요일 및 시간 (0=일요일, 6=토요일)
    Weekly { day: u8, hour: u8, minute: u8 },
    
    /// Cron 표현식: `분 시 일 월 요일`
    Cron(String),
}

/// 파싱된 Cron 필드
#[derive(Debug, Clone)]
enum CronField {
    /// 모든 값 (`*`)
    Any,
    /// 특정 값
    Value(u32),
    /// 스텝 (`*/N`)
    Step(u32),
    /// 범위 (`A-B`)
    Range(u32, u32),
}

impl CronField {
    /// 문자열에서 파싱
    fn parse(s: &str) -> Option<Self> {
        if s == "*" {
            return Some(CronField::Any);
        }
        if let Some(step) = s.strip_prefix("*/") {
            return step.parse::<u32>().ok().map(CronField::Step);
        }
        if let Some((a, b)) = s.split_once('-') {
            let start = a.parse::<u32>().ok()?;
            let end = b.parse::<u32>().ok()?;
            return Some(CronField::Range(start, end));
        }
        s.parse::<u32>().ok().map(CronField::Value)
    }
    
    /// 현재 값이 매칭하는지 확인
    fn matches(&self, current: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Value(v) => current == *v,
            CronField::Step(step) => *step > 0 && current % *step == 0,
            CronField::Range(start, end) => current >= *start && current <= *end,
        }
    }
    
    /// 현재 이후 가장 가까운 매칭 값 반환 (wrap_at 이상이면 순환)
    fn next_match(&self, current: u32, wrap_at: u32) -> u32 {
        for candidate in current..wrap_at {
            if self.matches(candidate) {
                return candidate;
            }
        }
        // 다음 주기에서 탐색
        for candidate in 0..wrap_at {
            if self.matches(candidate) {
                return candidate;
            }
        }
        current
    }
}

/// 파싱된 Cron 표현식
#[derive(Debug, Clone)]
struct CronExpr {
    minute: CronField,
    hour: CronField,
    day: CronField,
    month: CronField,
    weekday: CronField,
}

impl CronExpr {
    /// Cron 문자열 파싱: `"분 시 일 월 요일"`
    fn parse(expr: &str) -> Option<Self> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return None;
        }
        Some(Self {
            minute: CronField::parse(parts[0])?,
            hour: CronField::parse(parts[1])?,
            day: CronField::parse(parts[2])?,
            month: CronField::parse(parts[3])?,
            weekday: CronField::parse(parts[4])?,
        })
    }
    
    /// 현재 시각(Unix) 이후 다음 실행 시간 계산
    fn next_run_after(&self, now_secs: u64) -> u64 {
        // 현재 시간 분해 (UTC 기준 간단 구현)
        let secs_per_min = 60u64;
        let secs_per_hour = 3600u64;
        let secs_per_day = 86400u64;
        
        // 1분 후부터 탐색 (현재 시각은 제외)
        let start = now_secs + 60 - (now_secs % 60);
        
        // 최대 366일 탐색
        let max_iterations = 366 * 24 * 60;
        let mut candidate = start;
        
        for _ in 0..max_iterations {
            let total_minutes = candidate / secs_per_min;
            let minute = (total_minutes % 60) as u32;
            let total_hours = candidate / secs_per_hour;
            let hour = (total_hours % 24) as u32;
            
            // 간단한 일/월/요일 계산 (Unix epoch = 1970-01-01 목요일)
            let days_since_epoch = (candidate / secs_per_day) as u32;
            let weekday = (days_since_epoch + 4) % 7; // 0=일, 4=목 (1970-01-01)
            let (_, month, day) = Self::days_to_ymd(days_since_epoch);
            
            if self.minute.matches(minute)
                && self.hour.matches(hour)
                && self.day.matches(day)
                && self.month.matches(month)
                && self.weekday.matches(weekday)
            {
                return candidate;
            }
            
            candidate += secs_per_min; // 1분씩 전진
        }
        
        // fallback: 1시간 후
        now_secs + secs_per_hour
    }
    
    /// 에포크 이후 일수 → (년, 월, 일) 변환
    fn days_to_ymd(days: u32) -> (u32, u32, u32) {
        // 간소화된 그레고리력 변환
        let mut y = 1970u32;
        let mut remaining = days;
        
        loop {
            let days_in_year = if Self::is_leap(y) { 366 } else { 365 };
            if remaining < days_in_year {
                break;
            }
            remaining -= days_in_year;
            y += 1;
        }
        
        let leap = Self::is_leap(y);
        let month_days: [u32; 12] = [
            31, if leap { 29 } else { 28 }, 31, 30, 31, 30,
            31, 31, 30, 31, 30, 31,
        ];
        
        let mut m = 1u32;
        for &md in &month_days {
            if remaining < md {
                break;
            }
            remaining -= md;
            m += 1;
        }
        
        (y, m, remaining + 1)
    }
    
    fn is_leap(y: u32) -> bool {
        (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
    }
}

/// 스케줄
#[derive(Debug, Clone)]
pub struct Schedule {
    /// 스케줄 타입
    pub schedule_type: ScheduleType,
    
    /// 다음 실행 시간 (Unix timestamp)
    pub next_run: u64,
}

impl Schedule {
    /// 새 스케줄 생성
    pub fn new(schedule_type: ScheduleType) -> Self {
        let next_run = Self::calculate_next_run(&schedule_type);
        Self {
            schedule_type,
            next_run,
        }
    }
    
    /// 다음 실행 시간 계산
    fn calculate_next_run(schedule_type: &ScheduleType) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        match schedule_type {
            ScheduleType::Once(delay) => now + delay.as_secs(),
            ScheduleType::Interval(interval) => now + interval.as_secs(),
            ScheduleType::Daily { hour, minute } => {
                let seconds_in_day = 24 * 60 * 60;
                let target_seconds = (*hour as u64) * 3600 + (*minute as u64) * 60;
                let current_seconds = now % seconds_in_day;
                
                if current_seconds < target_seconds {
                    now + (target_seconds - current_seconds)
                } else {
                    now + (seconds_in_day - current_seconds + target_seconds)
                }
            }
            ScheduleType::Weekly { day: _, hour, minute } => {
                let seconds_in_week = 7 * 24 * 60 * 60;
                let target_seconds = (*hour as u64) * 3600 + (*minute as u64) * 60;
                now + seconds_in_week + target_seconds
            }
            ScheduleType::Cron(expr) => {
                if let Some(cron) = CronExpr::parse(expr) {
                    cron.next_run_after(now)
                } else {
                    // 파싱 실패 시 1시간 후
                    now + 3600
                }
            }
        }
    }
    
    /// 스케줄 업데이트 (다음 실행 시간 재계산)
    pub fn update(&mut self) {
        self.next_run = match &self.schedule_type {
            ScheduleType::Once(_) => {
                self.next_run
            }
            ScheduleType::Interval(interval) => {
                self.next_run + interval.as_secs()
            }
            _ => Self::calculate_next_run(&self.schedule_type),
        };
    }
    
    /// 실행 준비 여부 확인
    pub fn is_ready(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now >= self.next_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schedule_once() {
        let schedule = Schedule::new(ScheduleType::Once(Duration::from_secs(10)));
        assert!(!schedule.is_ready());
    }
    
    #[test]
    fn test_schedule_interval() {
        let mut schedule = Schedule::new(ScheduleType::Interval(Duration::from_secs(60)));
        let first_run = schedule.next_run;
        schedule.update();
        assert_eq!(schedule.next_run, first_run + 60);
    }
    
    #[test]
    fn test_schedule_daily() {
        let schedule = Schedule::new(ScheduleType::Daily { hour: 9, minute: 0 });
        assert!(schedule.next_run > 0);
    }
    
    #[test]
    fn test_cron_field_parse() {
        assert!(matches!(CronField::parse("*"), Some(CronField::Any)));
        assert!(matches!(CronField::parse("*/5"), Some(CronField::Step(5))));
        assert!(matches!(CronField::parse("30"), Some(CronField::Value(30))));
        assert!(matches!(CronField::parse("1-5"), Some(CronField::Range(1, 5))));
    }
    
    #[test]
    fn test_cron_field_matches() {
        assert!(CronField::Any.matches(42));
        assert!(CronField::Value(5).matches(5));
        assert!(!CronField::Value(5).matches(6));
        assert!(CronField::Step(5).matches(0));
        assert!(CronField::Step(5).matches(10));
        assert!(!CronField::Step(5).matches(3));
        assert!(CronField::Range(1, 5).matches(3));
        assert!(!CronField::Range(1, 5).matches(6));
    }
    
    #[test]
    fn test_cron_expr_parse() {
        let cron = CronExpr::parse("*/5 * * * *");
        assert!(cron.is_some());
        
        let bad = CronExpr::parse("invalid");
        assert!(bad.is_none());
    }
    
    #[test]
    fn test_cron_next_run() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 매분 실행
        let cron = CronExpr::parse("* * * * *").unwrap();
        let next = cron.next_run_after(now);
        assert!(next > now);
        assert!(next <= now + 60);
        
        // 매시 정각
        let cron = CronExpr::parse("0 * * * *").unwrap();
        let next = cron.next_run_after(now);
        assert!(next > now);
        assert!(next <= now + 3600);
    }
    
    #[test]
    fn test_cron_schedule_integration() {
        let schedule = Schedule::new(ScheduleType::Cron("*/5 * * * *".to_string()));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        assert!(schedule.next_run > now);
        // 5분 이내 (최대 5분 대기)
        assert!(schedule.next_run <= now + 300);
    }
    
    #[test]
    fn test_days_to_ymd() {
        // 1970-01-01
        assert_eq!(CronExpr::days_to_ymd(0), (1970, 1, 1));
        // 2000-01-01 = 10957일
        assert_eq!(CronExpr::days_to_ymd(10957), (2000, 1, 1));
    }
}
