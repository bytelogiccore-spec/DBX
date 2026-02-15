---
layout: default
title: API 레퍼런스
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 6
---

# API 레퍼런스

## Database 클래스

### 생성자

#### `Database(path: str)`

파일 기반 데이터베이스를 엽니다.

**매개변수:**
- `path` (str): 데이터베이스 파일 경로

**반환:** `Database` 인스턴스

**예제:**
```python
db = Database("mydb.db")
```

#### `Database.open_in_memory() -> Database`

인메모리 데이터베이스를 엽니다.

**반환:** `Database` 인스턴스

**예제:**
```python
db = Database.open_in_memory()
```

### Key-Value 메서드

#### `insert(table: str, key: bytes, value: bytes) -> None`

키-값 쌍을 삽입합니다.

**매개변수:**
- `table` (str): 테이블 이름
- `key` (bytes): 키 (바이너리)
- `value` (bytes): 값 (바이너리)

**예제:**
```python
db.insert("users", b"user:1", b"Alice")
```

#### `get(table: str, key: bytes) -> bytes | None`

키로 값을 조회합니다.

**매개변수:**
- `table` (str): 테이블 이름
- `key` (bytes): 키 (바이너리)

**반환:** 값 (bytes) 또는 None (키가 없을 경우)

**예제:**
```python
value = db.get("users", b"user:1")
if value:
    print(value.decode())
```

#### `delete(table: str, key: bytes) -> None`

키를 삭제합니다.

**매개변수:**
- `table` (str): 테이블 이름
- `key` (bytes): 키 (바이너리)

**예제:**
```python
db.delete("users", b"user:1")
```

#### `count(table: str) -> int`

테이블의 행 개수를 반환합니다.

**매개변수:**
- `table` (str): 테이블 이름

**반환:** 행 개수 (int)

**예제:**
```python
count = db.count("users")
print(f"Total: {count}")
```

### SQL 메서드

#### `execute_sql(sql: str) -> str`

SQL 문을 실행합니다.

**매개변수:**
- `sql` (str): SQL 문

**반환:** 결과 (문자열, JSON 형식)

**예제:**
```python
# DDL
db.execute_sql("CREATE TABLE users (id INTEGER, name TEXT)")

# DML
db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")

# 조회
result = db.execute_sql("SELECT * FROM users")
print(result)
```

### 트랜잭션 메서드

#### `begin_transaction() -> Transaction`

트랜잭션을 시작합니다.

**반환:** `Transaction` 객체

**예제:**
```python
tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    tx.commit()
except:
    tx.rollback()
```

### 유틸리티 메서드

#### `flush() -> None`

버퍼를 디스크에 플러시합니다.

**예제:**
```python
db.flush()
```

#### `close() -> None`

데이터베이스를 닫습니다.

**예제:**
```python
db.close()
```

### Context Manager

#### `__enter__() -> Database`

Context manager 진입.

#### `__exit__(exc_type, exc_val, exc_tb) -> None`

Context manager 종료. 자동으로 `flush()` 및 `close()` 호출.

**예제:**
```python
with Database("mydb.db") as db:
    db.insert("users", b"user:1", b"Alice")
# 자동으로 flush() 및 close() 호출됨
```

## Transaction 클래스

### 메서드

#### `commit() -> None`

트랜잭션을 커밋합니다.

**예제:**
```python
tx = db.begin_transaction()
db.insert("users", b"user:1", b"Alice")
tx.commit()
```

#### `rollback() -> None`

트랜잭션을 롤백합니다.

**예제:**
```python
tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    tx.commit()
except:
    tx.rollback()
```

## 예외

### `DbxError`

DBX 관련 모든 예외의 기본 클래스.

**예제:**
```python
from dbx_py import Database, DbxError

try:
    db = Database("mydb.db")
    db.insert("users", b"user:1", b"Alice")
except DbxError as e:
    print(f"Error: {e}")
```

## 타입 힌트

```python
from typing import Optional
from dbx_py import Database

class Database:
    def __init__(self, path: str) -> None: ...
    
    @staticmethod
    def open_in_memory() -> 'Database': ...
    
    def insert(self, table: str, key: bytes, value: bytes) -> None: ...
    
    def get(self, table: str, key: bytes) -> Optional[bytes]: ...
    
    def delete(self, table: str, key: bytes) -> None: ...
    
    def count(self, table: str) -> int: ...
    
    def execute_sql(self, sql: str) -> str: ...
    
    def begin_transaction(self) -> 'Transaction': ...
    
    def flush(self) -> None: ...
    
    def close(self) -> None: ...
    
    def __enter__(self) -> 'Database': ...
    
    def __exit__(self, exc_type, exc_val, exc_tb) -> None: ...
```

## 버전 정보

```python
import dbx_py

print(dbx_py.__version__)  # {{ site.dbx_py_version }}
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [KV 작업](kv-operations) - Key-Value 작업
- [실전 예제](examples) - 실무 활용 예제
