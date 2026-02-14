//! SIMD 벡터화 프로토타입
//!
//! 간단한 집계 연산 (SUM, AVG, MIN, MAX)에 대한 SIMD 최적화

/// SIMD를 사용한 f64 배열 합계
///
/// # Safety
///
/// 이 함수는 안전합니다. 내부적으로 unsafe SIMD를 사용하지만,
/// 모든 경계 검사는 수행됩니다.
pub fn sum_f64(values: &[f64]) -> f64 {
    // 작은 배열은 스칼라로 처리
    if values.len() < 8 {
        return values.iter().sum();
    }

    // SIMD 경로 (nightly 필요)
    #[cfg(feature = "simd")]
    {
        sum_f64_simd(values)
    }

    // 폴백: 스칼라
    #[cfg(not(feature = "simd"))]
    {
        values.iter().sum()
    }
}

/// SIMD를 사용한 f64 배열 평균
pub fn avg_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(sum_f64(values) / values.len() as f64)
}

/// SIMD를 사용한 f64 배열 최소값
pub fn min_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    // 작은 배열은 스칼라로 처리
    if values.len() < 8 {
        return values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap());
    }

    // SIMD 경로
    #[cfg(feature = "simd")]
    {
        Some(min_f64_simd(values))
    }

    // 폴백: 스칼라
    #[cfg(not(feature = "simd"))]
    {
        values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }
}

/// SIMD를 사용한 f64 배열 최대값
pub fn max_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    // 작은 배열은 스칼라로 처리
    if values.len() < 8 {
        return values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap());
    }

    // SIMD 경로
    #[cfg(feature = "simd")]
    {
        Some(max_f64_simd(values))
    }

    // 폴백: 스칼라
    #[cfg(not(feature = "simd"))]
    {
        values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
    }
}

// SIMD 구현 (feature = "simd" 시에만 컴파일)
#[cfg(feature = "simd")]
fn sum_f64_simd(values: &[f64]) -> f64 {
    use std::simd::{f64x4, prelude::SimdFloat};

    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();

    // SIMD 합계
    let mut sum_vec = f64x4::splat(0.0);
    for chunk in chunks {
        let vec = f64x4::from_slice(chunk);
        sum_vec += vec;
    }

    // SIMD 결과 합산
    let mut sum = sum_vec.reduce_sum();

    // 나머지 스칼라 처리
    sum += remainder.iter().sum::<f64>();

    sum
}

#[cfg(feature = "simd")]
fn min_f64_simd(values: &[f64]) -> f64 {
    use std::simd::{f64x4, prelude::SimdFloat};

    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();

    // SIMD 최소값
    let mut min_vec = f64x4::splat(f64::INFINITY);
    for chunk in chunks {
        let vec = f64x4::from_slice(chunk);
        min_vec = min_vec.simd_min(vec);
    }

    // SIMD 결과 최소값
    let mut min = min_vec.reduce_min();

    // 나머지 스칼라 처리
    for &v in remainder {
        if v < min {
            min = v;
        }
    }

    min
}

#[cfg(feature = "simd")]
fn max_f64_simd(values: &[f64]) -> f64 {
    use std::simd::{f64x4, prelude::SimdFloat};

    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();

    // SIMD 최대값
    let mut max_vec = f64x4::splat(f64::NEG_INFINITY);
    for chunk in chunks {
        let vec = f64x4::from_slice(chunk);
        max_vec = max_vec.simd_max(vec);
    }

    // SIMD 결과 최대값
    let mut max = max_vec.reduce_max();

    // 나머지 스칼라 처리
    for &v in remainder {
        if v > max {
            max = v;
        }
    }

    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_f64() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(sum_f64(&values), 15.0);
    }

    #[test]
    fn test_sum_f64_large() {
        let values: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
        let expected: f64 = (1..=1000).sum();
        assert_eq!(sum_f64(&values), expected);
    }

    #[test]
    fn test_avg_f64() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(avg_f64(&values), Some(3.0));
    }

    #[test]
    fn test_avg_f64_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(avg_f64(&values), None);
    }

    #[test]
    fn test_min_f64() {
        let values = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        assert_eq!(min_f64(&values), Some(1.0));
    }

    #[test]
    fn test_max_f64() {
        let values = vec![5.0, 2.0, 8.0, 1.0, 9.0];
        assert_eq!(max_f64(&values), Some(9.0));
    }

    #[test]
    fn test_min_max_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(min_f64(&values), None);
        assert_eq!(max_f64(&values), None);
    }
}
