---
layout: default
title: 고급 기능
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능

## 트랜잭션

### 기본 트랜잭션

```python
from dbx_py import Database

db = Database("mydb.db")

tx = db.begin_transaction()
try:
    db.insert("users", b"user:1", b"Alice")
    db.insert("users", b"user:2", b"Bob")
    tx.commit()
except Exception as e:
    tx.rollback()
    print(f"Transaction failed: {e}")
```

### Context Manager

```python
# 향후 지원 예정
```

## 암호화

```python
# 향후 지원 예정
# db = Database.open_encrypted("mydb.db", password="secret")
```

## 성능 튜닝

### 배치 작업

```python
# 트랜잭션 사용
tx = db.begin_transaction()
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
tx.commit()
db.flush()
```

### 버퍼 플러시

```python
# 명시적 플러시
db.flush()
```

## 멀티스레딩

```python
import threading

def worker(db_path, thread_id):
    db = Database(db_path)
    for i in range(1000):
        key = f"thread:{thread_id}:key:{i}".encode()
        value = f"value:{i}".encode()
        db.insert("data", key, value)
    db.close()

threads = []
for i in range(4):
    t = threading.Thread(target=worker, args=("mydb.db", i))
    threads.append(t)
    t.start()

for t in threads:
    t.join()
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
