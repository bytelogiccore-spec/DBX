---
layout: default
title: API Reference
parent: Node.js (dbx-native)
grand_parent: Packages
great_grand_parent: English
nav_order: 6
---

# API Reference

## Database Class

### Static Methods

#### `Database.open(path: string): Database`

Opens a file-based database.

**Parameters:**
- `path` (string): Database file path

**Returns:** `Database` instance

**Example:**
```typescript
const db = Database.open('mydb.db');
```

#### `Database.openInMemory(): Database`

Opens an in-memory database.

**Returns:** `Database` instance

**Example:**
```typescript
const db = Database.openInMemory();
```

### Key-Value Methods

#### `insert(table: string, key: Buffer, value: Buffer): void`

Inserts a key-value pair.

**Parameters:**
- `table` (string): Table name
- `key` (Buffer): Key (binary)
- `value` (Buffer): Value (binary)

**Example:**
```typescript
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
```

#### `get(table: string, key: Buffer): Buffer | null`

Gets a value by key.

**Parameters:**
- `table` (string): Table name
- `key` (Buffer): Key (binary)

**Returns:** Value (Buffer) or null

**Example:**
```typescript
const value = db.get('users', Buffer.from('user:1'));
if (value) {
  console.log(value.toString());
}
```

#### `delete(table: string, key: Buffer): void`

Deletes a key.

**Parameters:**
- `table` (string): Table name
- `key` (Buffer): Key (binary)

**Example:**
```typescript
db.delete('users', Buffer.from('user:1'));
```

#### `count(table: string): number`

Returns the number of rows in a table.

**Parameters:**
- `table` (string): Table name

**Returns:** Row count (number)

**Example:**
```typescript
const count = db.count('users');
console.log(`Total: ${count}`);
```

### SQL Methods

#### `executeSql(sql: string): string`

Executes a SQL statement.

**Parameters:**
- `sql` (string): SQL statement

**Returns:** Result (string, JSON format)

**Example:**
```typescript
// DDL
db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');

// DML
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");

// Query
const result = db.executeSql('SELECT * FROM users');
console.log(result);
```

### Transaction Methods

#### `beginTransaction(): Transaction`

Begins a transaction.

**Returns:** `Transaction` object

**Example:**
```typescript
const tx = db.beginTransaction();
try {
  db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
  tx.commit();
} catch (error) {
  tx.rollback();
}
```

### Utility Methods

#### `flush(): void`

Flushes the buffer to disk.

**Example:**
```typescript
db.flush();
```

#### `close(): void`

Closes the database.

**Example:**
```typescript
db.close();
```

## Transaction Class

### Methods

#### `commit(): void`

Commits the transaction.

**Example:**
```typescript
const tx = db.beginTransaction();
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
tx.commit();
```

#### `rollback(): void`

Rolls back the transaction.

**Example:**
```typescript
const tx = db.beginTransaction();
try {
  db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
  tx.commit();
} catch (error) {
  tx.rollback();
}
```

## TypeScript Definitions

```typescript
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
```

## Error Handling

```typescript
try {
  const db = Database.open('mydb.db');
  db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
} catch (error) {
  console.error('DBX Error:', error);
}
```

## Next Steps

- [SQL Guide](sql-guide) - SQL usage
- [KV Operations](kv-operations) - Key-Value operations
- [Examples](examples) - Real-world examples
