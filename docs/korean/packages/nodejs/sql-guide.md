---
layout: default
title: SQL 가이드
parent: Node.js (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 4
---

# SQL 가이드

DBX는 표준 SQL을 지원합니다. Node.js에서 동기/비동기 모두 사용 가능합니다.

## 테이블 생성 (CREATE TABLE)

```typescript
import { Database } from 'dbx-py';

const db = Database.open('mydb.db');

// 기본 테이블
db.executeSql(`
  CREATE TABLE users (
    id INTEGER,
    name TEXT,
    email TEXT,
    age INTEGER
  )
`);

// Primary Key 지정
db.executeSql(`
  CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    price REAL
  )
`);
```

## 데이터 삽입 (INSERT)

### 단일 행 삽입

```typescript
// 기본 INSERT
db.executeSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// 컬럼 명시
db.executeSql(`
  INSERT INTO users (id, name, email) 
  VALUES (2, 'Bob', 'bob@example.com')
`);
```

### 다중 행 삽입

```typescript
// 배치 삽입
const users = [
  { id: 1, name: 'Alice', email: 'alice@example.com', age: 25 },
  { id: 2, name: 'Bob', email: 'bob@example.com', age: 30 },
  { id: 3, name: 'Carol', email: 'carol@example.com', age: 28 }
];

for (const user of users) {
  db.executeSql(
    `INSERT INTO users VALUES (${user.id}, '${user.name}', '${user.email}', ${user.age})`
  );
}
```

## 데이터 조회 (SELECT)

### 기본 조회

```typescript
// 전체 조회
const result = db.executeSql('SELECT * FROM users');
console.log(result);

// 특정 컬럼
const names = db.executeSql('SELECT name, email FROM users');

// WHERE 조건
const adults = db.executeSql('SELECT * FROM users WHERE age >= 18');
```

### 정렬 및 제한

```typescript
// ORDER BY
const sorted = db.executeSql('SELECT * FROM users ORDER BY age DESC');

// LIMIT
const top10 = db.executeSql('SELECT * FROM users LIMIT 10');

// OFFSET
const page2 = db.executeSql('SELECT * FROM users LIMIT 10 OFFSET 10');
```

### 집계 함수

```typescript
// COUNT
const count = db.executeSql('SELECT COUNT(*) FROM users');

// AVG, SUM, MIN, MAX
const stats = db.executeSql(`
  SELECT 
    AVG(age) as avg_age,
    MIN(age) as min_age,
    MAX(age) as max_age
  FROM users
`);

// GROUP BY
const ageGroups = db.executeSql(`
  SELECT age, COUNT(*) as count
  FROM users
  GROUP BY age
`);
```

## 데이터 수정 (UPDATE)

```typescript
// 단일 컬럼 수정
db.executeSql("UPDATE users SET age = 26 WHERE id = 1");

// 다중 컬럼 수정
db.executeSql(`
  UPDATE users 
  SET name = 'Alice Smith', email = 'alice.smith@example.com'
  WHERE id = 1
`);

// 조건부 수정
db.executeSql("UPDATE users SET age = age + 1 WHERE age < 30");
```

## 데이터 삭제 (DELETE)

```typescript
// 특정 행 삭제
db.executeSql('DELETE FROM users WHERE id = 1');

// 조건부 삭제
db.executeSql('DELETE FROM users WHERE age < 18');

// 전체 삭제 (주의!)
db.executeSql('DELETE FROM users');
```

## 트랜잭션과 함께 사용

```typescript
const tx = db.beginTransaction();

try {
  db.executeSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");
  db.executeSql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)");
  
  tx.commit();
} catch (error) {
  tx.rollback();
  console.error('Transaction failed:', error);
}
```

## TypeScript와 함께 사용

