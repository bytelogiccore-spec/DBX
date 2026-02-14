---
layout: default
title: Node.js (dbx-native)
parent: Packages
grand_parent: English
nav_order: 4
---

# Node.js — dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)

Native Node.js bindings for DBX via N-API with near-zero overhead.

## Installation

```bash
npm install dbx-native
```

## Quick Start

```javascript
const dbx = require('dbx-native');

// Open in-memory database
const db = dbx.openInMemory();

// Insert
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));

// Get
const value = db.get('users', Buffer.from('user:1'));
console.log(value.toString()); // Alice

// Delete
db.delete('users', Buffer.from('user:2'));

// Close
db.close();
```

## SQL Interface

```javascript
const db = dbx.openInMemory();

db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");

const result = db.executeSql('SELECT * FROM users');
console.log(result);

db.close();
```

## API Reference

| Method | Returns | Description |
|--------|---------|-------------|
| `openInMemory()` | `Database` | Open in-memory database |
| `open(path)` | `Database` | Open file-based database |
| `insert(table, key, value)` | `void` | Insert key-value pair |
| `get(table, key)` | `Buffer` | Get value by key |
| `delete(table, key)` | `void` | Delete key |
| `executeSql(sql)` | `Result` | Execute SQL statement |
| `close()` | `void` | Close database |

## Benchmarks

```bash
cd lang/nodejs
npm run bench
```

## Requirements

- Node.js 18+
- Windows x64
