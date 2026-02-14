---
layout: default
title: Advanced
parent: Node.js (dbx-native)
grand_parent: Packages
great_grand_parent: English
nav_order: 5
---

# Advanced Features

## Transactions

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

## Performance Tuning

### Batch Operations

```typescript
const tx = db.beginTransaction();
for (let i = 0; i < 10000; i++) {
  db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
}
tx.commit();
db.flush();
```

### Buffer Reuse

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

## Next Steps

- [Examples](examples) - More examples
- [API Reference](api-reference) - Complete API
