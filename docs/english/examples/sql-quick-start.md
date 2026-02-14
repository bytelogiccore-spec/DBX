---
layout: default
title: SQL Quick Start
parent: Examples
nav_order: 2
---

# SQL Quick Start

The fastest way to use SQL in DBX.

## 1. Creating a Table

```rust
use dbx_core::Database;

let db = Database::open_in_memory()?;

db.execute_sql("CREATE TABLE users (
    id INT,
    name TEXT,
    age INT
)")?;
```

## 2. Inserting Data

```rust
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 30)")?;
db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 25)")?;
db.execute_sql("INSERT INTO users VALUES (3, 'Charlie', 35)")?;
```

## 3. Basic Queries

```rust
// Query all data
let results = db.execute_sql("SELECT * FROM users")?;

// Filtering with conditions
let results = db.execute_sql("SELECT * FROM users WHERE age > 25")?;

// Ordering
let results = db.execute_sql("SELECT * FROM users ORDER BY age DESC")?;
```

## 4. Aggregate Functions

```rust
// Total count
let count = db.execute_sql("SELECT COUNT(*) FROM users")?;

// Average age
let avg = db.execute_sql("SELECT AVG(age) FROM users")?;

// Grouping and aggregation
let results = db.execute_sql("
    SELECT age, COUNT(*) as count
    FROM users
    GROUP BY age
")?;
```

## 5. Complete Example

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create table
    db.execute_sql("CREATE TABLE products (
        id INT,
        name TEXT,
        price REAL
    )")?;
    
    // Insert data
    db.execute_sql("INSERT INTO products VALUES (1, 'Laptop', 999.99)")?;
    db.execute_sql("INSERT INTO products VALUES (2, 'Mouse', 29.99)")?;
    
    // Execute query
    let results = db.execute_sql("
        SELECT name, price 
        FROM products 
        WHERE price > 50
        ORDER BY price DESC
    ")?;
    
    println!("Found {} products", results.batches[0].num_rows());
    
    Ok(())
}
```

## Next Steps

- [**SQL Reference**](../guides/sql-reference.md) — Complete SQL reference
- [**GPU Acceleration**](../guides/gpu-acceleration.md) — Accelerate SQL queries
- [**Indexing**](indexing.md) — Query optimization
