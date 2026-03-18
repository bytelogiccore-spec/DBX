//! 파티셔닝 — Range / Hash / List 파티션 키 기반 물리적 분할
//!
//! 파티션된 테이블은 N개의 sub-테이블 (`table__p_part_0`, ...) 로 저장됩니다.
//! 라우팅 함수가 key 값 → sub-table 이름을 결정합니다.
//!
//! # 사용 예
//!
//! ```rust
//! use dbx_core::storage::partition::{PartitionMap, PartitionType, PartitionValue};
//!
//! let map = PartitionMap {
//!     table: "users".into(),
//!     partition_type: PartitionType::Hash {
//!         column: "id".into(),
//!         num_partitions: 4,
//!     },
//!     num_partitions: 4,
//! };
//!
//! let sub_table = map.route_key(&PartitionValue::Int(42));
//! assert!(sub_table.starts_with("users__p_part_"));
//! ```

/// 파티션 키 값 — i64 정수, 문자열, 부동소수점 지원
#[derive(Debug, Clone, PartialEq)]
pub enum PartitionValue {
    Int(i64),
    Float(f64),
    Text(String),
}

impl PartitionValue {
    fn as_i64(&self) -> i64 {
        match self {
            PartitionValue::Int(v) => *v,
            PartitionValue::Float(v) => *v as i64,
            PartitionValue::Text(s) => s.parse::<i64>().unwrap_or(0),
        }
    }

    fn to_string_repr(&self) -> String {
        match self {
            PartitionValue::Int(v) => v.to_string(),
            PartitionValue::Float(v) => v.to_string(),
            PartitionValue::Text(s) => s.clone(),
        }
    }
}

/// 파티션 타입
#[derive(Debug, Clone)]
pub enum PartitionType {
    /// 범위 파티션: 각 (low, high) 범위가 하나의 파티션 [low, high)
    Range {
        column: String,
        bounds: Vec<(i64, i64)>,
    },
    /// 해시 파티션: FNV1a 해시 후 num_partitions로 모듈러
    Hash {
        column: String,
        num_partitions: usize,
    },
    /// 리스트 파티션: 각 파티션이 특정 값 목록 소유
    List {
        column: String,
        values: Vec<Vec<String>>,
    },
}

/// 파티션 맵 — 테이블 하나의 파티션 구성 전체를 담음
#[derive(Debug, Clone)]
pub struct PartitionMap {
    /// 원본 테이블 이름
    pub table: String,
    /// 파티션 타입
    pub partition_type: PartitionType,
    /// 총 파티션 수
    pub num_partitions: usize,
}

impl PartitionMap {
    /// key 값을 받아 sub-table 이름 반환
    ///
    /// 반환 형식: `{table}__p_part_{idx}`
    pub fn route_key(&self, key_value: &PartitionValue) -> String {
        let idx = self.partition_index(key_value);
        format!("{}__p_part_{}", self.table, idx)
    }

    fn partition_index(&self, key_value: &PartitionValue) -> usize {
        match &self.partition_type {
            PartitionType::Hash { num_partitions, .. } => {
                let s = key_value.to_string_repr();
                let h = fnv1a_hash(s.as_bytes());
                h % num_partitions
            }
            PartitionType::Range { bounds, .. } => {
                let v = key_value.as_i64();
                bounds
                    .iter()
                    .position(|(lo, hi)| v >= *lo && v < *hi)
                    .unwrap_or(self.num_partitions.saturating_sub(1))
            }
            PartitionType::List { values, .. } => {
                let s = key_value.to_string_repr();
                values
                    .iter()
                    .position(|group| group.iter().any(|v| v == &s))
                    .unwrap_or(0)
            }
        }
    }

    /// WHERE 조건값으로 스캔할 파티션 목록 반환 (Pruning)
    ///
    /// - `filter_value = None` → 모든 파티션 반환 (full scan)
    /// - `filter_value = Some(v)` → 해당 v가 속한 파티션만 반환
    pub fn pruned_partitions(&self, filter_value: Option<&PartitionValue>) -> Vec<String> {
        match filter_value {
            None => (0..self.num_partitions)
                .map(|i| format!("{}__p_part_{}", self.table, i))
                .collect(),
            Some(v) => vec![self.route_key(v)],
        }
    }

