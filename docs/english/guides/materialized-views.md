---
layout: default
title: Materialized Views
parent: English
nav_order: 22
description: "Guide for pre-computed query results and automatic refresh"
---

# Materialized Views
{: .no_toc }

Materialized Views allow you to pre-compute the results of complex SQL queries and store them in a cache. This can significantly improve the performance of analytical queries on large datasets.
{: .fs-6 .fw-300 }

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Unlike standard Views, which reference the base tables every time a query is executed, **Materialized Views** store the results in a physical cache.

### Key Features
- **Query Performance Boost**: Returns the results of complex queries involving JOINs or aggregations instantly.
- **Auto-Refresh**: Automatically updates the results in the background every set number of seconds.
- **Transparent Caching**: When you execute a `SELECT` query, the DBX engine automatically uses the cache if a corresponding materialized view exists and is fresh.

---

## Creating Materialized Views

### Basic Syntax

```sql
CREATE MATERIALIZED VIEW [view_name] 
[REFRESH EVERY [seconds]] 
AS [select_query]
```

### Example: Sales summary with 60s auto-refresh

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create a view that automatically refreshes sales statistics every 60 seconds
    db.execute_sql(
        "CREATE MATERIALIZED VIEW sales_summary 
         REFRESH EVERY 60 
         AS SELECT category, SUM(price) FROM orders GROUP BY category"
    )?;
    
    Ok(())
}
```

---

## Using Materialized Views

Once a materialized view is created, the DBX engine check the cache internally when executing the same SQL query.

```rust
// When you execute a regular SELECT statement, DBX returns the cached result if available.
let results = db.execute_sql("SELECT category, SUM(price) FROM orders GROUP BY category")?;
```

> [!NOTE]
> If `REFRESH EVERY` is not specified, the view retains its initial cache until manually refreshed.

---

## Management Commands

### Manual Refresh

Use this to synchronize with the latest data immedately.

```sql
REFRESH MATERIALIZED VIEW sales_summary
```

### Dropping a View

```sql
DROP MATERIALIZED VIEW sales_summary
```

---

## Internal Architecture

1. **Registration**: The SQL statement and refresh interval are stored in the `MaterializedViewRegistry`.
2. **Background Thread**: A dedicated thread (created when the `Database` opens) periodically checks `is_fresh()` and re-calculates expired views (currently every 60s).
3. **Interception**: When `execute_sql` is called, the engine checks if a materialized view matches the query. If a fresh cache exists, it returns it instantly without planning or executing the full query.

---

## Next Steps

- [SQL Reference](sql-reference) — Check supported SQL syntax
- [Streaming Ingestion](streaming-ingestion) — Build real-time data pipelines
- [Storage Layers](storage-layers) — Understand the 5-tier architecture
