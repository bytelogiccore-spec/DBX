---
layout: default
title: SQL Triggers
parent: English
nav_order: 32
---

# SQL Triggers
{: .no_toc }

Define automatic SQL execution logic on data changes using SQL standard syntax.
{: .fs-6 .fw-300 }

---

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

SQL Triggers automatically execute SQL statements when INSERT, UPDATE, or DELETE events occur, using SQL standard syntax.

> **Note**: For Rust closure-based EventHook, see [EventHook Guide](event-hook).

### Key Features

- ✅ SQL standard syntax support
- ✅ Metadata persistence
- ✅ Auto-registration on database restart
- ✅ BEFORE/AFTER execution timing

---

## CREATE TRIGGER

### Basic Syntax

```sql
CREATE TRIGGER trigger_name
{BEFORE | AFTER} {INSERT | UPDATE | DELETE} ON table_name
FOR EACH ROW
[WHEN (condition)]
BEGIN
    sql_statement;
    ...
END;
```

### Example: Audit Log

```sql
CREATE TRIGGER audit_trigger
AFTER INSERT ON users
FOR EACH ROW
BEGIN
    INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', NOW());
END;
```

### Example: Data Validation

```sql
CREATE TRIGGER validate_price
BEFORE UPDATE ON products
FOR EACH ROW
WHEN (NEW.price < 0)
BEGIN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'Price must be non-negative';
END;
```

---

## DROP TRIGGER

```sql
DROP TRIGGER trigger_name;
```

---

## Rust API

### Parsing Triggers

```rust
use dbx_core::automation::{parse_create_trigger, parse_drop_trigger};

// Parse CREATE TRIGGER
let sql = r#"
    CREATE TRIGGER audit_trigger
    AFTER INSERT ON users
    FOR EACH ROW
    BEGIN
        INSERT INTO audit_logs VALUES (NEW.id, 'INSERT');
    END;
"#;

let trigger = parse_create_trigger(sql)?;
println!("Trigger: {}", trigger.name);

// Parse DROP TRIGGER
let drop_sql = "DROP TRIGGER audit_trigger;";
let name = parse_drop_trigger(drop_sql)?;
```

### Metadata Storage/Loading

```rust
use dbx_core::engine::metadata;

// Save trigger
metadata::save_trigger(&wos, &trigger)?;

// Load trigger
let loaded = metadata::load_trigger(&wos, "audit_trigger")?.unwrap();

// Load all triggers
let all_triggers = metadata::load_all_triggers(&wos)?;
```

### Auto-Registration

Saved triggers are automatically registered when opening the database:

```rust
let db = Database::open("./my_db")?;

// Saved triggers auto-registered
let trigger_count = db.trigger_executor.read().unwrap().list_triggers().len();
println!("Loaded {} triggers", trigger_count);
```

---

## Execution Timing

| Timing | Description | Use Case |
|--------|-------------|----------|
| **BEFORE** | Before data change | Validation, data transformation |
| **AFTER** | After data change | Audit logs, statistics update |

---

## Event Types

| Event | Description |
|-------|-------------|
| **INSERT** | On new row insertion |
| **UPDATE** | On existing row modification |
| **DELETE** | On row deletion |

---

## NEW/OLD References

Reference before/after values inside triggers:

| Reference | Available | Description |
|-----------|-----------|-------------|
| **NEW** | INSERT, UPDATE | New values |
| **OLD** | UPDATE, DELETE | Old values |

```sql
CREATE TRIGGER track_changes
AFTER UPDATE ON products
FOR EACH ROW
BEGIN
    INSERT INTO change_log VALUES (
        OLD.id, OLD.price, NEW.price, NOW()
    );
END;
```

---

## Conditional Execution (WHEN)

```sql
CREATE TRIGGER high_value_alert
AFTER INSERT ON orders
FOR EACH ROW
WHEN (NEW.total > 10000)
BEGIN
    INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER');
END;
```

---

## Limitations

⚠️ **Current Limitations**
- SQL execution logic is planned for future implementation
- Currently supports metadata storage/loading and parsing only

🔧 **Future Improvements**
- Actual trigger body SQL execution
- WHEN condition evaluation
- Transaction support
- Enhanced error handling

---

## Next Steps

- [Stored Procedures Guide](stored-procedures) — Reusable SQL procedures
- [EventHook Guide](event-hook) — Rust closure-based event handling
- [UDF Guide](udf) — User-defined functions
