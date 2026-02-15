---
layout: default
title: User-Defined Functions (UDF)
parent: English
nav_order: 30
---

# User-Defined Functions (UDF)
{: .no_toc }

Run custom business logic directly inside SQL queries.
{: .fs-6 .fw-300 }

---

## Overview

DBX's UDF framework supports three types of user-defined functions:

| Type | Description | Example |
|------|-------------|---------|
| **Scalar** | Takes one row, returns one value | `UPPER(name)`, `HASH(key)` |
| **Aggregate** | Takes many rows, returns one value | `SUM(price)`, `MEDIAN(score)` |
| **Table** | Takes input, returns a row set | `GENERATE_SERIES(1, 100)` |

---

## Scalar UDF

The most basic UDF, executed per row.

```rust
use dbx_core::automation::udf::{ScalarUdf, UdfRegistry};
use arrow::array::{ArrayRef, StringArray};
use std::sync::Arc;

// UDF that converts strings to uppercase
let upper_fn = ScalarUdf::new(
    "my_upper",
    vec![DataType::Utf8],   // input types
    DataType::Utf8,          // output type
    |args: &[ArrayRef]| {
        let input = args[0].as_any().downcast_ref::<StringArray>().unwrap();
        let result: StringArray = input.iter()
            .map(|v| v.map(|s| s.to_uppercase()))
            .collect();
        Ok(Arc::new(result) as ArrayRef)
    },
);

let mut registry = UdfRegistry::new();
registry.register_scalar(upper_fn);
```

---

## Aggregate UDF

Reduces multiple rows into a single result.

```rust
use dbx_core::automation::udf::AggregateUdf;

// Median calculation UDF
let median_fn = AggregateUdf::new(
    "median",
    DataType::Float64,
    DataType::Float64,
    || Vec::new(),                          // initial state
    |state: &mut Vec<f64>, value: f64| {    // accumulate
        state.push(value);
    },
    |state: &Vec<f64>| -> f64 {             // finalize
        let mut sorted = state.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[sorted.len() / 2]
    },
);
```

---

## Table UDF

Returns a row set, used in the FROM clause.

```rust
use dbx_core::automation::udf::TableUdf;

// Table UDF that generates a number sequence
let generate_series = TableUdf::new(
    "generate_series",
    vec![DataType::Int64, DataType::Int64],
    Schema::new(vec![Field::new("value", DataType::Int64, false)]),
    |args| {
        let start = args[0];
        let end = args[1];
        // Build and return RecordBatch from start to end
        Ok(vec![batch])
    },
);
```

---

## Next Steps

- [Triggers Guide](triggers) — Auto-execute logic on data changes
- [SQL Reference](sql-reference) — Using UDFs in SQL
