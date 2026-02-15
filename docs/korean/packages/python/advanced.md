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

## 기능 플래그

```python
# 런타임에 기능 활성화/비활성화
db.enable_feature("parallel_query")
db.enable_feature("query_plan_cache")
db.disable_feature("parallel_query")

# 상태 확인
if db.is_feature_enabled("parallel_query"):
    print("병렬 쿼리 활성화됨")
```

## 쿼리 플랜 캐시

```python
# 플랜 캐시 활성화 후 동일 SQL 반복 실행 시 자동으로 캐시됨
db.enable_feature("query_plan_cache")

# 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for _ in range(100):
    results = db.execute_sql("SELECT * FROM users WHERE age > 20")
```

## 스키마 버저닝

```python
# 테이블 스키마 변경 이력이 자동으로 관리됨
db.execute_sql("CREATE TABLE users (id INT, name TEXT)")      # v1
db.execute_sql("ALTER TABLE users ADD COLUMN email TEXT")      # v2

# 스키마 버전 조회
version = db.schema_version("users")  # → 2
```

## UDF (사용자 정의 함수)

```python
# 스칼라 UDF 등록
def double_value(x):
    return x * 2

db.register_scalar_udf("double", double_value)

# SQL에서 사용
results = db.execute_sql("SELECT double(price) FROM products")
```

## 트리거

```python
# 데이터 변경 시 자동 실행
def on_user_insert(event):
    print(f"새 사용자: {event.new_values}")

db.register_trigger("users", "after_insert", on_user_insert)
```

## 스케줄러

```python
# 주기적 작업 등록
db.schedule_job("cleanup", "0 0 * * *", lambda: db.execute_sql(
    "DELETE FROM sessions WHERE expired = 1"
))
```

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
