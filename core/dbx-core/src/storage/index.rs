//! Bloom Filter Index — Tier 4 인덱스
//!
//! Parquet 파일별 Bloom Filter를 생성하여 I/O를 최소화
//! 간단한 비트맵 기반 구현

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Bloom Filter Index
pub struct BloomIndex {
    bitmap: Vec<u64>,
    num_hashes: usize,
    items_count: usize,
    false_positive_rate: f64,
}

impl BloomIndex {
    /// 새로운 Bloom Filter 생성
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let bitmap_bits = Self::optimal_bitmap_size(expected_items, false_positive_rate);
        let num_hashes = Self::optimal_num_hashes(expected_items, bitmap_bits);
        let bitmap_size = bitmap_bits.div_ceil(64); // Round up to u64

        Self {
            bitmap: vec![0u64; bitmap_size],
            num_hashes,
            items_count: 0,
            false_positive_rate,
        }
    }

    /// 기본 설정으로 생성 (1% 오탐률)
    pub fn with_default_fpr(expected_items: usize) -> Self {
        Self::new(expected_items, 0.01)
    }

    /// 키 삽입
    pub fn insert(&mut self, key: &[u8]) {
        for i in 0..self.num_hashes {
            let hash = self.hash_with_seed(key, i);
            let bit_index = (hash as usize) % (self.bitmap.len() * 64);
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            self.bitmap[word_index] |= 1u64 << bit_offset;
        }
        self.items_count += 1;
    }

    /// 키가 존재할 가능성 확인
    pub fn may_contain(&self, key: &[u8]) -> bool {
        for i in 0..self.num_hashes {
            let hash = self.hash_with_seed(key, i);
            let bit_index = (hash as usize) % (self.bitmap.len() * 64);
            let word_index = bit_index / 64;
            let bit_offset = bit_index % 64;
            if (self.bitmap[word_index] & (1u64 << bit_offset)) == 0 {
                return false;
            }
        }
        true
    }

    /// 삽입된 아이템 개수
    pub fn len(&self) -> usize {
        self.items_count
    }

    /// 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.items_count == 0
    }

    /// Bloom Filter 통계
    pub fn stats(&self) -> BloomStats {
        BloomStats {
            items_count: self.items_count,
            bitmap_size: self.bitmap.len() * 64,
            num_hashes: self.num_hashes,
            target_fpr: self.false_positive_rate,
        }
    }

    /// 시드를 사용한 해싱
    fn hash_with_seed(&self, key: &[u8], seed: usize) -> u64 {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// 최적 비트맵 크기 계산
    fn optimal_bitmap_size(n: usize, p: f64) -> usize {
        let ln2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        let m = -(n as f64) * p.ln() / ln2_squared;
        m.ceil() as usize
    }

    /// 최적 해시 함수 개수 계산
    fn optimal_num_hashes(n: usize, m: usize) -> usize {
        let k = (m as f64 / n as f64) * std::f64::consts::LN_2;
        (k.ceil() as usize).max(1)
    }
}

/// Bloom Filter 통계
#[derive(Debug, Clone)]
pub struct BloomStats {
    pub items_count: usize,
    pub bitmap_size: usize,
    pub num_hashes: usize,
    pub target_fpr: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_basic() {
        let mut bloom = BloomIndex::with_default_fpr(1000);

        bloom.insert(b"key1");
        bloom.insert(b"key2");
        bloom.insert(b"key3");

        assert!(bloom.may_contain(b"key1"));
        assert!(bloom.may_contain(b"key2"));
        assert!(bloom.may_contain(b"key3"));
        assert!(!bloom.may_contain(b"key4"));
    }

    #[test]
    fn test_bloom_false_positive_rate() {
        let mut bloom = BloomIndex::new(1000, 0.01);

        for i in 0..1000 {
            bloom.insert(format!("key{}", i).as_bytes());
        }

        let mut false_positives = 0;
        for i in 1000..2000 {
            if bloom.may_contain(format!("key{}", i).as_bytes()) {
                false_positives += 1;
            }
        }

        let actual_fpr = false_positives as f64 / 1000.0;
        println!("Actual FPR: {:.4}", actual_fpr);

        assert!(actual_fpr < 0.05);
    }

    #[test]
    fn test_bloom_stats() {
        let mut bloom = BloomIndex::with_default_fpr(1000);

        for i in 0..100 {
            bloom.insert(format!("key{}", i).as_bytes());
        }

        let stats = bloom.stats();
        assert_eq!(stats.items_count, 100);
        assert!(stats.bitmap_size > 0);
        assert!(stats.num_hashes > 0);
    }

    #[test]
    fn test_bloom_empty() {
        let bloom = BloomIndex::with_default_fpr(1000);

        assert!(bloom.is_empty());
        assert_eq!(bloom.len(), 0);
        assert!(!bloom.may_contain(b"key1"));
    }
}
