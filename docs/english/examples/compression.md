---
layout: default
title: Compression
parent: English
nav_order: 14
---

# Compression Quick Start

The fastest way to use compression in DBX.

## 1. Enabling Compression

```rust
use dbx_core::Database;

let db = Database::open("./db")?;

// Enable compression (Default level 3)
db.enable_compression("logs")?;
```

## 2. Inserting Data

```rust
// Insert data (automatic compression)
let large_data = vec![b'x'; 10000];  // 10KB
db.insert("logs", b"log:1", &large_data)?;
```

## 3. Adjusting Compression Levels

```rust
// Fast compression (Level 1)
db.set_compression_level("realtime", 1)?;

// Balanced compression (Level 6)
db.set_compression_level("data", 6)?;

// Maximum compression (Level 15)
db.set_compression_level("archive", 15)?;
```

## 4. Measuring Compression Ratio

```rust
use std::fs;

// Insert data
for i in 0..1000 {
    let key = format!("data:{}", i).into_bytes();
    let value = vec![b'A'; 1000];  // 1KB each
    db.insert("data", &key, &value)?;
}

// Check disk usage
let metadata = fs::metadata("./db/wos/db")?;
let compressed_size = metadata.len();

println!("Compressed size: {} bytes", compressed_size);
```

## 5. Complete Example

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./compressed_db")?;
    
    // Enable compression
    db.enable_compression("logs")?;
    
    // Store large volume of data
    for i in 0..1000 {
        let key = format!("log:{}", i).into_bytes();
        let value = vec![b'A'; 1000];  // 1KB each
        db.insert("logs", &key, &value)?;
    }
    
    println!("✓ 1000 rows compressed and stored");
    println!("✓ Compression ratio: ~10x (estimated)");
    println!("✓ Space saved: ~90%");
    
    Ok(())
}
```

## Next Steps

- [**Compression Guide**](../guides/compression.md) — Complete compression guide
- [**Encryption**](encryption.md) — Data encryption
- [**Quick Start**](quick-start.md) — Basic CRUD
