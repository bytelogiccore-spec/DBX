---
layout: default
title: Triggers
parent: English
nav_order: 31
---

# Triggers
{: .no_toc }

Register logic that automatically fires on data change events.
{: .fs-6 .fw-300 }

---

## Overview

Triggers are callback functions that execute automatically when INSERT, UPDATE, or DELETE occurs on a table.

| Timing | Description |
|--------|-------------|
| **Before** | Runs before the change (validation, transformation) |
| **After** | Runs after the change (audit logs, notifications) |

---

## Registering a Trigger

```rust
use dbx_core::automation::trigger::{Trigger, TriggerEvent, TriggerTiming};

let audit_trigger = Trigger::new(
    "audit_log",                    // trigger name
    "users",                        // target table
    TriggerEvent::Insert,           // fires on INSERT
    TriggerTiming::After,           // runs after the change
    |event| {
        println!("New user added: {:?}", event.new_values);
        Ok(())
    },
);

let mut registry = TriggerRegistry::new();
registry.register(audit_trigger);
```

---

## Event Types

```rust
pub enum TriggerEvent {
    Insert,     // new row inserted
    Update,     // existing row modified
    Delete,     // row deleted
    Any,        // any change
}
```

---

## Examples

### Data Validation (Before Trigger)

```rust
Trigger::new(
    "validate_age", "users",
    TriggerEvent::Insert, TriggerTiming::Before,
    |event| {
        let age = event.get_field::<i64>("age")?;
        if age < 0 || age > 150 {
            return Err(DbxError::Validation("Invalid age".into()));
        }
        Ok(())
    },
);
```

### Change Tracking (After Trigger)

```rust
Trigger::new(
    "log_changes", "orders",
    TriggerEvent::Any, TriggerTiming::After,
    |event| {
        log::info!("[{}] {}: {:?}", event.table, event.event_type, event.timestamp);
        Ok(())
    },
);
```

---

## Next Steps

- [Scheduler Guide](scheduler) — Register periodic tasks
- [UDF Guide](udf) — Define custom functions
