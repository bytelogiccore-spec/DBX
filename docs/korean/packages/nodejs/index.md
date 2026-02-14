---
layout: default
title: Node.js (dbx-native)
nav_order: 4
parent: 패키지
grand_parent: 한국어
has_children: true
---

# Node.js — dbx-native

[![npm](https://img.shields.io/npm/v/dbx-native.svg)](https://www.npmjs.com/package/dbx-native)

고성능 임베디드 데이터베이스 DBX의 공식 Node.js 바인딩입니다.

## 주요 기능

- 🚀 **네이티브 성능**: Rust 기반 N-API 바인딩
- 💾 **5-Tier 스토리지**: WOS → L0 → L1 → L2 → Cold Storage
- 🔒 **MVCC 트랜잭션**: 스냅샷 격리 지원
- 📊 **SQL 지원**: DDL + DML 완벽 지원
- 🔐 **암호화**: AES-GCM-SIV, ChaCha20-Poly1305
- 📘 **TypeScript**: 완벽한 타입 정의

## 빠른 시작

```bash
npm install dbx-native
```

```typescript
import { Database } from 'dbx-native';

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

## 문서 구조

- [설치](installation) - 설치 및 환경 설정
- [빠른 시작](quickstart) - 5분 안에 시작하기
- [KV 작업](kv-operations) - Key-Value 작업 가이드
- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화, 성능 튜닝
- [TypeScript](typescript) - TypeScript 사용법
- [API 레퍼런스](api-reference) - 전체 API 문서
- [실전 예제](examples) - 실무 활용 예제

## 버전 정보

- **현재 버전**: 0.0.3-beta
- **Node.js 요구사항**: 16+
- **플랫폼**: Windows x64 (Linux/macOS 계획됨)

## 라이선스

MIT License
