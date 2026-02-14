---
layout: default
title: API 레퍼런스
parent: Node.js (dbx-native)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 6
---

# API 레퍼런스

## Database 클래스

### 생성자

#### `Database.open(path: string): Database`

파일 기반 데이터베이스를 엽니다.

**매개변수:**
- `path` (string): 데이터베이스 파일 경로

**반환:** `Database` 인스턴스

**예제:**
```typescript
const db = Database.open('mydb.db');
```

#### `Database.openInMemory(): Database`

인메모리 데이터베이스를 엽니다.

**반환:** `Database` 인스턴스

**예제:**
```typescript
const db = Database.openInMemory();
```

### Key-Value 메서드

#### `insert(table: string, key: Buffer, value: Buffer): void`

키-값 쌍을 삽입합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (Buffer): 키
- `value` (Buffer): 값

**예제:**
```typescript
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
```

#### `get(table: string, key: Buffer): Buffer | null`

키로 값을 조회합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (Buffer): 키

**반환:** 값 (Buffer) 또는 null

**예제:**
```typescript
const value = db.get('users', Buffer.from('user:1'));
if (value) {
    console.log(value.toString());
}
```

#### `delete(table: string, key: Buffer): void`

키를 삭제합니다.

**매개변수:**
- `table` (string): 테이블 이름
- `key` (Buffer): 키

**예제:**
```typescript
db.delete('users', Buffer.from('user:1'));
```

#### `count(table: string): number`

테이블의 행 개수를 반환합니다.

**매개변수:**
- `table` (string): 테이블 이름

**반환:** 행 개수 (number)

**예제:**
```typescript
const count = db.count('users');
console.log(`Total: ${count}`);
```

### SQL 메서드

#### `executeSql(sql: string): string`

SQL 문을 실행합니다.

**매개변수:**
- `sql` (string): SQL 문

**반환:** 결과 (문자열, JSON 형식)

**예제:**
```typescript
// DDL
db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');

// DML
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");

// 조회
const result = db.executeSql('SELECT * FROM users');
console.log(result);
```

### 트랜잭션 메서드

#### `beginTransaction(): Transaction`

트랜잭션을 시작합니다.

**반환:** `Transaction` 객체

**예제:**
```typescript
const tx = db.beginTransaction();
try {
    db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
    tx.commit();
} catch (error) {
    tx.rollback();
}
```

### 유틸리티 메서드

#### `flush(): void`

버퍼를 디스크에 플러시합니다.

**예제:**
```typescript
db.flush();
```

#### `close(): void`

데이터베이스를 닫습니다.

**예제:**
```typescript
db.close();
```

## Transaction 클래스

### 메서드

#### `commit(): void`

트랜잭션을 커밋합니다.

**예제:**
```typescript
const tx = db.beginTransaction();
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
tx.commit();
```

#### `rollback(): void`

트랜잭션을 롤백합니다.

**예제:**
```typescript
const tx = db.beginTransaction();
try {
    db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
    tx.commit();
} catch (error) {
    tx.rollback();
}
```

## TypeScript 타입 정의

```typescript
declare module 'dbx-native' {
    export class Database {
        static open(path: string): Database;
        static openInMemory(): Database;
        
        insert(table: string, key: Buffer, value: Buffer): void;
        get(table: string, key: Buffer): Buffer | null;
        delete(table: string, key: Buffer): void;
        count(table: string): number;
        
        executeSql(sql: string): string;
        
        beginTransaction(): Transaction;
        
        flush(): void;
        close(): void;
    }
    
    export class Transaction {
        commit(): void;
        rollback(): void;
    }
}
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [실전 예제](examples) - 실무 활용 예제
