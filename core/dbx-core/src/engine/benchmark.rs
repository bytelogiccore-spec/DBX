// Phase 0.1: 성능 벤치마크 프레임워크
//
// TDD 방식으로 구현:
// 1. Red: 테스트 작성 (실패)
// 2. Green: 최소 구현 (통과)
// 3. Refactor: 코드 개선

use crate::error::{DbxError, DbxResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// 벤치마크 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// 벤치마크 이름
    pub name: String,
    
    /// 평균 실행 시간 (밀리초)
    pub avg_time_ms: f64,
    
    /// 최소 실행 시간 (밀리초)
    pub min_time_ms: f64,
    
    /// 최대 실행 시간 (밀리초)
    pub max_time_ms: f64,
    
    /// 표준 편차
    pub std_dev_ms: f64,
    
    /// 샘플 수
    pub sample_count: usize,
    
    /// 타임스탬프
    pub timestamp: i64,
}

impl BenchmarkResult {
    /// 새 벤치마크 결과 생성
    pub fn new(name: String, samples: &[Duration]) -> Self {
        let sample_count = samples.len();
        
        // 밀리초로 변환
        let times_ms: Vec<f64> = samples
            .iter()
            .map(|d| d.as_secs_f64() * 1000.0)
            .collect();
        
        // 통계 계산
        let avg_time_ms = times_ms.iter().sum::<f64>() / sample_count as f64;
        let min_time_ms = times_ms.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_time_ms = times_ms.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        
        // 표준 편차
        let variance = times_ms
            .iter()
            .map(|t| {
                let diff = t - avg_time_ms;
                diff * diff
            })
            .sum::<f64>() / sample_count as f64;
        let std_dev_ms = variance.sqrt();
        
        Self {
            name,
            avg_time_ms,
            min_time_ms,
            max_time_ms,
            std_dev_ms,
            sample_count,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}

/// 성능 벤치마크 러너
pub struct BenchmarkRunner {
    /// 베이스라인 결과 (이름 → 결과)
    baseline: Arc<RwLock<HashMap<String, BenchmarkResult>>>,
    
    /// 성능 회귀 임계값 (예: 1.1 = 10% 저하 허용)
    threshold: f64,
    
    /// 베이스라인 파일 경로
    baseline_path: PathBuf,
    
    /// 샘플 수
    sample_count: usize,
}

impl BenchmarkRunner {
    /// 새 벤치마크 러너 생성
    pub fn new() -> Self {
        Self {
            baseline: Arc::new(RwLock::new(HashMap::new())),
            threshold: 1.1, // 10% 저하 허용
            baseline_path: PathBuf::from("target/benchmark_baseline.json"),
            sample_count: 100,
        }
    }
    
    /// 임계값 설정
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }
    
    /// 베이스라인 경로 설정
    pub fn with_baseline_path(mut self, path: PathBuf) -> Self {
        self.baseline_path = path;
        self
    }
    
    /// 샘플 수 설정
    pub fn with_sample_count(mut self, count: usize) -> Self {
        self.sample_count = count;
        self
    }
    
    /// 벤치마크 실행
    pub fn run<F>(&self, name: &str, mut f: F) -> DbxResult<BenchmarkResult>
    where
        F: FnMut(),
    {
        let mut samples = Vec::with_capacity(self.sample_count);
        
        // Warmup (5회)
        for _ in 0..5 {
            f();
        }
        
        // 실제 측정
        for _ in 0..self.sample_count {
            let start = Instant::now();
            f();
            let duration = start.elapsed();
            samples.push(duration);
        }
        
        Ok(BenchmarkResult::new(name.to_string(), &samples))
    }
    
    /// 베이스라인 저장
    pub fn save_baseline(&self) -> DbxResult<()> {
        let baseline = self.baseline.read().unwrap();
        let json = serde_json::to_string_pretty(&*baseline)?;
        
        // 디렉토리 생성
        if let Some(parent) = self.baseline_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.baseline_path, json)?;
        Ok(())
    }
    
    /// 베이스라인 로드
    pub fn load_baseline(&self) -> DbxResult<()> {
        if !self.baseline_path.exists() {
            return Ok(()); // 베이스라인 없으면 무시
        }
        
        let json = fs::read_to_string(&self.baseline_path)?;
        let loaded: HashMap<String, BenchmarkResult> = serde_json::from_str(&json)?;
        
        let mut baseline = self.baseline.write().unwrap();
        *baseline = loaded;
        
        Ok(())
    }
    
