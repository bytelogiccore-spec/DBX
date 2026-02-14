# dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Guide](https://img.shields.io/badge/guide-GitHub%20Pages-blue)](https://bytelogiccore-spec.github.io/DBX/english/packages/nodejs)

> High-performance Node.js bindings for DBX embedded database

**dbx-native** provides native Node.js bindings to the DBX database engine via N-API, delivering near-zero overhead access to the high-performance Rust core.

## Installation

```bash
npm install dbx-native
```

## Quick Start

```javascript
const dbx = require('dbx-native');

// Open an in-memory database
const db = dbx.openInMemory();

// Insert data
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));

// Get data
const value = db.get('users', Buffer.from('user:1'));
console.log(value.toString()); // Alice

// Delete data
db.delete('users', Buffer.from('user:2'));

// Close database
db.close();
```

## SQL Interface

```javascript
const dbx = require('dbx-native');

const db = dbx.openInMemory();

// Execute SQL
db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");

const result = db.executeSql('SELECT * FROM users');
console.log(result);

db.close();
```

## API Reference

| Method | Description |
|--------|-------------|
| `openInMemory()` | Open an in-memory database |
| `open(path)` | Open a file-based database |
| `insert(table, key, value)` | Insert a key-value pair |
| `get(table, key)` | Get value by key |
| `delete(table, key)` | Delete a key |
| `executeSql(sql)` | Execute a SQL statement |
| `close()` | Close and free resources |

## Benchmarks

```bash
npm run bench
```

## Requirements

- Node.js 18+
- Windows x64 (native addon included)

## License

MIT License — see [LICENSE](https://github.com/bytelogiccore-spec/DBX/blob/main/LICENSE) for details.
