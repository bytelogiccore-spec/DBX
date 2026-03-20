---
layout: default
title: Streaming Ingestion
parent: English
nav_order: 23
description: "Guide for building high-performance real-time data pipelines"
---

# Streaming Ingestion
{: .no_toc }

Streaming Ingestion is a high-performance pipeline feature that allows you to stream data seamlessly from external sources or real-time event streams into DBX.
{: .fs-6 .fw-300 }

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

DBX's **StreamIngester** maximizes write performance by optimizing batch processing. It provides significantly higher throughput than individual `insert()` calls and supports CDC (Change Data Capture) patterns.

### Key Features
- **MPSC Pipeline**: Uses a Multi-Producer Single-Consumer model to safely ingest data from multiple sources.
- **Automatic Batching**: Performs a flush automatically when the `batch_size` is reached or the `max_latency` time has elapsed.
- **Full DML Support**: Supports not just `Insert`, but also `Update` and `Delete` events in real-time.

---

## Getting Started

You can create a `StreamIngester` directly from a `Database` instance.

### Example: Real-time Telemetry Ingestion

```rust
use dbx_core::{Database, engine::stream_ingester::StreamEvent};
use std::time::Duration;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // Create an ingester for the 'telemetry' table with batch_size 500 and 100ms max latency
    let ingester = db.create_stream_ingester("telemetry", 500, 100);
    let sender = ingester.sender();

    // The sender can be cloned and used in multiple threads or async tasks
    let tx = sender.clone();
    std::thread::spawn(move || {
        for i in 0..1000 {
            let event = StreamEvent::Insert {
                key: format!("device:{}", i).into_bytes(),
                value: format!("{{\"temp\": {}}}", 20 + (i % 10)).into_bytes(),
            };
            tx.send(vec![event]).unwrap();
        }
    });

    // When work is complete, call flush to save remaining buffers and shut down safely.
    ingester.flush()?;
    
    Ok(())
}
```

---

## Event Types (StreamEvent)

The streaming engine handles three types of events:

- **`Insert`**: Adds a new key-value pair.
- **`Update`**: Updates the value for an existing key (internally an Upsert).
- **`Delete`**: Removes a specific key.

---

## Performance Optimization

| Option | Description | Recommended Setting |
|:---|:---|:---|
| `batch_size` | Number of records to store at once | 1,000 ~ 10,000 |
| `max_latency` | Maximum wait time before a flush (ms) | 100ms ~ 1,000ms |

> [!TIP]
> Increase `batch_size` to maximize throughput, or decrease `max_latency` if real-time responsiveness is critical.

---

## Internal Mechanism

1. **Channel Collection**: Events sent via the `sender` go into an `mpsc` channel handled by a background thread.
2. **Buffering**: The background thread accumulates events into a local vector.
3. **Automatic Flush**: Triggers a flush (writes to Tier 1 Delta Store via `insert_batch()`) when:
   - Vector size >= `batch_size`
   - Elapsed time since last flush >= `max_latency`
4. **Shutdown**: Calling `ingester.flush()` processes all remaining data and safely joins the background thread.

---

## Next Steps

- [CRUD Operations](crud-operations) — Standard data insertion methods
- [Materialized Views](materialized-views) — Build real-time analytical views on ingested data
- [Database Config](db-config) — Optimize durability and synchronization settings
