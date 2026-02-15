---
layout: default
title: Job Scheduler
parent: English
nav_order: 32
---

# Job Scheduler
{: .no_toc }

Automate periodic maintenance tasks and batch processing.
{: .fs-6 .fw-300 }

---

## Overview

The scheduler registers and runs jobs periodically using cron expressions.

---

## Registering Jobs

```rust
use dbx_core::automation::scheduler::{JobScheduler, Job, Schedule};

let mut scheduler = JobScheduler::new();

// Refresh stats every 5 minutes
scheduler.schedule(Job::new(
    "refresh_stats",
    Schedule::cron("*/5 * * * *"),
    || {
        println!("Refreshing stats table...");
        Ok(())
    },
));

// Cleanup at midnight
scheduler.schedule(Job::new(
    "daily_cleanup",
    Schedule::cron("0 0 * * *"),
    || {
        println!("Cleaning up expired sessions...");
        Ok(())
    },
));
```

---

## Cron Expressions

| Expression | Description |
|------------|-------------|
| `*/5 * * * *` | Every 5 minutes |
| `0 * * * *` | Every hour |
| `0 0 * * *` | Daily at midnight |
| `0 0 * * 0` | Every Sunday at midnight |
| `0 0 1 * *` | First day of every month |

Format: `minute hour day month weekday`

---

## Job Management

```rust
// List registered jobs
let jobs = scheduler.list_jobs();
for job in &jobs {
    println!("{}: next run {:?}", job.name, job.next_run);
}

// Disable a specific job
scheduler.disable("refresh_stats");

// Run immediately
scheduler.run_now("daily_cleanup")?;
```

---

## Use Cases

| Task | Interval | Description |
|------|----------|-------------|
| WAL checkpoint | 5 min | Sync in-memory data to disk |
| Index optimization | Daily | Rebuild fragmented indexes |
| Stats refresh | 1 hour | Update table stats for query optimizer |
| Expired data cleanup | Daily | Delete rows past TTL |

---

## Next Steps

- [Triggers Guide](triggers) — Event-based automation
- [Feature Flags Guide](feature-flags) — Toggle scheduler functionality
