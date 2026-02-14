---
layout: default
title: KV 작업
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 3
---

# Key-Value 작업

DBX는 SQL 외에도 고성능 Key-Value 스토어로 사용할 수 있습니다.

## 기본 CRUD

### 삽입 (Insert)

```python
from dbx_py import Database

with Database.open_in_memory() as db:
    # 기본 삽입
    db.insert("users", b"user:1", b"Alice")
    
    # JSON 데이터
    import json
    user = {"id": 1, "name": "Alice", "email": "alice@example.com"}
    db.insert("users", b"user:1", json.dumps(user).encode())
    
    # 바이너리 데이터
    db.insert("files", b"file:1", b"\x89PNG\r\n\x1a\n...")
```

### 조회 (Get)

```python
# 단일 조회
value = db.get("users", b"user:1")
if value:
    print(value.decode())  # Alice
else:
    print("Not found")

# JSON 파싱
value = db.get("users", b"user:1")
if value:
    user = json.loads(value.decode())
    print(user["name"])  # Alice
```

### 삭제 (Delete)

```python
db.delete("users", b"user:1")

# 존재 확인 후 삭제
if db.get("users", b"user:1"):
    db.delete("users", b"user:1")
    print("Deleted")
```

### 개수 확인 (Count)

```python
count = db.count("users")
print(f"Total users: {count}")
```

## 배치 작업

```python
# 대량 삽입
for i in range(10000):
    key = f"user:{i}".encode()
    value = f"User {i}".encode()
    db.insert("users", key, value)

# 플러시
db.flush()
```

## 테이블 관리

```python
# 여러 테이블 사용
db.insert("users", b"user:1", b"Alice")
db.insert("products", b"prod:1", b"Laptop")
db.insert("orders", b"order:1", b"Order data")

# 테이블별 개수
print(f"Users: {db.count('users')}")
print(f"Products: {db.count('products')}")
print(f"Orders: {db.count('orders')}")
```

## 실전 예제

### 세션 저장소

```python
import time
import json
from dbx_py import Database

class SessionStore:
    def __init__(self, db_path: str):
        self.db = Database(db_path)
    
    def create_session(self, session_id: str, user_data: dict, ttl: int = 3600):
        """세션 생성"""
        session = {
            "data": user_data,
            "created_at": time.time(),
            "expires_at": time.time() + ttl
        }
        self.db.insert("sessions", session_id.encode(), json.dumps(session).encode())
    
    def get_session(self, session_id: str) -> dict | None:
        """세션 조회"""
        value = self.db.get("sessions", session_id.encode())
        if not value:
            return None
        
        session = json.loads(value.decode())
        
        # 만료 확인
        if time.time() > session["expires_at"]:
            self.db.delete("sessions", session_id.encode())
            return None
        
        return session["data"]
    
    def delete_session(self, session_id: str):
        """세션 삭제"""
        self.db.delete("sessions", session_id.encode())
    
    def cleanup_expired(self):
        """만료된 세션 정리 (수동)"""
        # 현재는 get 시 자동 정리됨
        pass
    
    def close(self):
        self.db.close()

# 사용 예제
store = SessionStore("sessions.db")

# 세션 생성
store.create_session("sess_abc123", {
    "user_id": 42,
    "username": "alice",
    "role": "admin"
}, ttl=3600)

# 세션 조회
session = store.get_session("sess_abc123")
if session:
    print(f"User: {session['username']}")

# 세션 삭제
store.delete_session("sess_abc123")

store.close()
```

### 캐시 시스템

```python
import time
import json
from dbx_py import Database

class Cache:
    def __init__(self, db_path: str, default_ttl: int = 300):
        self.db = Database(db_path)
        self.default_ttl = default_ttl
    
    def set(self, key: str, value: any, ttl: int | None = None):
        """캐시 설정"""
        ttl = ttl or self.default_ttl
        cache_data = {
            "value": value,
            "expires_at": time.time() + ttl
        }
        self.db.insert("cache", key.encode(), json.dumps(cache_data).encode())
    
    def get(self, key: str) -> any | None:
        """캐시 조회"""
        data = self.db.get("cache", key.encode())
        if not data:
            return None
        
        cache_data = json.loads(data.decode())
        
        # 만료 확인
        if time.time() > cache_data["expires_at"]:
            self.db.delete("cache", key.encode())
            return None
        
        return cache_data["value"]
    
    def delete(self, key: str):
        """캐시 삭제"""
        self.db.delete("cache", key.encode())
    
    def clear(self):
        """전체 캐시 삭제"""
        # 현재는 수동으로 키 삭제 필요
        pass
    
    def close(self):
        self.db.close()

# 사용 예제
cache = Cache("cache.db", default_ttl=300)

# 캐시 설정
cache.set("user:1", {"name": "Alice", "email": "alice@example.com"})
cache.set("product:1", {"name": "Laptop", "price": 999.99}, ttl=600)

# 캐시 조회
user = cache.get("user:1")
if user:
    print(f"Cached user: {user['name']}")
else:
    print("Cache miss")

# 캐시 삭제
cache.delete("user:1")

cache.close()
```

## 성능 최적화

### 1. 배치 작업 + 플러시

```python
# ❌ 느림
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
    db.flush()  # 매번 플러시

# ✅ 빠름
for i in range(10000):
    db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
db.flush()  # 한 번만 플러시
```

### 2. 바이너리 키 사용

```python
# ❌ 느림 (문자열 인코딩)
for i in range(10000):
    key = f"key:{i}".encode()
    db.insert("data", key, b"value")

# ✅ 빠름 (바이너리 직접 사용)
for i in range(10000):
    key = i.to_bytes(4, 'big')
    db.insert("data", key, b"value")
```

### 3. Context Manager 사용

```python
# ✅ 자동 플러시 및 정리
with Database("data.db") as db:
    for i in range(10000):
        db.insert("data", f"key:{i}".encode(), f"value:{i}".encode())
# 자동으로 flush() 및 close() 호출됨
```

## 다음 단계

- [SQL 가이드](sql-guide) - SQL 사용법
- [고급 기능](advanced) - 트랜잭션, 암호화
- [API 레퍼런스](api-reference) - 전체 API
