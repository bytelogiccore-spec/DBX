---
layout: default
title: SQL Guide
parent: C/C++ (dbx-ffi)
grand_parent: Packages
great_grand_parent: English
nav_order: 5
---

# SQL Guide

Complete SQL guide for C/C++.

## CREATE TABLE

```c
dbx_execute_sql(db, "CREATE TABLE users (id INTEGER, name TEXT)");
```

## INSERT

```c
dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice')");
```

## SELECT

```c
char* result = dbx_execute_sql(db, "SELECT * FROM users");
printf("%s\n", result);
dbx_free_string(result);
```

## UPDATE

```c
dbx_execute_sql(db, "UPDATE users SET name = 'Bob' WHERE id = 1");
```

## DELETE

```c
dbx_execute_sql(db, "DELETE FROM users WHERE id = 1");
```

## Transactions

```c
DbxTransaction* tx = dbx_begin_transaction(db);
dbx_execute_sql(db, "INSERT INTO users VALUES (1, 'Alice')");
dbx_commit(tx);
```

## Next Steps

- [KV Operations](kv-operations) - Key-Value operations
- [C API](c-api) - C function reference
