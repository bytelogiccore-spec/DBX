---
layout: default
title: 빠른 시작
parent: Node.js (dbx-native)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 2
---

# 빠른 시작

5분 안에 DBX를 시작해보세요!

## 설치

```bash
npm install dbx-native
```

## 첫 번째 프로그램

```typescript
import { Database } from 'dbx-native';

// 인메모리 데이터베이스
const db = Database.openInMemory();

// KV 작업
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
const value = db.get('users', Buffer.from('user:1'));
console.log(value?.toString());  // Alice

// SQL 작업
db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");
const result = db.executeSql('SELECT * FROM users');
console.log(result);

db.close();
```

## TypeScript 사용

```typescript
interface User {
  id: number;
  name: string;
  email: string;
}

const db = Database.open('mydb.db');

// JSON 저장
const user: User = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));

// JSON 조회
const data = db.get('users', Buffer.from('user:1'));
if (data) {
  const user: User = JSON.parse(data.toString());
  console.log(user.name);
}

db.close();
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [API 레퍼런스](api-reference) - 전체 API