    /// 모든 파티션의 sub-table 이름 반환
    pub fn all_partitions(&self) -> Vec<String> {
        self.pruned_partitions(None)
    }
}

/// FNV-1a 해시 (32-bit) — 결정론적, 가벼운 비암호학적 해시
fn fnv1a_hash(data: &[u8]) -> usize {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_partition_routing() {
        let map = PartitionMap {
            table: "users".into(),
            partition_type: PartitionType::Hash {
                column: "id".into(),
                num_partitions: 4,
            },
            num_partitions: 4,
        };
        let t0 = map.route_key(&PartitionValue::Int(0));
        let t3 = map.route_key(&PartitionValue::Int(3));
        assert!(t0.contains("part_"), "서브테이블 이름에 part_ 포함되어야 함");
        // 다른 값이 다른 파티션에 라우팅된다 (항상 참은 아니지만 0,3은 다름)
        // 적어도 테이블 이름이 올바른지 확인
        assert!(t0.starts_with("users__p_part_"));
        assert!(t3.starts_with("users__p_part_"));
    }

    #[test]
    fn test_hash_partition_all_valid_indices() {
        let n = 8usize;
        let map = PartitionMap {
            table: "logs".into(),
            partition_type: PartitionType::Hash {
                column: "id".into(),
                num_partitions: n,
            },
            num_partitions: n,
        };
        for i in 0i64..100 {
            let sub = map.route_key(&PartitionValue::Int(i));
            let idx: usize = sub.split('_').last().unwrap().parse().unwrap();
            assert!(idx < n, "인덱스 {}가 범위 이상", idx);
        }
    }

    #[test]
    fn test_range_partition_routing() {
        let map = PartitionMap {
            table: "orders".into(),
            partition_type: PartitionType::Range {
                column: "amount".into(),
                bounds: vec![(0, 100), (100, 500), (500, 10000)],
            },
            num_partitions: 3,
        };
        // 각 범위에 맞게 라우팅
        let p0 = map.route_key(&PartitionValue::Int(50));
        let p1 = map.route_key(&PartitionValue::Int(200));
        let p2 = map.route_key(&PartitionValue::Int(1000));
        assert!(p0.ends_with("_0"));
        assert!(p1.ends_with("_1"));
        assert!(p2.ends_with("_2"));
    }

    #[test]
    fn test_range_partition_pruning() {
        let map = PartitionMap {
            table: "logs".into(),
            partition_type: PartitionType::Range {
                column: "ts".into(),
                bounds: vec![(0, 1000), (1000, 2000), (2000, 3000)],
            },
            num_partitions: 3,
        };
        let partitions = map.pruned_partitions(Some(&PartitionValue::Int(1500)));
        assert_eq!(partitions.len(), 1, "단일 파티션만 스캔해야 함");
        assert!(partitions[0].ends_with("_1"), "1000-2000 범위는 파티션 1");

        // 필터 없으면 전체 스캔
        let all = map.pruned_partitions(None);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_list_partition_routing() {
        let map = PartitionMap {
            table: "regions".into(),
            partition_type: PartitionType::List {
                column: "country".into(),
                values: vec![
                    vec!["KR".into(), "JP".into()],
                    vec!["US".into(), "CA".into()],
                ],
            },
            num_partitions: 2,
        };
        let kr = map.route_key(&PartitionValue::Text("KR".into()));
        let us = map.route_key(&PartitionValue::Text("US".into()));
        assert!(kr.ends_with("_0"));
        assert!(us.ends_with("_1"));
    }

    #[test]
    fn test_all_partitions() {
        let map = PartitionMap {
            table: "data".into(),
            partition_type: PartitionType::Hash {
                column: "id".into(),
                num_partitions: 5,
            },
            num_partitions: 5,
        };
        let all = map.all_partitions();
        assert_eq!(all.len(), 5);
        for (i, name) in all.iter().enumerate() {
            assert_eq!(*name, format!("data__p_part_{}", i));
        }
    }

    #[test]
    fn test_fnv1a_hash_deterministic() {
        let h1 = fnv1a_hash(b"hello");
        let h2 = fnv1a_hash(b"hello");
        assert_eq!(h1, h2, "FNV1a는 결정론적이어야 함");

        let h3 = fnv1a_hash(b"world");
        assert_ne!(h1, h3, "다른 입력은 다른 해시");
    }
}
