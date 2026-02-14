---
layout: default
title: Node.js (dbx-native)
parent: 패키지
grand_parent: 한국어
nav_order: 4
---

# Node.js — dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)

N-API를 통한 네이티브 Node.js 바인딩으로 오버헤드 거의 없이 동작합니다.

## 설치

```bash
npm install dbx-native
```

## 빠른 시작

```javascript
const dbx = require('dbx-native');

// 인메모리 데이터베이스
const db = dbx.openInMemory();

// 삽입
db.insert('users', Buffer.from('user:1'), Buffer.from('Alice'));
db.insert('users', Buffer.from('user:2'), Buffer.from('Bob'));

// 조회
const value = db.get('users', Buffer.from('user:1'));
console.log(value.toString()); // Alice

// 삭제
db.delete('users', Buffer.from('user:2'));

// 닫기
db.close();
```

## SQL 인터페이스

```javascript
const db = dbx.openInMemory();

db.executeSql('CREATE TABLE users (id INTEGER, name TEXT)');
db.executeSql("INSERT INTO users VALUES (1, 'Alice')");

const result = db.executeSql('SELECT * FROM users');
console.log(result);

db.close();
```

## API 레퍼런스

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `openInMemory()` | `Database` | 인메모리 DB 열기 |
| `open(path)` | `Database` | 파일 기반 DB 열기 |
| `insert(table, key, value)` | `void` | 키-값 삽입 |
| `get(table, key)` | `Buffer` | 키로 값 조회 |
| `delete(table, key)` | `void` | 키 삭제 |
| `executeSql(sql)` | `Result` | SQL 실행 |
| `close()` | `void` | 데이터베이스 닫기 |

## 벤치마크

```bash
cd lang/nodejs
npm run bench
```

## 요구 사항

- Node.js 18+
- Windows x64