```typescript
interface User {
  id: number;
  name: string;
  email: string;
  age: number;
}

class UserRepository {
  constructor(private db: Database) {
    this.initSchema();
  }

  private initSchema(): void {
    this.db.executeSql(`
      CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT NOT NULL,
        age INTEGER
      )
    `);
  }

  createUser(user: Omit<User, 'id'>): number {
    const id = Date.now();
    this.db.executeSql(
      `INSERT INTO users (id, name, email, age) 
       VALUES (${id}, '${user.name}', '${user.email}', ${user.age})`
    );
    return id;
  }

  getUser(id: number): string {
    return this.db.executeSql(`SELECT * FROM users WHERE id = ${id}`);
  }

  updateUser(id: number, updates: Partial<User>): void {
    const setClauses: string[] = [];
    
    if (updates.name) setClauses.push(`name = '${updates.name}'`);
    if (updates.email) setClauses.push(`email = '${updates.email}'`);
    if (updates.age) setClauses.push(`age = ${updates.age}`);
    
    if (setClauses.length > 0) {
      this.db.executeSql(
        `UPDATE users SET ${setClauses.join(', ')} WHERE id = ${id}`
      );
    }
  }

  deleteUser(id: number): void {
    this.db.executeSql(`DELETE FROM users WHERE id = ${id}`);
  }

  listUsers(limit: number = 100): string {
    return this.db.executeSql(`SELECT * FROM users LIMIT ${limit}`);
  }
}

// 사용 예제
const db = Database.open('users.db');
const repo = new UserRepository(db);

const userId = repo.createUser({
  name: 'Alice',
  email: 'alice@example.com',
  age: 25
});

console.log('Created user:', userId);

const user = repo.getUser(userId);
console.log('User:', user);

repo.updateUser(userId, { age: 26 });

const users = repo.listUsers();
console.log('All users:', users);

db.close();
```

## Express.js 통합

```typescript
import express from 'express';
import { Database } from 'dbx-py';

const app = express();
const db = Database.open('api.db');

// 스키마 초기화
db.executeSql(`
  CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL
  )
`);

app.use(express.json());

// 사용자 생성
app.post('/users', (req, res) => {
  const { name, email } = req.body;
  const id = Date.now();
  
  try {
    db.executeSql(
      `INSERT INTO users (id, name, email) VALUES (${id}, '${name}', '${email}')`
    );
    res.json({ id, name, email });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// 사용자 조회
app.get('/users/:id', (req, res) => {
  const { id } = req.params;
  
  try {
    const result = db.executeSql(`SELECT * FROM users WHERE id = ${id}`);
    res.json(JSON.parse(result));
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// 사용자 목록
app.get('/users', (req, res) => {
  const limit = parseInt(req.query.limit as string) || 100;
  
  try {
    const result = db.executeSql(`SELECT * FROM users LIMIT ${limit}`);
    res.json(JSON.parse(result));
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// 사용자 수정
app.put('/users/:id', (req, res) => {
  const { id } = req.params;
  const { name, email } = req.body;
  
  try {
    db.executeSql(
      `UPDATE users SET name = '${name}', email = '${email}' WHERE id = ${id}`
    );
    res.json({ id, name, email });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

// 사용자 삭제
app.delete('/users/:id', (req, res) => {
  const { id } = req.params;
  
  try {
    db.executeSql(`DELETE FROM users WHERE id = ${id}`);
    res.json({ success: true });
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.listen(3000, () => {
  console.log('Server running on http://localhost:3000');
});

// Graceful shutdown
process.on('SIGINT', () => {
  db.close();
  process.exit(0);
});
```

## 성능 팁

### 1. 배치 작업

```typescript
// ❌ 느림
for (let i = 0; i < 1000; i++) {
  db.executeSql(`INSERT INTO users VALUES (${i}, 'User${i}', 'user${i}@example.com', 25)`);
}

// ✅ 빠름 (트랜잭션 사용)
const tx = db.beginTransaction();
for (let i = 0; i < 1000; i++) {
  db.executeSql(`INSERT INTO users VALUES (${i}, 'User${i}', 'user${i}@example.com', 25)`);
}
tx.commit();
```

### 2. Prepared Statements (향후 지원 예정)

현재는 문자열 보간을 사용하지만, SQL Injection 방지를 위해 입력값을 검증하세요.

```typescript
// 입력값 검증
function sanitize(input: string): string {
  return input.replace(/'/g, "''");
}

const name = sanitize(userInput);
db.executeSql(`INSERT INTO users (name) VALUES ('${name}')`);
```

## 제한사항

- **JOIN**: 현재 미지원 (향후 지원 예정)
- **서브쿼리**: 제한적 지원
- **외래 키**: 현재 미지원

## 다음 단계

- [고급 기능](advanced) - 트랜잭션, 암호화
- [TypeScript](typescript) - 타입 정의
- [API 레퍼런스](api-reference) - 전체 API
