---
layout: default
title: Schema Versioning
parent: English
nav_order: 35
---

# Schema Versioning
{: .no_toc }

Change table schemas with zero downtime and roll back to any previous version.
{: .fs-6 .fw-300 }

---

## Overview

Altering schemas on a live database is risky. DBX's schema versioning keeps a full change history, ensuring safe DDL operations with rollback capability.

---

## Basic Usage

```rust
use dbx_core::engine::schema_versioning::SchemaVersionManager;

let manager = SchemaVersionManager::new();

// Register table (v1)
let v = manager.register_table("users", schema_v1)?;
assert_eq!(v, 1);

// Add column (v2)
let v = manager.alter_table("users", schema_v2, "Add email column")?;
assert_eq!(v, 2);

// Get current schema — O(1) performance (DashMap cache)
let schema = manager.get_current("users")?;
```

---

## Version Management

```rust
// Check current version
let version = manager.current_version("users")?;  // → 2

// Get schema at a specific version
let old_schema = manager.get_at_version("users", 1)?;

// Roll back to a previous version
manager.rollback("users", 1)?;
let version = manager.current_version("users")?;  // → 1
```

---

## Version History

```rust
let history = manager.history("users")?;
for entry in &history {
    println!("v{}: {} ({})", entry.version, entry.description, entry.created_at);
}
// Output:
// v1: Initial schema (2026-02-15 11:00)
// v2: Add email column (2026-02-15 14:30)
```

---

## Performance

| Operation | Time | Note |
|-----------|------|------|
| `get_current` | **46 ns** | Direct DashMap cache lookup |
| `alter_table` | **746 ns** | Add new version |
| 8-thread concurrent reads | **18.1M ops/s** | 2.44x faster than RwLock |

---

## Next Steps

- [Indexing Guide](indexing) — Index versioning integration
- [Transactions Guide](transactions) — DDL and transaction interaction
