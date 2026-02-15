---
layout: default
title: Advanced
parent: Node.js (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 5
---

# Advanced Features

## Transactions

```typescript
import { Database } from 'dbx-py';

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
    const { Database } = require('dbx-py');
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

## Feature Flags

```typescript
db.enableFeature('parallel_query');
db.enableFeature('query_plan_cache');
db.disableFeature('parallel_query');

if (db.isFeatureEnabled('parallel_query')) {
  console.log('Parallel query enabled');
}
```

## Query Plan Cache

```typescript
db.enableFeature('query_plan_cache');

// Repeated queries skip parsing (7.3x faster)
for (let i = 0; i < 100; i++) {
  const results = db.executeSql('SELECT * FROM users WHERE age > 20');
}
```

## Schema Versioning

```typescript
db.executeSql('CREATE TABLE users (id INT, name TEXT)');       // v1
db.executeSql('ALTER TABLE users ADD COLUMN email TEXT');       // v2

const version = db.schemaVersion('users');  // → 2
```

## UDF (User-Defined Functions)

```typescript
db.registerScalarUdf('double', (x: number) => x * 2);
const results = db.executeSql('SELECT double(price) FROM products');
```

## Triggers

```typescript
db.registerTrigger('users', 'after_insert', (event) => {
  console.log(`New user: ${JSON.stringify(event.newValues)}`);
});
```

## Scheduler

```typescript
db.scheduleJob('cleanup', '0 0 * * *', () => {
  db.executeSql('DELETE FROM sessions WHERE expired = 1');
});
```

## Next Steps

- [Examples](examples) - More examples
- [API Reference](api-reference) - Complete API
