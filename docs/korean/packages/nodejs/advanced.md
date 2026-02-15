---
layout: default
title: 고급 기능
parent: Node.js (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능

## 트랜잭션

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

## 기능 플래그

```typescript
// 런타임에 기능 활성화/비활성화
db.enableFeature('parallel_query');
db.enableFeature('query_plan_cache');
db.disableFeature('parallel_query');

if (db.isFeatureEnabled('parallel_query')) {
  console.log('병렬 쿼리 활성화됨');
}
```

## 쿼리 플랜 캐시

```typescript
db.enableFeature('query_plan_cache');

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (let i = 0; i < 100; i++) {
  const results = db.executeSql('SELECT * FROM users WHERE age > 20');
}
```

## 스키마 버저닝

```typescript
db.executeSql('CREATE TABLE users (id INT, name TEXT)');       // v1
db.executeSql('ALTER TABLE users ADD COLUMN email TEXT');       // v2

const version = db.schemaVersion('users');  // → 2
```

## UDF (사용자 정의 함수)

```typescript
// 스칼라 UDF 등록
db.registerScalarUdf('double', (x: number) => x * 2);

// SQL에서 사용
const results = db.executeSql('SELECT double(price) FROM products');
```

## 트리거

```typescript
db.registerTrigger('users', 'after_insert', (event) => {
  console.log(`새 사용자: ${JSON.stringify(event.newValues)}`);
});
```

## 스케줄러

```typescript
db.scheduleJob('cleanup', '0 0 * * *', () => {
  db.executeSql('DELETE FROM sessions WHERE expired = 1');
});
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
