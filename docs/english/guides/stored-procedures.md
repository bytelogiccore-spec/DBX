---
layout: default
title: Stored Procedures
parent: English
nav_order: 33
---

# Stored Procedures
{: .no_toc }

Define and call reusable SQL procedures.
{: .fs-6 .fw-300 }

---

## Table of Contents
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## Overview

Stored Procedures allow you to bundle multiple SQL statements into a single logical unit for reuse.

### Key Features

- ✅ Parameter support
- ✅ Multiple SQL statement execution
- ✅ Metadata persistence
- ✅ Auto-registration on database restart

---

## CREATE PROCEDURE

### Basic Syntax

```sql
CREATE PROCEDURE procedure_name (param1 TYPE, param2 TYPE, ...)
BEGIN
    sql_statement;
    ...
END;
```

### Example: Update Balance

```sql
CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
BEGIN
    UPDATE accounts SET balance = balance + amount WHERE id = user_id;
    INSERT INTO transactions VALUES (user_id, amount, NOW());
END;
```

### Example: Parameterless Procedure

```sql
CREATE PROCEDURE reset_stats ()
BEGIN
    UPDATE stats SET count = 0;
    DELETE FROM temp_data;
END;
```

---

## CALL PROCEDURE

```sql
CALL update_balance(123, 100.50);
CALL reset_stats();
```

---

## DROP PROCEDURE

```sql
DROP PROCEDURE procedure_name;
```

---

## Rust API

### Parsing Procedures

```rust
use dbx_core::automation::{
    parse_create_procedure, 
    parse_drop_procedure, 
    parse_call_procedure
};

// Parse CREATE PROCEDURE
let sql = r#"
    CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
    BEGIN
        UPDATE accounts SET balance = balance + amount WHERE id = user_id;
        INSERT INTO transactions VALUES (user_id, amount);
    END;
"#;

let proc = parse_create_procedure(sql)?;
println!("Procedure: {}", proc.name);
println!("Parameters: {}", proc.parameters.len());

// Parse CALL PROCEDURE
let call_sql = "CALL update_balance(123, 100.50);";
let (name, args) = parse_call_procedure(call_sql)?;
```

### Metadata Storage/Loading

```rust
use dbx_core::engine::metadata;

// Save procedure
metadata::save_procedure(&wos, &proc)?;

// Load procedure
let loaded = metadata::load_procedure(&wos, "update_balance")?.unwrap();

// Load all procedures
let all_procedures = metadata::load_all_procedures(&wos)?;
```

### Execution

```rust
// Execute procedure
db.procedure_executor.read().unwrap().execute(
    &db,
    "update_balance",
    &["123".to_string(), "100.50".to_string()],
)?;
```

---

## Parameters

### Parameter Definition

```rust
pub struct ProcedureParameter {
    pub name: String,
    pub data_type: String,
}
```

### Example

```sql
CREATE PROCEDURE transfer_funds (
    from_id INT,
    to_id INT,
    amount DECIMAL
)
BEGIN
    UPDATE accounts SET balance = balance - amount WHERE id = from_id;
    UPDATE accounts SET balance = balance + amount WHERE id = to_id;
    INSERT INTO transfers VALUES (from_id, to_id, amount, NOW());
END;
```

---

## Practical Examples

### Example 1: Data Cleanup

```sql
CREATE PROCEDURE cleanup_old_data (days_old INT)
BEGIN
    DELETE FROM logs WHERE created_at < NOW() - INTERVAL days_old DAY;
    DELETE FROM temp_files WHERE created_at < NOW() - INTERVAL days_old DAY;
    UPDATE stats SET last_cleanup = NOW();
END;
```

### Example 2: Statistics Refresh

```sql
CREATE PROCEDURE refresh_statistics ()
BEGIN
    UPDATE product_stats SET 
        total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id),
        avg_price = (SELECT AVG(price) FROM orders WHERE product_id = products.id);
    UPDATE global_stats SET last_updated = NOW();
END;
```

---

## Limitations

⚠️ **Current Limitations**
- SQL execution logic is planned for future implementation
- Currently supports metadata storage/loading and parsing only
- Parameter type validation not supported

🔧 **Future Improvements**
- Actual procedure body SQL execution
- Parameter type validation
- Return value support
- Transaction support

---

## Next Steps

- [SQL Triggers Guide](triggers) — Event-based automatic execution
- [UDF Guide](udf) — User-defined functions
- [Transactions Guide](transactions) — Transaction management
