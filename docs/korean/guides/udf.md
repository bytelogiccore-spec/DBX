---
layout: default
title: 사용자 정의 함수 (UDF)
parent: 한국어
nav_order: 30
---

# 사용자 정의 함수 (UDF)
{: .no_toc }

커스텀 비즈니스 로직을 SQL 쿼리 안에서 직접 실행할 수 있습니다.
{: .fs-6 .fw-300 }

---

## 개요

DBX의 UDF 프레임워크는 세 가지 유형의 사용자 정의 함수를 지원합니다:

| 유형 | 설명 | 예시 |
|------|------|------|
| **스칼라** | 행 하나를 받아 값 하나를 반환 | `UPPER(name)`, `HASH(key)` |
| **집계** | 여러 행을 받아 값 하나를 반환 | `SUM(price)`, `MEDIAN(score)` |
| **테이블** | 입력을 받아 테이블(행 집합)을 반환 | `GENERATE_SERIES(1, 100)` |

---

## 스칼라 UDF

행 단위로 실행되는 가장 기본적인 UDF입니다.

```rust
use dbx_core::automation::udf::{ScalarUdf, UdfRegistry};
use arrow::array::{ArrayRef, StringArray};
use std::sync::Arc;

// 문자열을 대문자로 변환하는 UDF
let upper_fn = ScalarUdf::new(
    "my_upper",
    vec![DataType::Utf8],  // 입력 타입
    DataType::Utf8,         // 출력 타입
    |args: &[ArrayRef]| {
        let input = args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let result: StringArray = input.iter()
            .map(|v| v.map(|s| s.to_uppercase()))
            .collect();
        Ok(Arc::new(result) as ArrayRef)
    },
);

// 레지스트리에 등록
let mut registry = UdfRegistry::new();
registry.register_scalar(upper_fn);
```

---

## 집계 UDF

여러 행을 하나의 결과로 축소합니다.

```rust
use dbx_core::automation::udf::AggregateUdf;

// 중앙값(Median) 계산 UDF
let median_fn = AggregateUdf::new(
    "median",
    DataType::Float64,
    DataType::Float64,
    || Vec::new(),                          // 초기 상태
    |state: &mut Vec<f64>, value: f64| {    // 축적
        state.push(value);
    },
    |state: &Vec<f64>| -> f64 {             // 최종 계산
        let mut sorted = state.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[sorted.len() / 2]
    },
);
```

---

## 테이블 UDF

행 집합을 반환하는 함수로, FROM 절에서 사용합니다.

```rust
use dbx_core::automation::udf::TableUdf;

// 숫자 시퀀스를 생성하는 테이블 UDF
let generate_series = TableUdf::new(
    "generate_series",
    vec![DataType::Int64, DataType::Int64],  // start, end
    Schema::new(vec![Field::new("value", DataType::Int64, false)]),
    |args| {
        let start = args[0];
        let end = args[1];
        // start부터 end까지의 RecordBatch를 생성하여 반환
        Ok(vec![batch])
    },
);
```

---

## 다음 단계

- [트리거 가이드](triggers) — 데이터 변경 시 자동 실행되는 로직
- [SQL 레퍼런스](sql-reference) — UDF를 SQL에서 활용하는 방법
