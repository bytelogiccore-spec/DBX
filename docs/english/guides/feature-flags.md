---
layout: default
title: Feature Flags
parent: English
nav_order: 36
---

# Feature Flags
{: .no_toc }

Toggle individual features on and off at runtime without restarting.
{: .fs-6 .fw-300 }

---

## Overview

Feature flags let you enable or disable features at runtime. Useful for A/B testing, gradual rollouts, and emergency feature kills.

---

## Available Flags

| Flag | Description | Default |
|------|-------------|:-------:|
| `BinarySerialization` | Binary serialization | Off |
| `MultiThreading` | Multi-threaded execution | Off |
| `MvccExtension` | MVCC extended features | Off |
| `QueryPlanCache` | Query plan cache | Off |
| `ParallelQuery` | Parallel query execution | Off |
| `ParallelWal` | WAL parallel writes | Off |
| `ParallelCheckpoint` | Parallel checkpoint | Off |
| `SchemaVersioning` | Schema versioning | Off |
| `IndexVersioning` | Index versioning | Off |

---

## Usage

```rust
use dbx_core::engine::feature_flags::{FeatureFlags, Feature};

let flags = FeatureFlags::new();

// Enable/disable features
flags.enable(Feature::ParallelQuery);
flags.disable(Feature::ParallelQuery);
flags.toggle(Feature::QueryPlanCache);

// Check status
if flags.is_enabled(Feature::ParallelQuery) {
    // parallel query execution path
}
```

---

## Persistence

### File Save/Load

```rust
// Save to JSON file
flags.save_to_file("./dbx_features.json")?;

// Load from file
let flags = FeatureFlags::load_from_file("./dbx_features.json")?;
```

### Environment Variables

Control features via environment variables:

```bash
export DBX_FEATURE_PARALLEL_QUERY=true
export DBX_FEATURE_QUERY_PLAN_CACHE=true
```

```rust
let flags = FeatureFlags::load_from_env();
```

---

## Use Cases

| Scenario | Approach |
|----------|----------|
| Gradual feature rollout | `enable` on selected servers |
| Emergency performance fix | `disable` ParallelQuery |
| Per-environment config | Separate JSON files for dev/staging/prod |
| CI test isolation | Test feature combinations via env vars |

---

## Next Steps

- [Parallel Query Guide](parallel-query) — Detailed parallel query configuration
- [Plan Cache Guide](plan-cache) — Cache feature toggle
