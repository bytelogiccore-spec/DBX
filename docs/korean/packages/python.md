---
layout: default
title: Python (dbx-py)
parent: 패키지
grand_parent: 한국어
nav_order: 3
---

# Python — dbx-py

[![PyPI](https://img.shields.io/pypi/v/dbx-py.svg)](https://pypi.org/project/dbx-py/)

FFI를 통한 Python 바인딩으로 Pythonic API 디자인을 제공합니다.

## 설치

```bash
pip install dbx-py
```

## 빠른 시작

```python
from dbx_py import Database

# 인메모리 데이터베이스
db = Database.open_in_memory()

# 삽입
db.insert("users", b"user:1", b"Alice")
db.insert("users", b"user:2", b"Bob")

# 조회
value = db.get("users", b"user:1")
print(value.decode('utf-8'))  # Alice

# 삭제
db.delete("users", b"user:2")

# 카운트
count = db.count("users")
print(f"총 사용자: {count}")

# 닫기
db.close()
```

## 컨텍스트 매니저

```python
with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
# 자동으로 닫힘
```

## API 레퍼런스

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `Database(path)` | `Database` | 파일 기반 DB 열기 |
| `Database.open_in_memory()` | `Database` | 인메모리 DB 열기 |
| `insert(table, key, value)` | `None` | 키-값 삽입 |
| `get(table, key)` | `bytes \| None` | 값 조회 |
| `delete(table, key)` | `None` | 키 삭제 |
| `count(table)` | `int` | 행 수 카운트 |
| `flush()` | `None` | 디스크에 저장 |
| `close()` | `None` | 데이터베이스 닫기 |

## 요구 사항

- Python 3.8+
- Windows x64
