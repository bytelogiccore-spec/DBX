---
layout: default
title: SQL API
nav_order: 3
parent: API Reference
---

# SQL API

SQL query execution for DBX.

---

## Overview

DBX provides **SQL support** for complex queries using Apache Arrow and DataFusion-inspired query execution.

**Supported SQL Features:**
- `SELECT` - Column projection
- `WHERE` - Filtering with predicates
- `JOIN` - Inner joins
- `GROUP BY` - Aggregation
- `ORDER BY` - Sorting
- `LIMIT` - Result limiting

---

## Executing SQL

### `execute_sql(sql: &str) -> DbxResult<RecordBatch>`

Executes a SQL query and returns results as an Arrow RecordBatch.

**Parameters:**
- `sql` - SQL query string

**Returns:**
- `DbxResult<RecordBatch>` - Query results in Arrow format

**Example:**
```rust
let result = db.execute_sql("SELECT name, age FROM users WHERE age > 18")?;
```

---

## SQL Syntax

### SELECT Statement

**Basic SELECT:**
```sql
SELECT column1, column2 FROM table_name
```

**Example:**
```rust
let result = db.execute_sql("SELECT name, email FROM users")?;
```

---

### WHERE Clause

**Filtering:**
```sql
SELECT * FROM table_name WHERE condition
```

**Supported Operators:**
- `=` - Equal
- `!=` or `<>` - Not equal
- `>` - Greater than
- `>=` - Greater than or equal
- `<` - Less than
- `<=` - Less than or equal
- `AND` - Logical AND
- `OR` - Logical OR
- `NOT` - Logical NOT

**Example:**
```rust
let result = db.execute_sql(
    "SELECT name FROM users WHERE age >= 18 AND city = 'Seoul'"
)?;
```

---

### JOIN Clause

**Inner Join:**
```sql
SELECT t1.col1, t2.col2 
FROM table1 t1 
JOIN table2 t2 ON t1.id = t2.user_id
```

**Example:**
```rust
let result = db.execute_sql(
    "SELECT u.name, o.amount 
     FROM users u 
     JOIN orders o ON u.id = o.user_id"
)?;
```

---

### GROUP BY Clause

**Aggregation:**
```sql
SELECT column, AGG_FUNC(column) 
FROM table_name 
GROUP BY column
```

**Supported Aggregate Functions:**
- `COUNT(*)` - Count rows
- `SUM(column)` - Sum values
- `AVG(column)` - Average values
- `MIN(column)` - Minimum value
- `MAX(column)` - Maximum value

**Example:**
```rust
let result = db.execute_sql(
    "SELECT city, COUNT(*), AVG(age) 
     FROM users 
     GROUP BY city"
)?;
```

---

### ORDER BY Clause

**Sorting:**
```sql
SELECT * FROM table_name ORDER BY column [ASC|DESC]
```

**Example:**
```rust
let result = db.execute_sql(
    "SELECT name, age FROM users ORDER BY age DESC"
)?;
```

---

### LIMIT Clause

**Result Limiting:**
```sql
SELECT * FROM table_name LIMIT n
```

**Example:**
```rust
let result = db.execute_sql(
    "SELECT name FROM users ORDER BY created_at DESC LIMIT 10"
)?;
```

---

## Working with Results

### RecordBatch

Results are returned as Arrow `RecordBatch`, which provides:
- **Zero-copy** access to data
- **Columnar format** for efficient processing
- **Type safety** with Arrow schema

**Example:**
```rust
use arrow::array::StringArray;

let result = db.execute_sql("SELECT name FROM users")?;

// Access column
let name_col = result
    .column(0)
    .as_any()
    .downcast_ref::<StringArray>()
    .unwrap();

// Iterate rows
for i in 0..result.num_rows() {
    println!("Name: {}", name_col.value(i));
}
```

---

### Converting to Rust Types

**Example: Extract to Vec:**
```rust
use arrow::array::{Int64Array, StringArray};

let result = db.execute_sql("SELECT id, name FROM users")?;

let ids = result.column(0).as_any().downcast_ref::<Int64Array>().unwrap();
let names = result.column(1).as_any().downcast_ref::<StringArray>().unwrap();

let users: Vec<(i64, String)> = (0..result.num_rows())
    .map(|i| (ids.value(i), names.value(i).to_string()))
    .collect();
```

---

## Query Optimization

### Automatic Optimizations

