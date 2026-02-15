---
layout: default
title: GPU Acceleration
parent: English
nav_order: 24
description: "CUDA-based GPU acceleration for analytical queries in DBX"
---

# GPU Acceleration
{: .no_toc }

Complete guide to GPU acceleration in DBX using CUDA.
{: .fs-6 .fw-300 }

## Table of contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX provides optional **CUDA-based GPU acceleration** for analytical queries, offering significant performance improvements for large datasets.

### Performance Gains

| Operation | Dataset Size | CPU Time | GPU Time | Speedup |
|-----------|--------------|----------|----------|---------|
| SUM | 1M rows | 456.66µs | 783.36µs | 0.58x |
| Filter (>500K) | 1M rows | 2.06ms | 673.38µs | **3.06x** |
| SUM | 10M rows | 4.5ms | 1.2ms | **3.75x** |
| Filter | 10M rows | 20ms | 4.4ms | **4.57x** |
| GROUP BY | 10M rows | 35ms | 12ms | **2.92x** |
| Hash Join | 10M rows | 50ms | 18ms | **2.78x** |

> **Note**: GPU shows greater performance gains on larger datasets (>10M rows).

---

## Requirements

### Hardware

- **NVIDIA GPU** with CUDA Compute Capability 6.0+
- **Minimum 2GB VRAM** (4GB+ recommended)
- **PCIe 3.0 x16** or better

### Software

- **CUDA Toolkit 12.x** or later
- **NVIDIA Driver** 525.60.13+ (Linux) or 528.33+ (Windows)
- **Rust 1.70+** with CUDA support

---

## Installation

### 1. Install CUDA Toolkit

**Linux:**
```bash
# Ubuntu/Debian
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-3
```

**Windows:**
Download and install from [NVIDIA CUDA Downloads](https://developer.nvidia.com/cuda-downloads)

### 2. Verify CUDA Installation

```bash
nvcc --version
nvidia-smi
```

### 3. Enable GPU Features in Cargo.toml

```toml
[dependencies]
dbx-core = { version = "{{ site.dbx_version }}", features = ["gpu"] }
```

### 4. Build with GPU Support

```bash
cargo build --features gpu --release
```

---

## Basic Usage

### Initialize GPU Manager

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // GPU manager is automatically initialized if available
    if let Some(gpu) = db.gpu_manager() {
        println!("GPU acceleration available!");
    } else {
        println!("GPU not available, using CPU");
    }
    
    Ok(())
}
```

### Sync Data to GPU

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table with data ...
    
    // Sync table to GPU cache
    db.sync_gpu_cache("users")?;
    
    Ok(())
}
```

---

## GPU Operations

### Aggregations

#### SUM

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table ...
    db.sync_gpu_cache("orders")?;
    
    if let Some(gpu) = db.gpu_manager() {
        // GPU-accelerated SUM
        let total = gpu.sum("orders", "amount")?;
        println!("Total: {}", total);
    }
    
    Ok(())
}
```

#### COUNT

```rust
if let Some(gpu) = db.gpu_manager() {
    let count = gpu.count("users")?;
    println!("Count: {}", count);
}
```

#### MIN / MAX

```rust
if let Some(gpu) = db.gpu_manager() {
    let min_age = gpu.min("users", "age")?;
    let max_age = gpu.max("users", "age")?;
    println!("Age range: {} - {}", min_age, max_age);
}
```

#### AVG

```rust
if let Some(gpu) = db.gpu_manager() {
    let avg_price = gpu.avg("products", "price")?;
    println!("Average price: {:.2}", avg_price);
}
```

### Filtering

#### Greater Than

```rust
if let Some(gpu) = db.gpu_manager() {
    // Filter rows where age > 30
    let filtered = gpu.filter_gt("users", "age", 30)?;
    println!("Found {} users", filtered.len());
}
```

#### Less Than

```rust
let filtered = gpu.filter_lt("products", "price", 100.0)?;
```

#### Equal

```rust
let filtered = gpu.filter_eq("users", "status", "active")?;
```

#### Range

```rust
let filtered = gpu.filter_range("orders", "amount", 100.0, 1000.0)?;
```

### GROUP BY

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table ...
    db.sync_gpu_cache("orders")?;
    
    if let Some(gpu) = db.gpu_manager() {
        // GROUP BY city, SUM(amount)
        let results = gpu.group_by_sum("orders", "city", "amount")?;
        
        for (city, total) in results {
            println!("{}: {}", city, total);
        }
    }
    
    Ok(())
}
```

### Hash Join

```rust
if let Some(gpu) = db.gpu_manager() {
    // Hash join users and orders
    let results = gpu.hash_join(
        "users", "id",
        "orders", "user_id"
    )?;
}
```

---

## Hash Strategies

DBX supports three GPU hash strategies for different performance characteristics:

### Linear Probing (Default)

**Characteristics:**
- Stable performance
- Low memory overhead
- Best for small to medium groups

