//! Hashing utilities for Arrow batches — Provides vectorized hashing without per-row allocations.

use crate::error::DbxResult;
use ahash::RandomState;
use arrow::array::*;
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;

/// 배치의 지정된 컬럼들에 대해 행 단위 해시값을 계산하여 UInt64Array로 반환합니다.
/// Vec<u8> 할당 없이 필드 값을 직접 해싱하여 속도를 극대화합니다.
pub fn hash_batch(batch: &RecordBatch, columns: &[usize], seed: u64) -> DbxResult<UInt64Array> {
    let num_rows = batch.num_rows();
    if num_rows == 0 {
        return Ok(UInt64Array::from(Vec::<u64>::new()));
    }

    let hasher_state = RandomState::with_seeds(seed, seed, seed, seed);
    let mut hashes = vec![seed; num_rows];

    for &col_idx in columns {
        let col = batch.column(col_idx);
        update_hashes(col, &mut hashes, &hasher_state)?;
    }

    Ok(UInt64Array::from(hashes))
}

/// 기존 해시값 배열에 새로운 컬럼의 해시값을 누적합니다.
fn update_hashes(col: &ArrayRef, hashes: &mut [u64], hasher: &RandomState) -> DbxResult<()> {
    let num_rows = col.len();

    match col.data_type() {
        DataType::Int32 => {
            let arr = col.as_any().downcast_ref::<Int32Array>().unwrap();
            for i in 0..num_rows {
                if !arr.is_null(i) {
                    let val_hash = hasher.hash_one(arr.value(i));
                    hashes[i] = combine_hashes(hashes[i], val_hash);
                } else {
                    hashes[i] = combine_hashes(hashes[i], 0);
                }
            }
        }
        DataType::Int64 => {
            let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
            for i in 0..num_rows {
                if !arr.is_null(i) {
                    let val_hash = hasher.hash_one(arr.value(i));
                    hashes[i] = combine_hashes(hashes[i], val_hash);
                } else {
                    hashes[i] = combine_hashes(hashes[i], 0);
                }
            }
        }
        DataType::Float64 => {
            let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
            for i in 0..num_rows {
                if !arr.is_null(i) {
                    let val_hash = hasher.hash_one(arr.value(i).to_bits());
                    hashes[i] = combine_hashes(hashes[i], val_hash);
                } else {
                    hashes[i] = combine_hashes(hashes[i], 0);
                }
            }
        }
        DataType::Utf8 => {
            let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
            for i in 0..num_rows {
                if !arr.is_null(i) {
                    let val_hash = hasher.hash_one(arr.value(i));
                    hashes[i] = combine_hashes(hashes[i], val_hash);
                } else {
                    hashes[i] = combine_hashes(hashes[i], 0);
                }
            }
        }
        _ => {
            // 다른 타입들은 포맷팅하여 해싱 (성능 저하 지점, 필요 시 확장)
            for i in 0..num_rows {
                let val_hash = hasher.hash_one(format!("{:?}", col.as_any()));
                hashes[i] = combine_hashes(hashes[i], val_hash);
            }
        }
    }
    Ok(())
}

/// 두 해시값을 결합합니다 (Boost hash_combine 스타일).
#[inline]
fn combine_hashes(h1: u64, h2: u64) -> u64 {
    // 0x9e3779b97f4a7c15는 골든 레이티오의 u64 버전
    h1 ^ (h2
        .wrapping_add(0x9e3779b97f4a7c15)
        .wrapping_add(h1 << 6)
        .wrapping_add(h1 >> 2))
}
