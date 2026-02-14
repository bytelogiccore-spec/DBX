---
layout: default
title: 고급 기능
parent: Node.js (dbx-native)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능

## 트랜잭션

```typescript
import { Database } from 'dbx-native';

const db = Database.open('mydb.db');

const tx = db.beginTransaction();
try {
  db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
  db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));
  tx.commit();
} catch (error) {
  tx.rollback();
  console.error('Transaction failed:', error);
}
```

## 성능 튜닝

### 배치 작업

```typescript
const tx = db.beginTransaction();
for (let i = 0; i < 10000; i++) {
  db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
}
tx.commit();
db.flush();
```

### Buffer 재사용

```typescript
const keyBuffer = Buffer.allocUnsafe(20);
for (let i = 0; i < 10000; i++) {
  keyBuffer.write(`key:${i}`);
  db.insert('data', keyBuffer, Buffer.from('value'));
}
```

## Worker Threads

```typescript
import { Worker } from 'worker_threads';

function createWorker(dbPath: string, workerId: number) {
  return new Worker(`
    const { Database } = require('dbx-native');
    const db = Database.open('${dbPath}');
    for (let i = 0; i < 1000; i++) {
      db.insert('data', 
        Buffer.from('worker:${workerId}:key:' + i),
        Buffer.from('value:' + i));
    }
    db.close();
  `, { eval: true });
}

const workers = [];
for (let i = 0; i < 4; i++) {
  workers.push(createWorker('mydb.db', i));
}

await Promise.all(workers.map(w => new Promise(resolve => w.on('exit', resolve))));
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
