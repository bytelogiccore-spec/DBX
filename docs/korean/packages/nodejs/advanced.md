---
layout: default
title: 고급 기능
parent: Node.js (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능
{: .no_toc }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

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

---

## SQL Trigger

SQL 표준 문법으로 데이터 변경 시 자동 실행되는 로직을 정의합니다.

### CREATE TRIGGER

```typescript
// 감사 로그 Trigger
db.executeSql(`
  CREATE TRIGGER audit_trigger
  AFTER INSERT ON users
  FOR EACH ROW
  BEGIN
    INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', datetime('now'));
  END;
`);

// 데이터 검증 Trigger (WHEN 조건 사용)
db.executeSql(`
  CREATE TRIGGER validate_price
  BEFORE UPDATE ON products
  FOR EACH ROW
  WHEN (NEW.price < 0)
  BEGIN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '가격은 0 이상이어야 합니다';
  END;
`);
```

### DROP TRIGGER

```typescript
db.executeSql('DROP TRIGGER audit_trigger;');
```

### 실전 예제

#### 변경 이력 추적

```typescript
// 변경 이력 테이블 생성
db.executeSql(`
  CREATE TABLE change_log (
    id INT,
    old_price DECIMAL,
    new_price DECIMAL,
    changed_at TIMESTAMP
  )
`);

// UPDATE 시 변경 이력 기록
db.executeSql(`
  CREATE TRIGGER track_price_changes
  AFTER UPDATE ON products
  FOR EACH ROW
  BEGIN
    INSERT INTO change_log VALUES (
      OLD.id, OLD.price, NEW.price, datetime('now')
    );
  END;
`);
```

#### 조건부 알림

```typescript
// 고액 주문 알림
db.executeSql(`
  CREATE TRIGGER high_value_alert
  AFTER INSERT ON orders
  FOR EACH ROW
  WHEN (NEW.total > 10000)
  BEGIN
    INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER', datetime('now'));
  END;
`);
```

---

## Stored Procedure

재사용 가능한 SQL 프로시저를 정의하고 호출합니다.

### CREATE PROCEDURE

```typescript
// 잔액 업데이트 프로시저
db.executeSql(`
  CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
  BEGIN
    UPDATE accounts SET balance = balance + amount WHERE id = user_id;
    INSERT INTO transactions VALUES (user_id, amount, datetime('now'));
  END;
`);

// 데이터 정리 프로시저
db.executeSql(`
  CREATE PROCEDURE cleanup_old_data (days_old INT)
  BEGIN
    DELETE FROM logs WHERE created_at < datetime('now', '-' || days_old || ' days');
    DELETE FROM temp_files WHERE created_at < datetime('now', '-' || days_old || ' days');
    UPDATE stats SET last_cleanup = datetime('now');
  END;
`);
```

### CALL PROCEDURE

```typescript
// 프로시저 호출
db.executeSql('CALL update_balance(123, 100.50);');
db.executeSql('CALL cleanup_old_data(30);');
```

### DROP PROCEDURE

```typescript
db.executeSql('DROP PROCEDURE update_balance;');
```

---

## Scheduler

Cron 표현식 기반으로 주기적인 작업을 자동 실행합니다.

### CREATE SCHEDULE

```typescript
// 매 5분마다 통계 갱신
db.executeSql(`
  CREATE SCHEDULE refresh_stats
  CRON '*/5 * * * *'
  BEGIN
    UPDATE product_stats SET 
      total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id);
  END;
`);

// 매일 자정에 정리 작업
db.executeSql(`
  CREATE SCHEDULE daily_cleanup
  CRON '0 0 * * *'
  BEGIN
    CALL cleanup_old_data(30);
  END;
`);
```

### Cron 표현식

| 표현식 | 설명 |
|--------|------|
| `*/5 * * * *` | 5분마다 |
| `0 * * * *` | 매시 정각 |
| `0 0 * * *` | 매일 자정 |
| `0 0 * * 0` | 매주 일요일 자정 |
| `0 0 1 * *` | 매월 1일 자정 |

형식: `분 시 일 월 요일`

### DROP SCHEDULE

```typescript
db.executeSql('DROP SCHEDULE refresh_stats;');
```

---

## UDF (사용자 정의 함수)

### CREATE FUNCTION (SQL)

```typescript
// SQL 표준 문법으로 함수 정의
db.executeSql(`
  CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
  BEGIN
    RETURN a + b;
  END;
`);
```

> **참고**: 현재는 메타데이터 파싱만 지원하며, 실제 함수 로직은 TypeScript 코드로 등록해야 합니다.

### TypeScript 함수 등록

```typescript
// 스칼라 UDF 등록
db.registerScalarUdf('double', (x: number) => x * 2);

// SQL에서 사용
const results = db.executeSql('SELECT double(price) FROM products');
```

### 집계 UDF

```typescript
// 중앙값 계산
db.registerAggregateUdf(
  'median',
  () => [] as number[],                    // 초기 상태
  (state, value) => { state.push(value); return state; },  // 축적
  (state) => {                             // 최종 계산
    const sorted = state.sort((a, b) => a - b);
    return sorted[Math.floor(sorted.length / 2)];
  }
);

// SQL에서 사용
const results = db.executeSql('SELECT median(score) FROM students');
```

---

## Event Hook (Rust 클로저 기반)

TypeScript에서는 SQL Trigger를 사용하는 것을 권장하지만, 향후 TypeScript 콜백 지원 예정입니다.

```typescript
// 향후 지원 예정
// db.registerEventHook('users', 'after_insert', (event) => {
//   console.log(`새 사용자: ${JSON.stringify(event.newValues)}`);
// });
```

---

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

---

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

---

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

---

## 쿼리 플랜 캐시

```typescript
db.enableFeature('query_plan_cache');

// 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for (let i = 0; i < 100; i++) {
  const results = db.executeSql('SELECT * FROM users WHERE age > 20');
}
```

---

## 스키마 버저닝

```typescript
db.executeSql('CREATE TABLE users (id INT, name TEXT)');       // v1
db.executeSql('ALTER TABLE users ADD COLUMN email TEXT');       // v2

const version = db.schemaVersion('users');  // → 2
```

---

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
