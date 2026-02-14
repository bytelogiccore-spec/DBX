---
layout: default
title: KV 작업
parent: Node.js (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 3
---

# Key-Value 작업

DBX는 SQL 외에도 고성능 Key-Value 스토어로 사용할 수 있습니다.

## 기본 CRUD

### 삽입 (Insert)

```typescript
import { Database } from 'dbx-py';

const db = Database.openInMemory();

// 기본 삽입
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));

// JSON 데이터
const user = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));

// 바이너리 데이터
db.insert('files', Buffer.from('file:1'), Buffer.from([0x89, 0x50, 0x4E, 0x47]));
```

### 조회 (Get)

```typescript
// 단일 조회
const value = db.get('users', Buffer.from('user:1'));
if (value) {
    console.log(value.toString());  // Alice
}

// JSON 파싱
const userBuffer = db.get('users', Buffer.from('user:1'));
if (userBuffer) {
    const user = JSON.parse(userBuffer.toString());
    console.log(user.name);  // Alice
}
```

### 삭제 (Delete)

```typescript
db.delete('users', Buffer.from('user:1'));

// 존재 확인 후 삭제
if (db.get('users', Buffer.from('user:1'))) {
    db.delete('users', Buffer.from('user:1'));
    console.log('Deleted');
}
```

### 개수 확인 (Count)

```typescript
const count = db.count('users');
console.log(`Total users: ${count}`);
```

## 배치 작업

```typescript
// 대량 삽입
for (let i = 0; i < 10000; i++) {
    const key = Buffer.from(`user:${i}`);
    const value = Buffer.from(`User ${i}`);
    db.insert('users', key, value);
}

// 플러시
db.flush();
```

## 실전 예제

### 세션 저장소

```typescript
import { Database } from 'dbx-py';

interface SessionData {
    userId: number;
    username: string;
    role: string;
}

class SessionStore {
    private db: Database;

    constructor(dbPath: string) {
        this.db = Database.open(dbPath);
    }

    createSession(sessionId: string, data: SessionData, ttlSeconds: number = 3600): void {
        const session = {
            data,
            createdAt: Date.now(),
            expiresAt: Date.now() + (ttlSeconds * 1000)
        };
        
        this.db.insert(
            'sessions',
            Buffer.from(sessionId),
            Buffer.from(JSON.stringify(session))
        );
    }

    getSession(sessionId: string): SessionData | null {
        const buffer = this.db.get('sessions', Buffer.from(sessionId));
        if (!buffer) return null;

        const session = JSON.parse(buffer.toString());

        // 만료 확인
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

// 사용 예제
const store = new SessionStore('sessions.db');

store.createSession('sess_abc123', {
    userId: 42,
    username: 'alice',
    role: 'admin'
}, 3600);

const session = store.getSession('sess_abc123');
if (session) {
    console.log(`User: ${session.username}`);
}

store.deleteSession('sess_abc123');
store.close();
```

### 캐시 시스템

```typescript
class Cache<T> {
    private db: Database;
    private defaultTtl: number;

    constructor(dbPath: string, defaultTtlSeconds: number = 300) {
        this.db = Database.open(dbPath);
        this.defaultTtl = defaultTtlSeconds * 1000;
    }

    set(key: string, value: T, ttlSeconds?: number): void {
        const ttl = ttlSeconds ? ttlSeconds * 1000 : this.defaultTtl;
        const cacheData = {
            value,
            expiresAt: Date.now() + ttl
        };

        this.db.insert(
            'cache',
            Buffer.from(key),
            Buffer.from(JSON.stringify(cacheData))
        );
    }

    get(key: string): T | null {
        const buffer = this.db.get('cache', Buffer.from(key));
        if (!buffer) return null;

        const cacheData = JSON.parse(buffer.toString());

        // 만료 확인
        if (Date.now() > cacheData.expiresAt) {
            this.db.delete('cache', Buffer.from(key));
            return null;
        }

        return cacheData.value;
    }

    delete(key: string): void {
        this.db.delete('cache', Buffer.from(key));
    }

    close(): void {
        this.db.close();
    }
}

// 사용 예제
interface User {
    name: string;
    email: string;
}

const cache = new Cache<User>('cache.db', 300);

cache.set('user:1', { name: 'Alice', email: 'alice@example.com' });

const user = cache.get('user:1');
if (user) {
    console.log(`Cached user: ${user.name}`);
} else {
    console.log('Cache miss');
}

cache.close();
```

## 성능 최적화

### 1. 배치 작업 + 플러시

```typescript
// ❌ 느림
for (let i = 0; i < 10000; i++) {
    db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
    db.flush();  // 매번 플러시
}

// ✅ 빠름
for (let i = 0; i < 10000; i++) {
    db.insert('data', Buffer.from(`key:${i}`), Buffer.from(`value:${i}`));
}
db.flush();  // 한 번만 플러시
```

### 2. Buffer 재사용

```typescript
// ❌ 느림 (매번 Buffer 생성)
for (let i = 0; i < 10000; i++) {
    const key = Buffer.from(`key:${i}`);
    db.insert('data', key, Buffer.from('value'));
}

// ✅ 빠름 (Buffer 재사용)
const keyBuffer = Buffer.allocUnsafe(20);
for (let i = 0; i < 10000; i++) {
    keyBuffer.write(`key:${i}`);
    db.insert('data', keyBuffer, Buffer.from('value'));
}
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화
- [API 레퍼런스](api-reference) - 전체 API
