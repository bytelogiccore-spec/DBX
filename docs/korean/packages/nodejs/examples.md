---
layout: default
title: 실전 예제
parent: Node.js (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 7
---

# 실전 예제

## Express.js REST API

```typescript
import express from 'express';
import { Database } from 'dbx-py';

const app = express();
const db = Database.open('api.db');

app.use(express.json());

// 스키마 초기화
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

## WebSocket 실시간 채팅

```typescript
import { WebSocketServer } from 'ws';
import { Database } from 'dbx-py';

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
    
    // 모든 클라이언트에 브로드캐스트
    wss.clients.forEach((client) => {
      client.send(JSON.stringify({ id, user, message, timestamp: id }));
    });
  });
});
```

## Redis 대체 캐시

```typescript
import { Database } from 'dbx-py';

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

// 사용
const cache = new RedisLikeCache('cache.db');
cache.set('user:1', JSON.stringify({ name: 'Alice' }), 300);
const user = cache.get('user:1');
console.log(user);
```

## 다음 단계

- [API 레퍼런스](api-reference) - 전체 API
- [고급 기능](advanced) - 트랜잭션, 성능 튜닝
