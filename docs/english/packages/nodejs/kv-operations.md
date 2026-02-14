---
layout: default
title: KV Operations
parent: Node.js (dbx-native)
grand_parent: Packages
great_grand_parent: English
nav_order: 4
---

# Key-Value Operations

DBX can be used as a high-performance Key-Value store in addition to SQL.

## Basic CRUD

### Insert

```typescript
import { Database } from 'dbx-native';

const db = Database.openInMemory();

// Basic insert
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));

// JSON data
const user = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));
```

### Get

```typescript
// Single get
const value = db.get('users', Buffer.from('user:1'));
if (value) {
  console.log(value.toString());  // Alice
}

// JSON parsing
const userBuffer = db.get('users', Buffer.from('user:1'));
if (userBuffer) {
  const user = JSON.parse(userBuffer.toString());
  console.log(user.name);
}
```

### Delete

```typescript
db.delete('users', Buffer.from('user:1'));

// Check before delete
if (db.get('users', Buffer.from('user:1'))) {
  db.delete('users', Buffer.from('user:1'));
  console.log('Deleted');
}
```

### Count

```typescript
const count = db.count('users');
console.log(`Total users: ${count}`);
```

## Batch Operations

```typescript
// Bulk insert
for (let i = 0; i < 10000; i++) {
  db.insert('users', Buffer.from(`user:${i}`), Buffer.from(`User ${i}`));
}

// Flush
db.flush();
```

## Practical Examples

### Session Store

```typescript
class SessionStore {
  private db: Database;

  constructor(dbPath: string) {
    this.db = Database.open(dbPath);
  }

  createSession(sessionId: string, data: any, ttlSeconds: number = 3600): void {
    const session = {
      data,
      createdAt: Date.now(),
      expiresAt: Date.now() + (ttlSeconds * 1000)
    };
    this.db.insert('sessions', Buffer.from(sessionId), Buffer.from(JSON.stringify(session)));
  }

  getSession(sessionId: string): any | null {
    const buffer = this.db.get('sessions', Buffer.from(sessionId));
    if (!buffer) return null;

    const session = JSON.parse(buffer.toString());

    // Check expiration
    if (Date.now() > session.expiresAt) {
      this.db.delete('sessions', Buffer.from(sessionId));
      return null;
    }

    return session.data;
  }

  deleteSession(sessionId: string): void {
    this.db.delete('sessions', Buffer.from(sessionId));
  }

  close(): void {
    this.db.close();
  }
}

// Usage
const store = new SessionStore('sessions.db');
store.createSession('sess_abc123', { userId: 42, username: 'alice' }, 3600);
const session = store.getSession('sess_abc123');
if (session) {
  console.log(`User: ${session.username}`);
}
store.close();
```

### Cache System

```typescript
class Cache<T> {
  private db: Database;
  private defaultTtl: number;

  constructor(dbPath: string, defaultTtlSeconds: number = 300) {
    this.db = Database.open(dbPath);
    this.defaultTtl = defaultTtlSeconds;
  }

  set(key: string, value: T, ttlSeconds?: number): void {
    const ttl = ttlSeconds ?? this.defaultTtl;
    const cacheData = {
      value,
      expiresAt: Date.now() + (ttl * 1000)
    };
    this.db.insert('cache', Buffer.from(key), Buffer.from(JSON.stringify(cacheData)));
  }

  get(key: string): T | null {
    const buffer = this.db.get('cache', Buffer.from(key));
    if (!buffer) return null;

    const cacheData = JSON.parse(buffer.toString());

    // Check expiration
    if (Date.now() > cacheData.expiresAt) {
      this.db.delete('cache', Buffer.from(key));
      return null;
    }

    return cacheData.value as T;
  }

  delete(key: string): void {
    this.db.delete('cache', Buffer.from(key));
  }

  close(): void {
    this.db.close();
  }
}

// Usage
interface User {
  name: string;
  email: string;
}

const cache = new Cache<User>('cache.db', 300);
cache.set('user:1', { name: 'Alice', email: 'alice@example.com' });
const user = cache.get('user:1');
if (user) {
  console.log(`Cached user: ${user.name}`);
}
cache.close();
```

## Performance Optimization

### 1. Batch Operations + Flush

```typescript
// ❌ Slow
for (let i = 0; i < 10000; i++) {
  db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
  db.flush();  // Flush every time
}

// ✅ Fast
for (let i = 0; i < 10000; i++) {
  db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
}
db.flush();  // Flush once
```

### 2. Use Transactions

```typescript
const tx = db.beginTransaction();
for (let i = 0; i < 10000; i++) {
  db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
}
tx.commit();
db.flush();
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [Advanced](advanced) - Transactions, encryption
- [API Reference](api-reference) - Complete API
