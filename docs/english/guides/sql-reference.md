---
layout: default
title: SQL Reference
parent: English
nav_order: 21
description: "Complete SQL syntax reference for DBX"
---

# SQL Reference
{: .no_toc }

Complete reference for SQL queries in DBX.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX supports standard SQL queries through Apache Arrow and DataFusion integration. SQL queries operate on the **Columnar Cache** layer for optimal analytical performance.

### Supported Features

- ✅ **SELECT** - Column projection and filtering
- ✅ **WHERE** - Predicate filtering
- ✅ **JOIN** - Inner, Left, Right, Full Outer joins
- ✅ **GROUP BY** - Aggregation and grouping
- ✅ **ORDER BY** - Result sorting
- ✅ **LIMIT** - Result limiting
- ✅ **Aggregate Functions** - SUM, COUNT, MIN, MAX, AVG
- ✅ **Scalar Functions** - String, math, date functions

---

## Basic Queries

### SELECT Statement

Select all columns:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table with data ...
    
    let results = db.execute_sql("SELECT * FROM users")?;
    
    Ok(())
}
```

Select specific columns:

```rust
let results = db.execute_sql("SELECT id, name, email FROM users")?;
```

Column aliases:

```rust
let results = db.execute_sql(
    "SELECT id AS user_id, name AS full_name FROM users"
)?;
```

### WHERE Clause

Basic filtering:

```rust
let results = db.execute_sql(
    "SELECT * FROM users WHERE age > 30"
)?;
```

Multiple conditions:

```rust
let results = db.execute_sql(
    "SELECT * FROM users WHERE age > 30 AND city = 'Seoul'"
)?;
```

Comparison operators:

```rust
// Equal
"SELECT * FROM users WHERE status = 'active'"

// Not equal
"SELECT * FROM users WHERE status != 'deleted'"

// Greater than / Less than
"SELECT * FROM orders WHERE amount > 1000"
"SELECT * FROM orders WHERE amount <= 500"

// LIKE pattern matching
"SELECT * FROM users WHERE email LIKE '%@gmail.com'"

// IN operator
"SELECT * FROM users WHERE city IN ('Seoul', 'Busan', 'Incheon')"

// BETWEEN
"SELECT * FROM orders WHERE created_at BETWEEN '2024-01-01' AND '2024-12-31'"
```

### ORDER BY Clause

Ascending order:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY age ASC"
)?;
```

Descending order:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY created_at DESC"
)?;
```

Multiple columns:

```rust
let results = db.execute_sql(
    "SELECT * FROM users ORDER BY city ASC, age DESC"
)?;
```

### LIMIT Clause

Limit results:

```rust
let results = db.execute_sql(
    "SELECT * FROM users LIMIT 10"
)?;
```

With offset:

```rust
let results = db.execute_sql(
    "SELECT * FROM users LIMIT 10 OFFSET 20"
)?;
```

---

## Aggregate Functions

### COUNT

Count all rows:

```rust
let results = db.execute_sql(
    "SELECT COUNT(*) FROM users"
)?;
```

Count non-null values:

```rust
let results = db.execute_sql(
    "SELECT COUNT(email) FROM users"
)?;
```

Count distinct:

```rust
let results = db.execute_sql(
    "SELECT COUNT(DISTINCT city) FROM users"
)?;
```

### SUM

Sum numeric column:

```rust
let results = db.execute_sql(
    "SELECT SUM(amount) FROM orders"
)?;
```

### AVG

Average value:

```rust
let results = db.execute_sql(
    "SELECT AVG(age) FROM users"
)?;
```

### MIN / MAX

Minimum and maximum:

```rust
let results = db.execute_sql(
    "SELECT MIN(age), MAX(age) FROM users"
)?;
```

---

## GROUP BY

### Basic Grouping

Group by single column:

```rust
let results = db.execute_sql(
    "SELECT city, COUNT(*) FROM users GROUP BY city"
)?;
```

Group by multiple columns:

```rust
let results = db.execute_sql(
    "SELECT city, status, COUNT(*) 
     FROM users 
     GROUP BY city, status"
)?;
```

### HAVING Clause

Filter grouped results:

```rust
let results = db.execute_sql(
    "SELECT city, COUNT(*) as user_count
     FROM users 
     GROUP BY city
     HAVING user_count > 100"
)?;
```

Complex aggregations:

```rust
let results = db.execute_sql(
    "SELECT 
        city,
        COUNT(*) as total_users,
        AVG(age) as avg_age,
        SUM(order_count) as total_orders
     FROM users
     GROUP BY city
     HAVING total_users > 50 AND avg_age > 25"
)?;
```

---

## JOIN Operations

### INNER JOIN

Join two tables:

```rust
let results = db.execute_sql(
    "SELECT u.id, u.name, o.order_id, o.amount
     FROM users u
     INNER JOIN orders o ON u.id = o.user_id"
)?;
```

### LEFT JOIN

Include all rows from left table:

```rust
let results = db.execute_sql(
    "SELECT u.id, u.name, o.order_id
     FROM users u
     LEFT JOIN orders o ON u.id = o.user_id"
)?;
```

### RIGHT JOIN

Include all rows from right table:

```rust
let results = db.execute_sql(
    "SELECT u.id, u.name, o.order_id
     FROM users u
     RIGHT JOIN orders o ON u.id = o.user_id"
)?;
```

### FULL OUTER JOIN

Include all rows from both tables:

```rust
let results = db.execute_sql(
    "SELECT u.id, u.name, o.order_id
     FROM users u
     FULL OUTER JOIN orders o ON u.id = o.user_id"
)?;
```

### Multiple Joins

Join multiple tables:

```rust
let results = db.execute_sql(
    "SELECT 
        u.name,
        o.order_id,
        p.product_name,
        p.price
     FROM users u
     INNER JOIN orders o ON u.id = o.user_id
     INNER JOIN products p ON o.product_id = p.id"
)?;
```

---

## Scalar Functions

### String Functions

```rust
// UPPER / LOWER
"SELECT UPPER(name), LOWER(email) FROM users"

