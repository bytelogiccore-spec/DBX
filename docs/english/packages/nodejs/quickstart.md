---
layout: default
title: Quick Start
parent: Node.js (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 2
---

# Quick Start

Get started with DBX in 5 minutes!

## Installation

```bash
npm install dbx-py
```

## First Program

```typescript
import { Database } from 'dbx-py';

// Open in-memory database
const db = Database.openInMemory();

// KV operations
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
const value = db.get('users', Buffer.from('user:1'));
console.log(value?.toString());  // Alice

// SQL operations
db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");
const result = db.executeSql('SELECT * FROM users');
console.log(result);

db.close();
```

## TypeScript Usage

```typescript
interface User {
  id: number;
  name: string;
  email: string;
}

const db = Database.open('mydb.db');

// Store JSON
const user: User = { id: 1, name: 'Alice', email: 'alice@example.com' };
db.insert('users', Buffer.from('user:1'), Buffer.from(JSON.stringify(user)));

// Retrieve JSON
const data = db.get('users', Buffer.from('user:1'));
if (data) {
  const user: User = JSON.parse(data.toString());
  console.log(user.name);
}

db.close();
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [KV Operations](kv-operations) - Key-Value operations
- [API Reference](api-reference) - Complete API