**Usage:**
```rust
use dbx_core::gpu::HashStrategy;

db.set_gpu_hash_strategy(HashStrategy::Linear)?;
```

**Performance:**
- Baseline performance
- Consistent across workloads

### Cuckoo Hashing

**Characteristics:**
- Aggressive performance
- Higher memory overhead
- Best for large datasets with high collision rates

**Usage:**
```rust
use dbx_core::gpu::HashStrategy;

db.set_gpu_hash_strategy(HashStrategy::Cuckoo)?;
```

**Performance:**
- SUM: **+73%** faster than Linear
- Filtering: **+32%** faster than Linear
- GROUP BY: **+45%** faster than Linear

### Robin Hood Hashing

**Characteristics:**
- Balanced performance
- Moderate memory overhead
- Best for general-purpose workloads

**Usage:**
```rust
use dbx_core::gpu::HashStrategy;

db.set_gpu_hash_strategy(HashStrategy::RobinHood)?;
```

**Performance:**
- SUM: **+7%** faster than Linear
- Filtering: **+10%** faster than Linear
- GROUP BY: **+12%** faster than Linear

### Strategy Selection Guide

| Workload | Recommended Strategy | Reason |
|----------|---------------------|--------|
| Small datasets (<1M rows) | Linear | Lower overhead |
| Large datasets (>10M rows) | Cuckoo | Maximum performance |
| Mixed workloads | Robin Hood | Balanced |
| Memory-constrained | Linear | Lowest memory usage |
| High collision rate | Cuckoo | Best collision handling |

---

## Sharding Strategies

For multi-GPU environments, DBX provides three sharding strategies to distribute data across devices:

| Strategy | Behavior | Recommended For |
|----------|----------|-----------------|
| **RoundRobin** | Distributes rows sequentially | Balanced workloads |
| **Hash** | Hash-based distribution on first column (ahash) | GROUP BY, JOIN queries |
| **Range** | Assigns contiguous row ranges | Sorted data, range scans |

```rust
use dbx_core::storage::gpu::ShardingStrategy;

let manager = ShardManager::new(device_count, ShardingStrategy::Hash);
let shards = manager.shard_batch(&batch)?;
```

---

## PTX Persistent Kernel

Uses NVRTC to compile CUDA C kernels to PTX at runtime. The kernel persists on GPU, continuously processing work queue items until shutdown.

```rust
use dbx_core::storage::gpu::persistent::PersistentKernelManager;

let manager = PersistentKernelManager::new(device.clone());
manager.compile_kernel()?;

if let Some(func) = manager.get_kernel_function() {
    // Execute kernel
}
```

> **Note**: Only available with the `gpu` feature enabled. As of `cudarc` 0.19.2, Unified Memory and P2P access are not supported; host memory with explicit transfers is used instead.

---

## CUDA Stream Management

Create separate streams for parallel GPU operations via `fork_default_stream()`:

```rust
use dbx_core::engine::stream::GpuStreamContext;

let ctx = GpuStreamContext::new(device.clone())?;
// Execute async GPU work on separate stream
```

---

## SQL Integration

GPU acceleration is automatically used for compatible SQL operations:

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register table ...
    db.sync_gpu_cache("orders")?;
    
    // These operations automatically use GPU if available:
    
    // 1. Aggregations
    let results = db.execute_sql(
        "SELECT SUM(amount), AVG(amount) FROM orders"
    )?;
    
    // 2. Filtering
    let results = db.execute_sql(
        "SELECT * FROM orders WHERE amount > 1000"
    )?;
    
    // 3. GROUP BY
    let results = db.execute_sql(
        "SELECT city, SUM(amount) FROM orders GROUP BY city"
    )?;
    
    // 4. Joins
    let results = db.execute_sql(
        "SELECT u.name, o.amount 
         FROM users u 
         JOIN orders o ON u.id = o.user_id"
    )?;
    
    Ok(())
}
```

---

## Performance Tuning

### Memory Management

#### Allocate Sufficient VRAM

```rust
use dbx_core::gpu::GpuConfig;

let config = GpuConfig::default()
    .max_memory_mb(2048)  // 2GB VRAM
    .cache_size(1000000); // 1M records

db.configure_gpu(config)?;
```

#### Monitor GPU Memory

```rust
if let Some(gpu) = db.gpu_manager() {
    let stats = gpu.memory_stats()?;
    println!("Used: {} MB / {} MB", stats.used_mb, stats.total_mb);
}
```

### Batch Size Optimization

```rust
let config = GpuConfig::default()
    .batch_size(10000);  // Process 10k rows per batch

db.configure_gpu(config)?;
```

### Data Transfer Optimization

#### Minimize CPU-GPU Transfers

```rust
// Good: Sync once, query multiple times
db.sync_gpu_cache("orders")?;

for i in 0..100 {
    let results = gpu.filter_gt("orders", "amount", i * 100)?;
}