    /// 베이스라인 업데이트
    pub fn update_baseline(&self, name: &str, result: &BenchmarkResult) {
        self.baseline.write().unwrap().insert(name.to_string(), result.clone());
    }
    
    /// 성능 회귀 검사
    pub fn check_regression(&self, name: &str, result: &BenchmarkResult) -> DbxResult<()> {
        let baseline = self.baseline.read().unwrap();
        
        if let Some(baseline_result) = baseline.get(name) {
            let ratio = result.avg_time_ms / baseline_result.avg_time_ms;
            
            if ratio > self.threshold {
                return Err(DbxError::PerformanceRegression {
                    name: name.to_string(),
                    baseline: baseline_result.avg_time_ms,
                    current: result.avg_time_ms,
                    ratio,
                });
            }
        }
        
        Ok(())
    }
    
    /// 벤치마크 실행 및 회귀 검사
    pub fn run_and_check<F>(&self, name: &str, f: F) -> DbxResult<BenchmarkResult>
    where
        F: FnMut(),
    {
        let result = self.run(name, f)?;
        self.check_regression(name, &result)?;
        Ok(result)
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    // TDD: Red - 테스트 작성 (실패)
    
    #[test]
    fn test_benchmark_runner_basic() {
        let runner = BenchmarkRunner::new();
        
        let result = runner.run("test_sleep", || {
            thread::sleep(Duration::from_millis(1));
        }).unwrap();
        
        assert_eq!(result.name, "test_sleep");
        assert!(result.avg_time_ms >= 1.0);
        assert!(result.sample_count > 0);
    }
    
    #[test]
    fn test_baseline_save_load() {
        let temp_path = PathBuf::from("target/test_baseline.json");
        let runner = BenchmarkRunner::new()
            .with_baseline_path(temp_path.clone());
        
        // 벤치마크 실행
        let result = runner.run("test_op", || {
            let _ = 1 + 1;
        }).unwrap();
        
        // 베이스라인 업데이트 및 저장
        runner.update_baseline("test_op", &result);
        runner.save_baseline().unwrap();
        
        // 새 러너로 로드
        let runner2 = BenchmarkRunner::new()
            .with_baseline_path(temp_path.clone());
        runner2.load_baseline().unwrap();
        
        // 베이스라인 확인
        let baseline = runner2.baseline.read().unwrap();
        assert!(baseline.contains_key("test_op"));
        
        // 정리
        let _ = fs::remove_file(temp_path);
    }
    
    #[test]
    fn test_regression_detection() {
        let temp_path = PathBuf::from("target/test_regression.json");
        let runner = BenchmarkRunner::new()
            .with_baseline_path(temp_path.clone())
            .with_threshold(1.5); // 50% 저하 허용
        
        // 빠른 연산으로 베이스라인 생성
        let baseline_result = runner.run("fast_op", || {
            let _ = 1 + 1;
        }).unwrap();
        
        runner.update_baseline("fast_op", &baseline_result);
        
        // 느린 연산 (회귀)
        let slow_result = BenchmarkResult {
            name: "fast_op".to_string(),
            avg_time_ms: baseline_result.avg_time_ms * 2.0, // 2배 느림
            min_time_ms: 0.0,
            max_time_ms: 0.0,
            std_dev_ms: 0.0,
            sample_count: 100,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };
        
        // 회귀 검출 확인
        let result = runner.check_regression("fast_op", &slow_result);
        assert!(result.is_err());
        
        // 정리
        let _ = fs::remove_file(temp_path);
    }
    
    #[test]
    fn test_threshold_configuration() {
        let runner = BenchmarkRunner::new()
            .with_threshold(2.0); // 100% 저하 허용
        
        assert_eq!(runner.threshold, 2.0);
    }
    
    #[test]
    fn test_benchmark_comparison() {
        let runner = BenchmarkRunner::new();
        
        // 빠른 연산
        let fast = runner.run("fast", || {
            let _ = 1 + 1;
        }).unwrap();
        
        // 느린 연산
        let slow = runner.run("slow", || {
            thread::sleep(Duration::from_micros(10));
        }).unwrap();
        
        // 느린 연산이 더 오래 걸림
        assert!(slow.avg_time_ms > fast.avg_time_ms);
    }
}
