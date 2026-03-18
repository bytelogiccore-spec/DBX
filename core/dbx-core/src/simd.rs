//! SIMD vectorized aggregates - Stable Rust implementation via `wide` crate.
//!
//! P7: nightly `std::simd` 에서 stable `wide` crate으로 전환.
//! feature flag 없이 기본 빌드에서 항상 SIMD 가속이 활성화됩니다.

use wide::f64x4;

// ──────────────────────────────────────────────────────────────────

/// SIMD 가속 f64 배열 합계 (4-lane wide::f64x4 사용).
pub fn sum_f64(values: &[f64]) -> f64 {
    if values.len() < 4 {
        return values.iter().sum();
    }
    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();
    let mut acc = f64x4::splat(0.0);
    for chunk in chunks {
        acc += f64x4::from([chunk[0], chunk[1], chunk[2], chunk[3]]);
    }
    let lanes: [f64; 4] = acc.into();
    let mut sum = lanes[0] + lanes[1] + lanes[2] + lanes[3];
    sum += remainder.iter().sum::<f64>();
    sum
}

/// SIMD 가속 f64 배열 평균.
pub fn avg_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(sum_f64(values) / values.len() as f64)
}

/// SIMD 가속 f64 배열 최소값 (min_by_component으로 4-lane 동시 비교).
pub fn min_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.len() < 4 {
        return values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();
    let mut acc = f64x4::splat(f64::INFINITY);
    for chunk in chunks {
        let v = f64x4::from([chunk[0], chunk[1], chunk[2], chunk[3]]);
        acc = acc.min_by_component(v);
    }
    let lanes: [f64; 4] = acc.into();
    let mut min = lanes[0].min(lanes[1]).min(lanes[2]).min(lanes[3]);
    for &v in remainder {
        if v < min {
            min = v;
        }
    }
    Some(min)
}

/// SIMD 가속 f64 배열 최대값.
pub fn max_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.len() < 4 {
        return values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let chunks = values.chunks_exact(4);
    let remainder = chunks.remainder();
    let mut acc = f64x4::splat(f64::NEG_INFINITY);
    for chunk in chunks {
        let v = f64x4::from([chunk[0], chunk[1], chunk[2], chunk[3]]);
        acc = acc.max_by_component(v);
    }
    let lanes: [f64; 4] = acc.into();
    let mut max = lanes[0].max(lanes[1]).max(lanes[2]).max(lanes[3]);
    for &v in remainder {
        if v > max {
            max = v;
        }
    }
    Some(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_f64() {
        assert_eq!(sum_f64(&[1.0, 2.0, 3.0, 4.0, 5.0]), 15.0);
    }

    #[test]
    fn test_sum_f64_large() {
        let values: Vec<f64> = (1..=1000).map(|i| i as f64).collect();
        let expected = (1..=1000_i64).sum::<i64>() as f64;
        assert!((sum_f64(&values) - expected).abs() < 1e-6);
    }

    #[test]
    fn test_avg_f64() {
        assert_eq!(avg_f64(&[1.0, 2.0, 3.0, 4.0, 5.0]), Some(3.0));
    }

    #[test]
    fn test_avg_f64_empty() {
        assert_eq!(avg_f64(&[]), None);
    }

    #[test]
    fn test_min_f64() {
        assert_eq!(min_f64(&[5.0, 2.0, 8.0, 1.0, 9.0]), Some(1.0));
    }

    #[test]
    fn test_max_f64() {
        assert_eq!(max_f64(&[5.0, 2.0, 8.0, 1.0, 9.0]), Some(9.0));
    }

    #[test]
    fn test_min_max_empty() {
        assert_eq!(min_f64(&[]), None);
        assert_eq!(max_f64(&[]), None);
    }

    #[test]
    fn test_simd_matches_scalar() {
        let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let scalar_sum: f64 = values.iter().sum();
        assert!((sum_f64(&values) - scalar_sum).abs() < 1e-9);
        let scalar_min = values.iter().copied().fold(f64::INFINITY, f64::min);
        assert_eq!(min_f64(&values), Some(scalar_min));
        let scalar_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert_eq!(max_f64(&values), Some(scalar_max));
    }
}
