---
layout: default
title: Node.js (dbx-native)
parent: Packages
grand_parent: English
nav_order: 4
---

# Node.js — dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)

High-performance Node.js native addon for DBX embedded database built with NAPI-RS.

## Installation

```bash
npm install dbx-native
# or
yarn add dbx-native
# or
pnpm add dbx-native
```

## Quick Start

```javascript
const { Database } = require('dbx-native');

// Open in-memory database
const db = Database.openInMemory();

// Insert
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));

// Get
const value = db.get('users', Buffer.from('user:1'));
console.log(value.toString()); // Alice

// Delete
db.delete('users', Buffer.from('user:2'));

// Count
const count = db.count('users');
console.log(`Total users: ${count}`);

// Close
db.close();
```

## TypeScript Support

```typescript
import { Database } from 'dbx-native';

const db = Database.openInMemory();

db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
const value: Buffer | null = db.get('users', Buffer.from('user:1'));

if (value) {
  console.log(value.toString());
}

db.close();
```

## Advanced Usage

### Working with JSON

```javascript
const { Database } = require('dbx-native');

const db = Database.openInMemory();

// Store JSON data
const user = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));

// Retrieve JSON data
const data = db.get('users', Buffer.from('user:1'));
const retrievedUser = JSON.parse(data.toString());
console.log(retrievedUser.name); // Alice

db.close();
```

### Async/Await Pattern

```javascript
const { Database } = require('dbx-native');

async function main() {
  const db = Database.open('my_database.db');
  
  try {
    // Batch insert
    for (let i = 0; i < 1000; i++) {
      const key = Buffer.from(`item:${i}`);
      const value = Buffer.from(`value_${i}`);
      db.insert('items', key, value);
    }
    
    // Flush to disk
    db.flush();
    
    console.log('Batch insert completed');
  } finally {
    db.close();
  }
}

main().catch(console.error);
```

### Error Handling

```javascript
const { Database } = require('dbx-native');

try {
  const db = Database.open('my.db');
  
  db.insert('users', Buffer.from('key1'), Buffer.from('value1'));
  db.flush();
  
  db.close();
} catch (error) {
  console.error('Database error:', error.message);
}
```

### Using with Express.js

```javascript
const express = require('express');
const { Database } = require('dbx-native');

const app = express();
const db = Database.open('sessions.db');

app.use(express.json());

// Set session
app.post('/session', (req, res) => {
  const { sessionId, data } = req.body;
  db.insert('sessions', 
    Buffer.from(sessionId), 
    Buffer.from(JSON.stringify(data))
  );
  res.json({ success: true });
});

// Get session
app.get('/session/:id', (req, res) => {
  const data = db.get('sessions', Buffer.from(req.params.id));
  if (data) {
    res.json(JSON.parse(data.toString()));
  } else {
    res.status(404).json({ error: 'Session not found' });
  }
});

// Cleanup on exit
process.on('SIGINT', () => {
  db.close();
  process.exit(0);
});

app.listen(3000, () => {
  console.log('Server running on port 3000');
});
```

## API Reference

### Database Class

#### Static Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `Database.open(path)` | `path: string` | `Database` | Opens file-based database |
| `Database.openInMemory()` | - | `Database` | Opens in-memory database |

#### Instance Methods

| Method | Parameters | Returns | Description |
|--------|------------|---------|-------------|
| `insert` | `table: string, key: Buffer, value: Buffer` | `void` | Inserts key-value pair |
| `get` | `table: string, key: Buffer` | `Buffer \| null` | Gets value by key |
| `delete` | `table: string, key: Buffer` | `void` | Deletes key |
| `count` | `table: string` | `number` | Counts rows in table |
| `flush` | - | `void` | Flushes to disk |
| `close` | - | `void` | Closes database |

## TypeScript Definitions

```typescript
export class Database {
  static open(path: string): Database;
  static openInMemory(): Database;
  
  insert(table: string, key: Buffer, value: Buffer): void;
  get(table: string, key: Buffer): Buffer | null;
  delete(table: string, key: Buffer): void;
  count(table: string): number;
  flush(): void;
  close(): void;
}
```

## Performance Tips

1. **Use Buffers**: Always use `Buffer` for keys and values to avoid encoding overhead
2. **Batch Operations**: Group multiple inserts before calling `flush()`
3. **Connection Pooling**: Reuse database instances across requests
4. **In-Memory for Cache**: Use in-memory database for session storage

## Examples

### Simple Key-Value Store

