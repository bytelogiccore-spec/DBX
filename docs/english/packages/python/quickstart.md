---
layout: default
title: 빠른 시작
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 2
---

# 빠른 시작

5분 안에 DBX를 시작해보세요!

## 설치

```bash
pip install dbx-py
```

## 첫 번째 프로그램

```python
from dbx_py import Database

# 인메모리 데이터베이스 열기
db = Database.open_in_memory()

# KV 작업
db.insert("users", b"user:1", b"Alice")
value = db.get("users", b"user:1")
print(value.decode())  # Alice

# SQL 작업
db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT)")
db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")
result = db.execute_sql("SELECT * FROM users")
print(result)

db.close()
```

## Context Manager 사용

```python
with Database("mydb.db") as db:
    db.insert("users", b"user:1", b"Alice")
    # 자동으로 flush() 및 close()
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [API 레퍼런스](api-reference) - 전체 API