// LENGTH
"SELECT name, LENGTH(name) as name_length FROM users"

// SUBSTRING
"SELECT SUBSTRING(email, 1, 10) FROM users"

// CONCAT
"SELECT CONCAT(first_name, ' ', last_name) as full_name FROM users"

// TRIM
"SELECT TRIM(name) FROM users"
```

### Math Functions

```rust
// ABS
"SELECT ABS(balance) FROM accounts"

// ROUND
"SELECT ROUND(price, 2) FROM products"

// FLOOR / CEIL
"SELECT FLOOR(rating), CEIL(rating) FROM reviews"

// POWER
"SELECT POWER(value, 2) FROM measurements"
```

### Date Functions

```rust
// CURRENT_DATE
"SELECT CURRENT_DATE()"

// CURRENT_TIMESTAMP
"SELECT CURRENT_TIMESTAMP()"

// DATE_TRUNC
"SELECT DATE_TRUNC('day', created_at) FROM orders"

// EXTRACT
"SELECT EXTRACT(YEAR FROM created_at) as year FROM orders"
```

---

## Advanced Queries

### Subqueries

Subquery in WHERE:

```rust
let results = db.execute_sql(
    "SELECT * FROM users
     WHERE age > (SELECT AVG(age) FROM users)"
)?;
```

Subquery in FROM:

```rust
let results = db.execute_sql(
    "SELECT city, avg_age
     FROM (
         SELECT city, AVG(age) as avg_age
         FROM users
         GROUP BY city
     ) AS city_stats
     WHERE avg_age > 30"
)?;
```

### CASE Expressions

Simple CASE:

```rust
let results = db.execute_sql(
    "SELECT 
        name,
        CASE 
            WHEN age < 18 THEN 'Minor'
            WHEN age < 65 THEN 'Adult'
            ELSE 'Senior'
        END as age_group
     FROM users"
)?;
```

CASE with aggregation:

```rust
let results = db.execute_sql(
    "SELECT 
        city,
        COUNT(CASE WHEN status = 'active' THEN 1 END) as active_users,
        COUNT(CASE WHEN status = 'inactive' THEN 1 END) as inactive_users
     FROM users
     GROUP BY city"
)?;
```

### Window Functions

(Note: Window function support depends on DataFusion version)

```rust
// ROW_NUMBER
"SELECT 
    name,
    age,
    ROW_NUMBER() OVER (ORDER BY age DESC) as rank
 FROM users"

// RANK
"SELECT 
    name,
    score,
    RANK() OVER (ORDER BY score DESC) as rank
 FROM students"

// Partitioned window
"SELECT 
    name,
    city,
    age,
    AVG(age) OVER (PARTITION BY city) as city_avg_age
 FROM users"