```javascript
const { Database } = require('dbx-native');

class KVStore {
  constructor(path) {
    this.db = Database.open(path);
  }
  
  set(key, value) {
    this.db.insert('kv', Buffer.from(key), Buffer.from(value));
  }
  
  get(key) {
    const data = this.db.get('kv', Buffer.from(key));
    return data ? data.toString() : null;
  }
  
  delete(key) {
    this.db.delete('kv', Buffer.from(key));
  }
  
  close() {
    this.db.close();
  }
}

// Usage
const store = new KVStore('store.db');
store.set('name', 'Alice');
console.log(store.get('name')); // Alice
store.close();
```

### Session Manager

```javascript
const { Database } = require('dbx-native');

class SessionManager {
  constructor() {
    this.db = Database.openInMemory();
  }
  
  createSession(sessionId, data, ttl = 3600) {
    const payload = {
      data,
      expires: Date.now() + (ttl * 1000)
    };
    this.db.insert('sessions', 
      Buffer.from(sessionId), 
      Buffer.from(JSON.stringify(payload))
    );
  }
  
  getSession(sessionId) {
    const raw = this.db.get('sessions', Buffer.from(sessionId));
    if (!raw) return null;
    
    const payload = JSON.parse(raw.toString());
    if (Date.now() > payload.expires) {
      this.db.delete('sessions', Buffer.from(sessionId));
      return null;
    }
    
    return payload.data;
  }
  
  deleteSession(sessionId) {
    this.db.delete('sessions', Buffer.from(sessionId));
  }
  
  close() {
    this.db.close();
  }
}

// Usage
const sessions = new SessionManager();
sessions.createSession('sess_123', { userId: 42, role: 'admin' });
console.log(sessions.getSession('sess_123'));
```

### Cache Wrapper

```javascript
const { Database } = require('dbx-native');

class Cache {
  constructor() {
    this.db = Database.openInMemory();
  }
  
  async wrap(key, fn, ttl = 300) {
    // Check cache
    const cached = this.db.get('cache', Buffer.from(key));
    if (cached) {
      const { data, expires } = JSON.parse(cached.toString());
      if (Date.now() < expires) {
        return data;
      }
    }
    
    // Execute function
    const result = await fn();
    
    // Store in cache
    const payload = {
      data: result,
      expires: Date.now() + (ttl * 1000)
    };
    this.db.insert('cache', 
      Buffer.from(key), 
      Buffer.from(JSON.stringify(payload))
    );
    
    return result;
  }
  
  invalidate(key) {
    this.db.delete('cache', Buffer.from(key));
  }
  
  close() {
    this.db.close();
  }
}

// Usage
const cache = new Cache();

async function expensiveOperation() {
  // Simulate expensive operation
  await new Promise(resolve => setTimeout(resolve, 1000));
  return { result: 'data' };
}

(async () => {
  const result = await cache.wrap('my-key', expensiveOperation);
  console.log(result); // First call: takes 1s
  
  const cached = await cache.wrap('my-key', expensiveOperation);
  console.log(cached); // Second call: instant
})();
```

## Requirements

- Node.js >= 16
- Windows x64 / Linux x64 / macOS (ARM64/x64)

## Troubleshooting

### Module Not Found

```bash
# Rebuild native addon
npm rebuild dbx-native

# Or reinstall
npm install dbx-native --force
```

### Performance Issues

```javascript
// Use batch operations
const db = Database.open('data.db');

// Bad: Flush after every insert
for (let i = 0; i < 10000; i++) {
  db.insert('items', Buffer.from(`k${i}`), Buffer.from(`v${i}`));
  db.flush(); // ❌ Slow
}

// Good: Flush once at the end
for (let i = 0; i < 10000; i++) {
  db.insert('items', Buffer.from(`k${i}`), Buffer.from(`v${i}`));
}
db.flush(); // ✅ Fast

db.close();
```

### Memory Leaks

```javascript
// Always close database connections
const db = Database.open('my.db');

process.on('SIGINT', () => {
  db.close();
  process.exit(0);
});

process.on('SIGTERM', () => {
  db.close();
  process.exit(0);
});
```

## Platform Support

| Platform | Architecture | Status |
|----------|--------------|--------|
| Windows | x64 | ✅ Supported |
| Linux | x64 | ✅ Supported |
| macOS | x64 (Intel) | ✅ Supported |
| macOS | ARM64 (Apple Silicon) | ✅ Supported |

## License

Dual-licensed under MIT or Commercial license.
