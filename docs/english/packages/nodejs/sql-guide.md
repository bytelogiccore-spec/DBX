---
layout: default
title: SQL Guide
parent: Node.js (dbx-native)
grand_parent: Packages
great_grand_parent: English
nav_order: 3
---

# SQL Guide

DBX supports standard SQL. You can use it via the `executeSql` method in Node.js.

## CREATE TABLE

```typescript
import { Database } from 'dbx-native';

const db = Database.open('mydb.db');

// Basic table
db.executeSql(`
  CREATE TABLE users (
    id INTEGER,
    name TEXT,
    email TEXT,
    age INTEGER
  )
`);

// With PRIMARY KEY
db.executeSql(`
  CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    price REAL
  )
`);
```

## INSERT

```typescript
// Basic INSERT
db.executeSql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)");

// Specify columns
db.executeSql(`
  INSERT INTO users (id, name, email) 
  VALUES (2, 'Bob', 'bob@example.com')
`);

// Multiple rows
const users = [
  [1, 'Alice', 'alice@example.com', 25],
  [2, 'Bob', 'bob@example.com', 30],
  [3, 'Carol', 'carol@example.com', 28]
];

users.forEach(([id, name, email, age]) => {
  db.executeSql(`INSERT INTO users VALUES (${id}, '${name}', '${email}', ${age})`);
});
```

## SELECT

```typescript
// All rows
const result = db.executeSql('SELECT * FROM users');
console.log(result);

// WHERE clause
const adults = db.executeSql('SELECT * FROM users WHERE age >= 18');

// ORDER BY
const sorted = db.executeSql('SELECT * FROM users ORDER BY age DESC');

// LIMIT
const top10 = db.executeSql('SELECT * FROM users LIMIT 10');

// Aggregation
const count = db.executeSql('SELECT COUNT(*) FROM users');
const stats = db.executeSql('SELECT AVG(age), MIN(age), MAX(age) FROM users');
```

## UPDATE

```typescript
// Update single column
db.executeSql("UPDATE users SET age = 26 WHERE id = 1");

// Update multiple columns
db.executeSql(`
  UPDATE users 
  SET name = 'Alice Smith', email = 'alice.smith@example.com'
  WHERE id = 1
`);
```

## DELETE

```typescript
// Delete specific row
db.executeSql('DELETE FROM users WHERE id = 1');

// Delete with condition
db.executeSql('DELETE FROM users WHERE age < 18');
```

## Transactions

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

## Practical Example

```typescript
class UserManager {
  private db: Database;

  constructor(dbPath: string) {
    this.db = Database.open(dbPath);
    this.initSchema();
  }

  private initSchema(): void {
    this.db.executeSql(`
      CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY,
        username TEXT NOT NULL,
        email TEXT NOT NULL,
        created_at INTEGER
      )
    `);
  }

  createUser(username: string, email: string): number {
    const id = Date.now();
    this.db.executeSql(
      `INSERT INTO users VALUES (${id}, '${username}', '${email}', ${id})`
    );
    return id;
  }

  getUser(userId: number): string {
    return this.db.executeSql(`SELECT * FROM users WHERE id = ${userId}`);
  }

  listUsers(limit: number = 100): string {
    return this.db.executeSql(`SELECT * FROM users LIMIT ${limit}`);
  }

  close(): void {
    this.db.close();
  }
}

// Usage
const mgr = new UserManager('users.db');
const userId = mgr.createUser('alice', 'alice@example.com');
console.log(`Created user: ${userId}`);
const user = mgr.getUser(userId);
console.log(`User: ${user}`);
mgr.close();
```

## Performance Tips

### 1. Use Transactions for Batch Operations

```typescript
// ❌ Slow
for (let i = 0; i < 1000; i++) {
  db.executeSql(`INSERT INTO users VALUES (${i}, 'User${i}', 'user${i}@example.com', 25)`);
}

// ✅ Fast
const tx = db.beginTransaction();
for (let i = 0; i < 1000; i++) {
  db.executeSql(`INSERT INTO users VALUES (${i}, 'User${i}', 'user${i}@example.com', 25)`);
}
tx.commit();
```

## Limitations

- **JOIN**: Not currently supported (planned)
- **Subqueries**: Limited support
- **Foreign Keys**: Not currently supported

## Next Steps

- [KV Operations](kv-operations) - Key-Value operations
- [Advanced](advanced) - Transactions, performance
- [API Reference](api-reference) - Complete API