// Avoid: Sync on every query
for i in 0..100 {
    db.sync_gpu_cache("orders")?;  // Too frequent!
    let results = gpu.filter_gt("orders", "amount", i * 100)?;
}
```

#### Async Transfers

```rust
// Async transfer (non-blocking)
db.sync_gpu_cache_async("orders")?;

// Continue with other work
// ...

// Wait for completion
db.wait_gpu_sync()?;
```

---

## Benchmarking

### Run GPU Benchmarks

```bash
cd testing/benchmarks
cargo bench --features gpu
```

### Custom Benchmarks

```rust
use dbx_core::Database;
use std::time::Instant;

fn benchmark_gpu_vs_cpu() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // ... register large dataset ...
    
    // CPU benchmark
    let start = Instant::now();
    let cpu_result = db.execute_sql("SELECT SUM(amount) FROM orders")?;
    let cpu_time = start.elapsed();
    
    // GPU benchmark
    db.sync_gpu_cache("orders")?;
    let start = Instant::now();
    let gpu_result = db.gpu_manager().unwrap().sum("orders", "amount")?;
    let gpu_time = start.elapsed();
    
    println!("CPU: {:?}", cpu_time);
    println!("GPU: {:?}", gpu_time);
    println!("Speedup: {:.2}x", cpu_time.as_secs_f64() / gpu_time.as_secs_f64());
    
    Ok(())
}
```

---

## Troubleshooting

### GPU Not Detected

**Problem:** `gpu_manager()` returns `None`

**Solutions:**
1. Verify CUDA installation: `nvcc --version`
2. Check NVIDIA driver: `nvidia-smi`
3. Rebuild with GPU features: `cargo build --features gpu`
4. Check GPU compatibility (Compute Capability 6.0+)

### Out of Memory Errors

**Problem:** `CudaError: out of memory`

**Solutions:**
1. Reduce batch size:
   ```rust
   let config = GpuConfig::default().batch_size(5000);
   db.configure_gpu(config)?;
   ```

2. Clear GPU cache:
   ```rust
   db.clear_gpu_cache()?;
   ```

3. Use smaller datasets or split queries

### Slow Performance

**Problem:** GPU slower than CPU

**Possible Causes:**
1. **Dataset too small** - GPU overhead dominates
2. **Frequent CPU-GPU transfers** - Minimize syncs
3. **Wrong hash strategy** - Try Cuckoo for large datasets

**Solutions:**
```rust
// Use GPU only for large datasets
if row_count > 1_000_000 {
    db.sync_gpu_cache("table")?;
    // Use GPU operations
} else {
    // Use CPU operations
}
```

---

## Advanced Features

### Custom CUDA Kernels

For advanced users, DBX allows custom CUDA kernels:

```rust
use dbx_core::gpu::CudaKernel;

let kernel = CudaKernel::from_source(r#"
    __global__ void custom_filter(int* data, int* result, int threshold, int n) {
        int idx = blockIdx.x * blockDim.x + threadIdx.x;
        if (idx < n) {
            result[idx] = data[idx] > threshold ? 1 : 0;
        }
    }
"#)?;

db.register_kernel("custom_filter", kernel)?;
```

### Multi-GPU Support

```rust
use dbx_core::gpu::GpuConfig;

let config = GpuConfig::default()
    .device_ids(vec![0, 1, 2, 3]);  // Use 4 GPUs

db.configure_gpu(config)?;
```

---

## Best Practices

### 1. Use GPU for Large Datasets

```rust
// Good: Large dataset (>1M rows)
if row_count > 1_000_000 {
    db.sync_gpu_cache("table")?;
    let result = gpu.sum("table", "column")?;
}

// Avoid: Small dataset
if row_count < 10_000 {
    // CPU is faster for small datasets
}
```

### 2. Batch GPU Operations

```rust
// Good: Batch multiple operations
db.sync_gpu_cache("orders")?;
let sum = gpu.sum("orders", "amount")?;
let avg = gpu.avg("orders", "amount")?;
let count = gpu.count("orders")?;

// Avoid: Sync for each operation
db.sync_gpu_cache("orders")?;
let sum = gpu.sum("orders", "amount")?;
db.sync_gpu_cache("orders")?;  // Redundant!
let avg = gpu.avg("orders", "amount")?;
```

### 3. Choose Appropriate Hash Strategy

```rust
// Large dataset with many groups
db.set_gpu_hash_strategy(HashStrategy::Cuckoo)?;

// General-purpose workload
db.set_gpu_hash_strategy(HashStrategy::RobinHood)?;
```

### 4. Monitor GPU Utilization

```rust
if let Some(gpu) = db.gpu_manager() {
    let stats = gpu.utilization_stats()?;
    println!("GPU Utilization: {}%", stats.utilization);
    println!("Memory Used: {} MB", stats.memory_used_mb);
}
```

---

## Next Steps

- [SQL Reference](sql-reference) — Use GPU with SQL queries
- [Storage Layers](storage-layers) — Understand data flow
- [Performance Benchmarks](../benchmarks) — Optimize GPU performance
- [Benchmarks](../benchmarks) — See detailed performance comparisons
