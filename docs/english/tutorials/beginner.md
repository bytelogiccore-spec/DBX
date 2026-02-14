---
layout: default
title: Beginner Tutorial
parent: Tutorials
nav_order: 1
description: "Step-by-step tutorial for DBX beginners"
---

# Beginner Tutorial
{: .no_toc }

Step-by-step guide to get started with DBX.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Introduction

This tutorial will guide you through creating your first DBX database, performing basic operations, and running simple SQL queries.

**What you'll learn:**
- Installing DBX
- Creating a database
- Inserting and querying data
- Using transactions
- Running SQL queries

**Prerequisites:**
- Rust 1.70 or later
- Basic Rust knowledge

---

## Step 1: Create a New Project

Create a new Rust project:

```bash
cargo new my_dbx_app
cd my_dbx_app
```

Add DBX to `Cargo.toml`:

```toml
[dependencies]
dbx-core = "0.0.1-beta"
arrow = "50.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## Step 2: Your First Database

Create a simple database in `src/main.rs`:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // Create an in-memory database
    let db = Database::open_in_memory()?;
    
    println!("Database created successfully!");
    
    Ok(())
}
```

Run it:

```bash
cargo run
```

You should see: `Database created successfully!`

---

## Step 3: Insert Data

Let's add some data to our database:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Insert users
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    db.insert("users", b"user:3", b"Charlie")?;
    
    println!("Inserted 3 users");
    
    Ok(())
}
```

---

## Step 4: Query Data

Retrieve data from the database:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Insert data
    db.insert("users", b"user:1", b"Alice")?;
    
    // Query data
    let value = db.get("users", b"user:1")?;
    
    match value {
        Some(data) => {
            let name = String::from_utf8(data).unwrap();
            println!("Found user: {}", name);
        }
        None => println!("User not found"),
    }
    
    Ok(())
}
```

---

## Step 5: Working with Structured Data

Use Serde to work with structured data:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u32,
    name: String,
    email: String,
    age: u32,
}

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Create a user
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        age: 25,
    };
    
    // Serialize and insert
    let key = format!("user:{}", user.id);
    let value = serde_json::to_vec(&user).unwrap();
    db.insert("users", key.as_bytes(), &value)?;
    
    // Retrieve and deserialize
    let retrieved = db.get("users", key.as_bytes())?;
    if let Some(data) = retrieved {
        let user: User = serde_json::from_slice(&data).unwrap();
        println!("Retrieved user: {:?}", user);
    }
    
    Ok(())
}
```

---

## Step 6: Using Transactions

Perform multiple operations atomically:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // Begin transaction
    let tx = db.begin_transaction()?;
    
    // Insert multiple records
    tx.insert("users", b"user:1", b"Alice")?;
    tx.insert("users", b"user:2", b"Bob")?;
    tx.insert("users", b"user:3", b"Charlie")?;
    
    // Commit all changes
    tx.commit()?;
    
    println!("Transaction committed successfully!");
    
    Ok(())
}
```

---

## Step 7: SQL Queries

Run SQL queries on your data:

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
            Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
            Arc::new(StringArray::from(vec![
                "Alice", "Bob", "Charlie", "David", "Eve"
            ])),
            Arc::new(Int32Array::from(vec![25, 30, 35, 28, 32])),
        ],
    ).unwrap();
    
    // Register table
    db.register_table("users", vec![batch]);
    
    // Run SQL query
    let results = db.execute_sql(
        "SELECT name, age FROM users WHERE age > 28"
    )?;
    
    println!("Query results:");
    for batch in results {
        println!("{:?}", batch);
    }
    
    Ok(())
}
```

---

## Step 8: Persistent Storage

Save data to disk:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // Create persistent database
    let db = Database::open("./my_database")?;
    
    // Insert data
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    
    // Data is automatically persisted
    println!("Data saved to ./my_database");
    
    Ok(())
}
```

To read the data later:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // Open existing database
    let db = Database::open("./my_database")?;
    
    // Query data
    let value = db.get("users", b"user:1")?;
    println!("Retrieved: {:?}", value);
    
    Ok(())
}
```

---

## Complete Example

Here's a complete example combining everything:

```rust
use dbx_core::Database;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u32,
    name: String,
    email: String,
    age: u32,
}

fn main() -> dbx_core::DbxResult<()> {
    // Create database
    let db = Database::open("./tutorial_db")?;
    
    // Create users
    let users = vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            age: 25,
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            age: 30,
        },
        User {
            id: 3,
            name: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            age: 35,
        },
    ];
    
    // Insert users in a transaction
    let tx = db.begin_transaction()?;
    for user in &users {
        let key = format!("user:{}", user.id);
        let value = serde_json::to_vec(&user).unwrap();
        tx.insert("users", key.as_bytes(), &value)?;
    }
    tx.commit()?;
    
    println!("Inserted {} users", users.len());
    
    // Query a specific user
    let key = b"user:2";
    if let Some(data) = db.get("users", key)? {
        let user: User = serde_json::from_slice(&data).unwrap();
        println!("Found user: {:?}", user);
    }
    
    // Count all users
    let count = db.count("users")?;
    println!("Total users: {}", count);
    
    Ok(())
}
```

---

## Next Steps

Congratulations! You've learned the basics of DBX. Here's what to explore next:

- **[Intermediate Tutorial](intermediate)** — Learn advanced features
- **[CRUD Operations Guide](../guides/crud-operations)** — Deep dive into CRUD
- **[Transactions Guide](../guides/transactions)** — Master MVCC transactions
- **[SQL Reference](../guides/sql-reference)** — Learn SQL queries

---

## Common Mistakes

### 1. Forgetting to Commit Transactions

```rust
// Wrong: Transaction not committed
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
// Missing: tx.commit()?;

// Right: Always commit
let tx = db.begin_transaction()?;
tx.insert("users", b"user:1", b"Alice")?;
tx.commit()?;
```

### 2. Not Handling Errors

```rust
// Wrong: Unwrapping can panic
let value = db.get("users", b"user:1").unwrap();

// Right: Handle errors properly
match db.get("users", b"user:1") {
    Ok(Some(value)) => println!("Found: {:?}", value),
    Ok(None) => println!("Not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

### 3. Using Wrong Key Format

```rust
// Wrong: Inconsistent key format
db.insert("users", b"1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;

// Right: Consistent key format
db.insert("users", b"user:1", b"Alice")?;
db.insert("users", b"user:2", b"Bob")?;
```

---

## Exercises

Try these exercises to practice:

1. **User Management System**
   - Create a database with users
   - Implement add, get, update, delete operations
   - Use transactions for batch operations

2. **Simple Blog**
   - Store blog posts with title, content, author
   - Query posts by author
   - Count total posts

3. **Product Catalog**
   - Store products with name, price, category
   - Use SQL to query products by price range
   - Calculate average price per category

---

## Getting Help

- **[Documentation](../)** — Full documentation
- **[Examples](../examples/basic-crud)** — More code examples
- **[GitHub Issues](https://github.com/ByteLogicCore/DBX/issues)** — Report bugs
- **[Discussions](https://github.com/ByteLogicCore/DBX/discussions)** — Ask questions
