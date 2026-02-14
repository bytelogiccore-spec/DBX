---
layout: default
title: Python (dbx-py)
parent: Packages
grand_parent: 한국어
nav_order: 3
---

# Python — dbx-py

[![PyPI](https://img.shields.io/pypi/v/dbx-py.svg)](https://pypi.org/project/dbx-py/)

FFI를 통한 고성능 Python 바인딩으로 Pythonic API 디자인을 제공합니다.

## 설치

```bash
pip install dbx-py
```

## 빠른 시작

```python
from dbx_py import Database

# 인메모리 데이터베이스 열기
db = Database.open_in_memory()

# 삽입
db.insert("users", b"user:1", b"Alice")
db.insert("users", b"user:2", b"Bob")

# 조회
value = db.get("users", b"user:1")
print(value.decode('utf-8'))  # Alice

# 삭제
db.delete("users", b"user:2")

# 개수 확인
count = db.count("users")
print(f"전체 사용자: {count}")

# 닫기
db.close()
```

## Context Manager 사용 (권장)

```python
with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
# 자동으로 닫히고 플러시됨
```

## 고급 사용법

### JSON 데이터 다루기

```python
import json

with Database.open_in_memory() as db:
    # JSON 데이터 저장
    user = {"id": 1, "name": "Alice", "email": "alice@example.com"}
    db.insert("users", b"user:1", json.dumps(user).encode())
    
    # JSON 데이터 조회
    data = db.get("users", b"user:1")
    user = json.loads(data.decode('utf-8'))
    print(user["name"])  # Alice
```

### 배치 작업

```python
with Database("data.db") as db:
    # 배치 삽입
    for i in range(1000):
        key = f"item:{i}".encode()
        value = f"value_{i}".encode()
        db.insert("items", key, value)
    
    # 디스크에 플러시
    db.flush()
```

### 에러 처리

```python
from dbx_py import Database, DbxError

try:
    db = Database("my.db")
    db.insert("users", b"key1", b"value1")
    db.flush()
except DbxError as e:
    print(f"데이터베이스 오류: {e}")
finally:
    db.close()
```

### 반복 처리 (지원 시)

```python
with Database("data.db") as db:
    # 테스트 데이터 삽입
    for i in range(10):
        db.insert("test", f"key{i}".encode(), f"val{i}".encode())
    
    # 개수 확인
    count = db.count("test")
    print(f"전체 항목: {count}")
```

## API 레퍼런스

### Database 클래스

#### 생성자

| 메서드 | 반환 | 설명 |
|--------|------|------|
| `Database(path: str)` | `Database` | 파일 기반 데이터베이스 열기 |
| `Database.open_in_memory()` | `Database` | 인메모리 데이터베이스 열기 |

#### 핵심 메서드

| 메서드 | 매개변수 | 반환 | 설명 |
|--------|----------|------|------|
| `insert` | `table: str, key: bytes, value: bytes` | `None` | 키-값 쌍 삽입 |
| `get` | `table: str, key: bytes` | `bytes \| None` | 키로 값 조회 |
| `delete` | `table: str, key: bytes` | `None` | 키 삭제 |
| `count` | `table: str` | `int` | 테이블 행 개수 |
| `flush` | - | `None` | 디스크에 플러시 |
| `close` | - | `None` | 데이터베이스 닫기 |

#### Context Manager

```python
def __enter__(self) -> Database
def __exit__(self, exc_type, exc_val, exc_tb) -> None
```

## 타입 힌트

```python
from typing import Optional

class Database:
    def __init__(self, path: str) -> None: ...
    
    @staticmethod
    def open_in_memory() -> 'Database': ...
    
    def insert(self, table: str, key: bytes, value: bytes) -> None: ...
    
    def get(self, table: str, key: bytes) -> Optional[bytes]: ...
    
    def delete(self, table: str, key: bytes) -> None: ...
    
    def count(self, table: str) -> int: ...
    
    def flush(self) -> None: ...
    
    def close(self) -> None: ...
```

## 성능 팁

1. **Context Manager 사용**: 올바른 정리 보장
2. **배치 작업**: 플러시 전에 여러 삽입 그룹화
3. **바이너리 키**: 인코딩 오버헤드 방지를 위해 `bytes` 사용
4. **테스트용 인메모리**: 단위 테스트에 더 빠름

## 요구사항

- Python 3.8+
- Windows x64 / Linux x64 / macOS (ARM64/x64)

## 문제 해결

### Import 오류

```python
# "No module named 'dbx_py'" 오류 발생 시
pip install --upgrade dbx-py
```

### 성능 문제

```python
# 배치 모드 활성화
with Database("data.db") as db:
    for i in range(10000):
        db.insert("bulk", f"k{i}".encode(), f"v{i}".encode())
    db.flush()  # 마지막에 한 번만 플러시
```

## 예제

### 간단한 Key-Value 저장소

```python
from dbx_py import Database

class KVStore:
    def __init__(self, path: str):
        self.db = Database(path)
    
    def set(self, key: str, value: str):
        self.db.insert("kv", key.encode(), value.encode())
    
    def get(self, key: str) -> str | None:
        data = self.db.get("kv", key.encode())
        return data.decode() if data else None
    
    def close(self):
        self.db.close()

# 사용법
store = KVStore("store.db")
store.set("name", "Alice")
print(store.get("name"))  # Alice
store.close()
```

### 세션 캐시

```python
import time
from dbx_py import Database

class SessionCache:
    def __init__(self):
        self.db = Database.open_in_memory()
    
    def set(self, session_id: str, data: dict, ttl: int = 3600):
        import json
        payload = {
            "data": data,
            "expires": time.time() + ttl
        }
        self.db.insert("sessions", session_id.encode(), 
                      json.dumps(payload).encode())
    
    def get(self, session_id: str) -> dict | None:
        import json
        raw = self.db.get("sessions", session_id.encode())
        if not raw:
            return None
        
        payload = json.loads(raw.decode())
        if time.time() > payload["expires"]:
            self.db.delete("sessions", session_id.encode())
            return None
        
        return payload["data"]

# 사용법
cache = SessionCache()
cache.set("sess_123", {"user_id": 42, "role": "admin"})
print(cache.get("sess_123"))
```