DBX automatically applies:

1. **Projection Pushdown** - Only read required columns
2. **Predicate Pushdown** - Filter data early
3. **Column Pruning** - Skip unnecessary columns
4. **Vectorized Execution** - SIMD operations

**Example:**
```sql
-- Only reads 'age' column, filters early
SELECT COUNT(*) FROM users WHERE age > 18
```

---

### GPU Acceleration

For large datasets (>1M rows), DBX can use GPU acceleration:

```rust
// GPU is automatically used for supported operations
let result = db.execute_sql(
    "SELECT SUM(amount) FROM transactions WHERE amount > 500000"
)?;
```

**GPU-Accelerated Operations:**
- `SUM`, `AVG`, `MIN`, `MAX`
- `WHERE` with numeric predicates
- Large table scans (>1M rows)

---

## Performance Tips

### Best Practices

1. **Use column projection** - Select only needed columns
   ```sql
   -- ✅ Good
   SELECT name, age FROM users
   
   -- ❌ Bad
   SELECT * FROM users
   ```

2. **Filter early** - Use WHERE clause to reduce data
   ```sql
   -- ✅ Good
   SELECT name FROM users WHERE age > 18
   
   -- ❌ Bad (filter in application)
   SELECT name, age FROM users
   ```

3. **Use indexes** - Create indexes for frequently queried columns
   ```rust
   db.create_index("users", "age")?;
   ```

4. **Batch queries** - Combine multiple queries when possible
   ```sql
   -- ✅ Good
   SELECT city, COUNT(*), AVG(age) FROM users GROUP BY city
   
   -- ❌ Bad (multiple queries)
   -- SELECT COUNT(*) FROM users WHERE city = 'Seoul'
   -- SELECT AVG(age) FROM users WHERE city = 'Seoul'
   ```

---

## SQL Examples

### Example 1: User Analytics

```rust
let result = db.execute_sql(
    "SELECT 
        city,
        COUNT(*) as user_count,
        AVG(age) as avg_age,
        MAX(created_at) as last_signup
     FROM users
     WHERE active = true
     GROUP BY city
     ORDER BY user_count DESC
     LIMIT 10"
)?;
```

---

### Example 2: Sales Report

```rust
let result = db.execute_sql(
    "SELECT 
        p.name as product_name,
        SUM(o.quantity) as total_sold,
        SUM(o.amount) as total_revenue
     FROM orders o
     JOIN products p ON o.product_id = p.id
     WHERE o.created_at >= '2026-01-01'
     GROUP BY p.name
     ORDER BY total_revenue DESC"
)?;
```

---

### Example 3: Top Customers

```rust
let result = db.execute_sql(
    "SELECT 
        u.name,
        COUNT(o.id) as order_count,
        SUM(o.amount) as total_spent
     FROM users u
     JOIN orders o ON u.id = o.user_id
     GROUP BY u.name
     HAVING total_spent > 10000
     ORDER BY total_spent DESC
     LIMIT 20"
)?;
```

---

## Error Handling

### SQL Errors

- `DbxError::SqlParse` - SQL syntax error
- `DbxError::SqlExecution` - Query execution error
- `DbxError::ColumnNotFound` - Referenced column doesn't exist
- `DbxError::TableNotFound` - Referenced table doesn't exist

**Example:**
```rust
match db.execute_sql("SELECT * FROM nonexistent_table") {
    Ok(result) => println!("Success"),
    Err(DbxError::TableNotFound(table)) => {
        eprintln!("Table not found: {}", table);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Limitations

### Current Limitations

- ❌ No `UPDATE` or `DELETE` via SQL (use CRUD API)
- ❌ No subqueries
- ❌ No window functions
- ❌ No `UNION` or `INTERSECT`
- ❌ No `LEFT JOIN` or `RIGHT JOIN` (only `INNER JOIN`)

### Workarounds

**UPDATE via CRUD:**
```rust
// Instead of: UPDATE users SET age = 30 WHERE id = 1
db.insert("users", b"user:1", b"30")?;
```

**DELETE via CRUD:**
```rust
// Instead of: DELETE FROM users WHERE id = 1
db.delete("users", b"user:1")?;
```

---

## See Also

- [Database API](database) - Core database operations
- [SQL Reference Guide](../guides/sql-reference) - Detailed SQL syntax
- [GPU Acceleration Guide](../guides/gpu-acceleration) - GPU query optimization
