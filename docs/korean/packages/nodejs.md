---
layout: default
title: Node.js (dbx-native)
parent: Packages
grand_parent: 한국어
nav_order: 4
---

# Node.js — dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)

NAPI-RS로 구축된 DBX 임베디드 데이터베이스용 고성능 Node.js 네이티브 애드온입니다.

## 설치

```bash
npm install dbx-native
# 또는
yarn add dbx-native
# 또는
pnpm add dbx-native
```

## 빠른 시작

```javascript
const { Database } = require('dbx-native');

// 인메모리 데이터베이스 열기
const db = Database.openInMemory();

// 삽입
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));

// 조회
const value = db.get('users', Buffer.from('user:1'));
console.log(value.toString()); // Alice

// 삭제
db.delete('users', Buffer.from('user:2'));

// 개수 확인
const count = db.count('users');
console.log(`전체 사용자: ${count}`);

// 닫기
db.close();
```

## TypeScript 지원

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

## 고급 사용법

### JSON 데이터 다루기

```javascript
const { Database } = require('dbx-native');

const db = Database.openInMemory();

// JSON 데이터 저장
const user = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));

// JSON 데이터 조회
const data = db.get('users', Buffer.from('user:1'));
const retrievedUser = JSON.parse(data.toString());
console.log(retrievedUser.name); // Alice

db.close();
```

### Async/Await 패턴

```javascript
const { Database } = require('dbx-native');

async function main() {
  const db = Database.open('my_database.db');
  
  try {
    // 배치 삽입
    for (let i = 0; i < 1000; i++) {
      const key = Buffer.from(`item:${i}`);
      const value = Buffer.from(`value_${i}`);
      db.insert('items', key, value);
    }
    
    // 디스크에 플러시
    db.flush();
    
    console.log('배치 삽입 완료');
  } finally {
    db.close();
  }
}

main().catch(console.error);
```

### 에러 처리

```javascript
const { Database } = require('dbx-native');

try {
  const db = Database.open('my.db');
  
  db.insert('users', Buffer.from('key1'), Buffer.from('value1'));
  db.flush();
  
  db.close();
} catch (error) {
  console.error('데이터베이스 오류:', error.message);
}
```

### Express.js와 함께 사용

```javascript
const express = require('express');
const { Database } = require('dbx-native');

const app = express();
const db = Database.open('sessions.db');

app.use(express.json());

// 세션 설정
app.post('/session', (req, res) => {
  const { sessionId, data } = req.body;
  db.insert('sessions', 
    Buffer.from(sessionId), 
    Buffer.from(JSON.stringify(data))
  );
  res.json({ success: true });
});

// 세션 조회
app.get('/session/:id', (req, res) => {
  const data = db.get('sessions', Buffer.from(req.params.id));
  if (data) {
    res.json(JSON.parse(data.toString()));
  } else {
    res.status(404).json({ error: '세션을 찾을 수 없습니다' });
  }
});

// 종료 시 정리
process.on('SIGINT', () => {
  db.close();
  process.exit(0);
});

app.listen(3000, () => {
  console.log('서버가 3000 포트에서 실행 중입니다');
});
```

## API 레퍼런스

### Database 클래스

#### 정적 메서드

| 메서드 | 매개변수 | 반환 | 설명 |
|--------|----------|------|------|
| `Database.open(path)` | `path: string` | `Database` | 파일 기반 데이터베이스 열기 |
| `Database.openInMemory()` | - | `Database` | 인메모리 데이터베이스 열기 |

#### 인스턴스 메서드

| 메서드 | 매개변수 | 반환 | 설명 |
|--------|----------|------|------|
| `insert` | `table: string, key: Buffer, value: Buffer` | `void` | 키-값 쌍 삽입 |
| `get` | `table: string, key: Buffer` | `Buffer \| null` | 키로 값 조회 |
| `delete` | `table: string, key: Buffer` | `void` | 키 삭제 |
| `count` | `table: string` | `number` | 테이블 행 개수 |
| `flush` | - | `void` | 디스크에 플러시 |
| `close` | - | `void` | 데이터베이스 닫기 |

## TypeScript 정의

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

## 성능 팁

1. **Buffer 사용**: 인코딩 오버헤드를 피하기 위해 항상 키와 값에 `Buffer` 사용
2. **배치 작업**: `flush()` 호출 전에 여러 삽입 그룹화
3. **연결 풀링**: 요청 간 데이터베이스 인스턴스 재사용
4. **캐시용 인메모리**: 세션 저장소에 인메모리 데이터베이스 사용

## 예제

### 간단한 Key-Value 저장소

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

// 사용법
const store = new KVStore('store.db');
store.set('name', 'Alice');
console.log(store.get('name')); // Alice
store.close();
```

### 세션 매니저

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

// 사용법
const sessions = new SessionManager();
sessions.createSession('sess_123', { userId: 42, role: 'admin' });
console.log(sessions.getSession('sess_123'));
```

### 캐시 래퍼

```javascript
const { Database } = require('dbx-native');

class Cache {
  constructor() {
    this.db = Database.openInMemory();
  }
  
  async wrap(key, fn, ttl = 300) {
    // 캐시 확인
    const cached = this.db.get('cache', Buffer.from(key));
    if (cached) {
      const { data, expires } = JSON.parse(cached.toString());
      if (Date.now() < expires) {
        return data;
      }
    }
    
    // 함수 실행
    const result = await fn();
    
    // 캐시에 저장
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

// 사용법
const cache = new Cache();

async function expensiveOperation() {
  // 비용이 많이 드는 작업 시뮬레이션
  await new Promise(resolve => setTimeout(resolve, 1000));
  return { result: 'data' };
}

(async () => {
  const result = await cache.wrap('my-key', expensiveOperation);
  console.log(result); // 첫 번째 호출: 1초 소요
  
  const cached = await cache.wrap('my-key', expensiveOperation);
  console.log(cached); // 두 번째 호출: 즉시 반환
})();
```

## 요구사항

- Node.js >= 16
- **Windows x64 전용** (Linux/macOS 지원 예정)

## 문제 해결

### 모듈을 찾을 수 없음

```bash
# 네이티브 애드온 재빌드
npm rebuild dbx-native

# 또는 재설치
npm install dbx-native --force
```

### 성능 문제

```javascript
// 배치 작업 사용
const db = Database.open('data.db');

// 나쁨: 매번 삽입 후 플러시
for (let i = 0; i < 10000; i++) {
  db.insert('items', Buffer.from(`k${i}`), Buffer.from(`v${i}`));
  db.flush(); // ❌ 느림
}

// 좋음: 마지막에 한 번만 플러시
for (let i = 0; i < 10000; i++) {
  db.insert('items', Buffer.from(`k${i}`), Buffer.from(`v${i}`));
}
db.flush(); // ✅ 빠름

db.close();
```

### 메모리 누수

```javascript
// 항상 데이터베이스 연결 닫기
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

## 플랫폼 지원

| 플랫폼 | 아키텍처 | 상태 |
|--------|----------|------|
| Windows | x64 | ✅ 테스트 완료 |
| Linux | x64 | ⚠️ 계획됨 |
| macOS | x64 (Intel) | ⚠️ 계획됨 |
| macOS | ARM64 (Apple Silicon) | ⚠️ 계획됨 |

## 라이선스

MIT 또는 Commercial 라이선스로 이중 라이선스됩니다.
