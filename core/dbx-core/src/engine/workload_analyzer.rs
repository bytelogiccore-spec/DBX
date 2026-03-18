//! 적응형 워크로드 분석기
//!
//! 최근 쿼리 N개 중 OLTP(포인트 쿼리) vs OLAP(풀스캔/집계) 비율을 추적하고
//! 최적 설정을 추천합니다.
//!
//! # 동작 원리
//!
//! 1. `record(pattern)` : 각 쿼리 실행 후 패턴 기록
//! 2. `oltp_ratio()` : 최근 window 내 OLTP 비율 반환 (0.0~1.0)
//! 3. `recommended_config()` : OLTP/OLAP 비율에 따른 추천 설정 반환

use std::collections::VecDeque;

/// 쿼리 패턴 분류
#[derive(Debug, Clone, PartialEq)]
pub enum QueryPattern {
    /// OLTP: 단일 키 조회, INSERT, UPDATE
    PointQuery,
    /// OLAP: 범위 스캔
    RangeScan,
    /// OLAP: 집계 (COUNT, SUM, AVG 등)
    Aggregation,
    /// OLAP: 조인
    Join,
}

impl QueryPattern {
    /// OLTP 패턴 여부
    pub fn is_oltp(&self) -> bool {
        matches!(self, QueryPattern::PointQuery)
    }
}

/// 적응형 워크로드 설정
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveConfig {
    /// Delta 최대 크기 (행 수). OLTP에서 작게, OLAP에서 크게.
    pub delta_max_rows: usize,
    /// Columnar Cache 최대 크기 (MB). OLAP에서 크게.
    pub cache_max_mb: usize,
    /// Compaction 주기 (초). OLAP에서 자주.
    pub compaction_interval_secs: u64,
}

impl AdaptiveConfig {
    /// OLTP 중심 (포인트 쿼리 70%+)
    pub fn oltp_heavy() -> Self {
        Self {
            delta_max_rows: 1_000,      // 작은 delta → 빠른 쓰기
            cache_max_mb: 64,           // 캐시 최소화
            compaction_interval_secs: 3600, // 드물게 compaction
        }
    }

    /// OLAP 중심 (포인트 쿼리 30%-)
    pub fn olap_heavy() -> Self {
        Self {
            delta_max_rows: 100_000,    // 큰 delta → 배치 쓰기 효율
            cache_max_mb: 1_024,        // 큰 캐시 → 쿼리 가속
            compaction_interval_secs: 300, // 자주 compaction
        }
    }

    /// 균형 (30~70%)
    pub fn balanced() -> Self {
        Self {
            delta_max_rows: 10_000,
            cache_max_mb: 256,
            compaction_interval_secs: 900,
        }
    }
}

/// 적응형 워크로드 분석기
///
/// 슬라이딩 윈도우로 최근 쿼리 패턴을 추적합니다.
pub struct WorkloadAnalyzer {
    window: VecDeque<QueryPattern>,
    window_size: usize,
}

impl WorkloadAnalyzer {
    /// 새 분석기 생성. window_size는 추적할 최근 쿼리 수.
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
        }
    }

    /// 기본 윈도우(1000)로 분석기 생성
    pub fn default_window() -> Self {
        Self::new(1_000)
    }

    /// 쿼리 패턴 기록
    pub fn record(&mut self, pattern: QueryPattern) {
        if self.window.len() >= self.window_size {
            self.window.pop_front();
        }
        self.window.push_back(pattern);
    }

    /// 0.0 = 순수 OLAP, 1.0 = 순수 OLTP
    /// 윈도우가 비어있으면 0.5 (중립) 반환
    pub fn oltp_ratio(&self) -> f64 {
        if self.window.is_empty() {
            return 0.5;
        }
        let oltp_count = self.window.iter().filter(|p| p.is_oltp()).count();
        oltp_count as f64 / self.window.len() as f64
    }

    /// 현재 워크로드에 맞는 추천 설정 반환
    pub fn recommended_config(&self) -> AdaptiveConfig {
        let ratio = self.oltp_ratio();
        if ratio > 0.7 {
            AdaptiveConfig::oltp_heavy()
        } else if ratio < 0.3 {
            AdaptiveConfig::olap_heavy()
        } else {
            AdaptiveConfig::balanced()
        }
    }

    /// 현재 윈도우 내 쿼리 수
    pub fn window_len(&self) -> usize {
        self.window.len()
    }

    /// 분석기 초기화 (윈도우 비우기)
    pub fn reset(&mut self) {
        self.window.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oltp_ratio_empty() {
        let analyzer = WorkloadAnalyzer::new(100);
        assert!((analyzer.oltp_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_oltp_ratio_pure_oltp() {
        let mut analyzer = WorkloadAnalyzer::new(10);
        for _ in 0..10 {
            analyzer.record(QueryPattern::PointQuery);
        }
        assert!((analyzer.oltp_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_oltp_ratio_pure_olap() {
        let mut analyzer = WorkloadAnalyzer::new(10);
        for _ in 0..5 {
            analyzer.record(QueryPattern::Aggregation);
            analyzer.record(QueryPattern::RangeScan);
        }
        assert!((analyzer.oltp_ratio() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_recommended_config_oltp_heavy() {
        let mut analyzer = WorkloadAnalyzer::new(100);
        for _ in 0..80 {
            analyzer.record(QueryPattern::PointQuery);
        }
        for _ in 0..20 {
            analyzer.record(QueryPattern::RangeScan);
        }
        let config = analyzer.recommended_config();
        assert_eq!(config, AdaptiveConfig::oltp_heavy());
    }

    #[test]
    fn test_recommended_config_olap_heavy() {
        let mut analyzer = WorkloadAnalyzer::new(100);
        for _ in 0..10 {
            analyzer.record(QueryPattern::PointQuery);
        }
        for _ in 0..90 {
            analyzer.record(QueryPattern::Aggregation);
        }
        let config = analyzer.recommended_config();
        assert_eq!(config, AdaptiveConfig::olap_heavy());
    }

    #[test]
    fn test_recommended_config_balanced() {
        let mut analyzer = WorkloadAnalyzer::new(100);
        for _ in 0..50 {
            analyzer.record(QueryPattern::PointQuery);
        }
        for _ in 0..50 {
            analyzer.record(QueryPattern::Join);
        }
        let config = analyzer.recommended_config();
        assert_eq!(config, AdaptiveConfig::balanced());
    }

    #[test]
    fn test_sliding_window_eviction() {
        let mut analyzer = WorkloadAnalyzer::new(5);
        // 5개 OLAP 채우기
        for _ in 0..5 {
            analyzer.record(QueryPattern::Aggregation);
        }
        assert!((analyzer.oltp_ratio() - 0.0).abs() < f64::EPSILON);

        // OLTP 3개 추가 → 오래된 OLAP 3개 퇴출
        for _ in 0..3 {
            analyzer.record(QueryPattern::PointQuery);
        }
        // 윈도우: [OLAP, OLAP, OLTP, OLTP, OLTP]
        assert!((analyzer.oltp_ratio() - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_reset() {
        let mut analyzer = WorkloadAnalyzer::new(10);
        for _ in 0..5 {
            analyzer.record(QueryPattern::PointQuery);
        }
        assert_eq!(analyzer.window_len(), 5);
        analyzer.reset();
        assert_eq!(analyzer.window_len(), 0);
        assert!((analyzer.oltp_ratio() - 0.5).abs() < f64::EPSILON);
    }
}
