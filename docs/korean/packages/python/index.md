---
layout: default
title: Python (dbx-py)
nav_order: 3
parent: 패키지
grand_parent: 한국어
has_children: true
---

# Python — dbx-py

[![PyPI](https://img.shields.io/pypi/v/dbx-py.svg)](https://pypi.org/project/dbx-py/)

고성능 임베디드 데이터베이스 DBX의 공식 Python 바인딩입니다.

## 주요 기능

- 🚀 **고성능**: Rust 네이티브 구현으로 빠른 속도
- 💾 **5-Tier 스토리지**: WOS → L0 → L1 → L2 → Cold Storage
- 🔒 **MVCC 트랜잭션**: 스냅샷 격리 지원
- 📊 **SQL 지원**: DDL + DML 완벽 지원
- 🔐 **암호화**: AES-GCM-SIV, ChaCha20-Poly1305
- 🐍 **Pythonic API**: Context Manager, Type Hints

## 빠른 시작

```bash
pip install dbx-py
```

```python
from dbx_py import Database

with Database.open_in_memory() as db:
    # KV 작업
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode())  # Alice
    
    # SQL 작업
    db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT)")
    db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")
    result = db.execute_sql("SELECT * FROM users")
    print(result)
```

## 문서 구조

- [설치](installation) - 설치 및 환경 설정
- [빠른 시작](quickstart) - 5분 안에 시작하기
- [KV 작업](kv-operations) - Key-Value 작업 가이드
- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화, 성능 튜닝
- [API 레퍼런스](api-reference) - 전체 API 문서
- [실전 예제](examples) - 실무 활용 예제

## 버전 정보

- **현재 버전**: {{ site.dbx_version }}
- **Python 요구사항**: 3.8+
- **플랫폼**: Windows x64 (Linux/macOS 계획됨)

## 라이선스

MIT License