```

---

## Query Optimization

### Projection Pushdown

DBX automatically pushes column selection down to storage:

```rust
// Only reads 'id' and 'name' columns from storage
let results = db.execute_sql(
    "SELECT id, name FROM users"
)?;
```

### Predicate Pushdown

Filters are applied at the storage layer:

```rust
// Filter applied during scan, not after
let results = db.execute_sql(
    "SELECT * FROM users WHERE age > 30"
)?;
```

### Vectorized Execution

Queries use SIMD vectorization automatically:

```rust
// Vectorized aggregation
let results = db.execute_sql(
    "SELECT SUM(amount) FROM orders"
)?;
```

---

## Working with RecordBatch

### Registering Tables

Register Arrow RecordBatch as a table:

```rust
use dbx_core::Database;
use arrow::array::{Int32Array, StringArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create schema
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("name", DataType::Utf8, false),
        Field::new("age", DataType::Int32, false),
    ]));
    
    // Create data
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(Int32Array::from(vec![1, 2, 3])),
            Arc::new(StringArray::from(vec!["Alice", "Bob", "Charlie"])),
            Arc::new(Int32Array::from(vec![25, 30, 35])),
        ],
    ).unwrap();
    
    // Register table
    db.register_table("users", vec![batch]);
    
    // Now you can query it
    let results = db.execute_sql("SELECT * FROM users WHERE age > 28")?;
    
    Ok(())
}
```

### Processing Results

Iterate through results:

```rust
use arrow::array::AsArray;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table ...
    
    let results = db.execute_sql("SELECT id, name FROM users")?;
    
    for batch in results {
        let id_array = batch.column(0).as_primitive::<arrow::datatypes::Int32Type>();
        let name_array = batch.column(1).as_string::<i32>();
        
        for i in 0..batch.num_rows() {
            let id = id_array.value(i);
            let name = name_array.value(i);
            println!("ID: {}, Name: {}", id, name);
        }
    }
    
    Ok(())
}
```

---

## GPU Acceleration

### Enabling GPU for SQL

When GPU features are enabled, certain operations automatically use GPU:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table ...
    
    // Sync to GPU cache
    db.sync_gpu_cache("users")?;
    
    // These operations may use GPU acceleration:
    // - SUM, COUNT, MIN, MAX, AVG
    // - Filtering (WHERE clauses)
    // - GROUP BY operations
    // - Hash joins
    
    let results = db.execute_sql(
        "SELECT city, SUM(amount) 
         FROM orders 
         GROUP BY city"
    )?;
    
    Ok(())
}
```

---

## Performance Tips

### 1. Use Specific Columns

```rust
// Good: Only select needed columns
"SELECT id, name FROM users"

// Avoid: Select all columns
"SELECT * FROM users"
```

### 2. Filter Early

```rust
// Good: Filter before join
"SELECT u.name, o.amount
 FROM users u
 INNER JOIN (
     SELECT * FROM orders WHERE amount > 1000
 ) o ON u.id = o.user_id"
```

### 3. Use Appropriate Indexes

```rust
// Ensure Bloom filters are updated
db.rebuild_index("users")?;
```

### 4. Batch Queries

```rust
// Good: Single query with aggregation
"SELECT city, COUNT(*), AVG(age) FROM users GROUP BY city"

// Avoid: Multiple separate queries
// "SELECT COUNT(*) FROM users WHERE city = 'Seoul'"
// "SELECT COUNT(*) FROM users WHERE city = 'Busan'"
// ...
```

---

## Prepared Statements

### Creating Prepared Statements

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Prepare statement
    let stmt = db.prepare("SELECT * FROM users WHERE age > ?")?;
    
    // Execute with parameters
    let results = stmt.execute(&[30])?;
    
    Ok(())
}
```

### Benefits

- **Performance**: Query is parsed once, executed multiple times
- **Security**: Prevents SQL injection
- **Type Safety**: Parameter binding with type checking

---

## Error Handling

### Common SQL Errors

```rust
use dbx_core::{Database, DbxError};

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    match db.execute_sql("SELECT * FROM nonexistent_table") {
        Ok(results) => {
            // Process results
        }
        Err(DbxError::TableNotFound) => {
            println!("Table does not exist");
        }
        Err(DbxError::SqlParseError(msg)) => {
            println!("SQL syntax error: {}", msg);
        }
        Err(e) => {
            eprintln!("Query failed: {}", e);
        }
    }
    
    Ok(())
}
```

---

## SQL Examples

### Analytics Query

```rust
let results = db.execute_sql(
    "SELECT 
        DATE_TRUNC('month', order_date) as month,
        city,
        COUNT(*) as order_count,
        SUM(amount) as total_revenue,
        AVG(amount) as avg_order_value
     FROM orders
     WHERE order_date >= '2024-01-01'
     GROUP BY month, city
     HAVING total_revenue > 10000
     ORDER BY month DESC, total_revenue DESC
     LIMIT 100"
)?;
```

### User Segmentation

```rust
let results = db.execute_sql(
    "SELECT 
        CASE 
            WHEN order_count = 0 THEN 'New'
            WHEN order_count < 5 THEN 'Occasional'
            WHEN order_count < 20 THEN 'Regular'
            ELSE 'VIP'
        END as segment,
        COUNT(*) as user_count,
        AVG(lifetime_value) as avg_ltv
     FROM users
     GROUP BY segment
     ORDER BY avg_ltv DESC"
)?;
```

### Top N per Group

```rust
let results = db.execute_sql(
    "SELECT *
     FROM (
         SELECT 
             product_name,
             category,
             sales,
             ROW_NUMBER() OVER (
                 PARTITION BY category 
                 ORDER BY sales DESC
             ) as rank
         FROM products
     ) ranked
     WHERE rank <= 5"
)?;
```

---

## Next Steps

- [CRUD Operations](crud-operations) — Basic database operations
- [Transactions](transactions) — Use SQL with transactions
- [GPU Acceleration](gpu-acceleration) — Accelerate SQL queries
- [API Reference](../api/sql) — Complete SQL API documentation
