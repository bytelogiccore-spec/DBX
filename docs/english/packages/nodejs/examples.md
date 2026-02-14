---
layout: default
title: Examples
parent: Node.js (dbx-native)
grand_parent: Packages
great_grand_parent: English
nav_order: 7
---

# Real-World Examples

## Express.js REST API

```typescript
import express from 'express';
import { Database } from 'dbx-native';

const app = express();
const db = Database.open('api.db');

app.use(express.json());

// Initialize schema
db.executeSql('CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT, email TEXT)');

app.post('/users', (req, res) => {
  const { name, email } = req.body;
  const id = Date.now();
  db.executeSql(`INSERT INTO users VALUES (${id}, '${name}', '${email}')`);
  res.json({ id, name, email });
});

app.get('/users/:id', (req, res) => {
  const result = db.executeSql(`SELECT * FROM users WHERE id = ${req.params.id}`);
  res.json(JSON.parse(result));
});

app.listen(3000, () => console.log('Server running on port 3000'));
```

## WebSocket Real-time Chat

```typescript
import { WebSocketServer } from 'ws';
import { Database } from 'dbx-native';

const wss = new WebSocketServer({ port: 8080 });
const db = Database.open('chat.db');

db.executeSql('CREATE TABLE IF NOT EXISTS messages (id INTEGER, user TEXT, message TEXT, timestamp INTEGER)');

wss.on('connection', (ws) => {
  ws.on('message', (data) => {
    const { user, message } = JSON.parse(data.toString());
    const id = Date.now();
    
    db.executeSql(
      `INSERT INTO messages VALUES (${id}, '${user}', '${message}', ${id})`
    );
    
    // Broadcast to all clients
    wss.clients.forEach((client) => {
      client.send(JSON.stringify({ id, user, message, timestamp: id }));
    });
  });
});
```

## Redis-like Cache

```typescript
import { Database } from 'dbx-native';

class RedisLikeCache {
  private db: Database;

  constructor(dbPath: string) {
    this.db = Database.open(dbPath);
  }

  set(key: string, value: string, ttl?: number): void {
    const data = {
      value,
      expiresAt: ttl ? Date.now() + (ttl * 1000) : null
    };
    this.db.insert('cache', Buffer.from(key), Buffer.from(JSON.stringify(data)));
  }

  get(key: string): string | null {
    const buffer = this.db.get('cache', Buffer.from(key));
    if (!buffer) return null;

    const data = JSON.parse(buffer.toString());
    if (data.expiresAt && Date.now() > data.expiresAt) {
      this.db.delete('cache', Buffer.from(key));
      return null;
    }

    return data.value;
  }

  delete(key: string): void {
    this.db.delete('cache', Buffer.from(key));
  }

  close(): void {
    this.db.close();
  }
}

// Usage
const cache = new RedisLikeCache('cache.db');
cache.set('user:1', JSON.stringify({ name: 'Alice' }), 300);
const user = cache.get('user:1');
console.log(user);
```

## Next Steps

- [API Reference](api-reference) - Complete API
- [Advanced](advanced) - Transactions, performance tuning
