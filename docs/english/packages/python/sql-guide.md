---
layout: default
title: SQL 가이드
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 4
---

# SQL 가이드

DBX는 표준 SQL을 지원합니다. DDL과 DML 모두 사용 가능합니다.

## 테이블 생성 (CREATE TABLE)

```python
from dbx_py import Database

with Database("mydb.db") as db:
    # 기본 테이블
    db.execute_sql("""
        CREATE TABLE users (
            id INTEGER,
            name TEXT,
            email TEXT,
            age INTEGER
        )
    """)
    
    # Primary Key 지정
    db.execute_sql("""
        CREATE TABLE products (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            price REAL
        )
    """)
```

## 데이터 삽입 (INSERT)

### 단일 행 삽입

```python
# 기본 INSERT
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)")

# 컬럼 명시
db.execute_sql("""
    INSERT INTO users (id, name, email) 
    VALUES (2, 'Bob', 'bob@example.com')
""")
```

### 다중 행 삽입

```python
# 여러 INSERT 문
db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)")
db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)")
db.execute_sql("INSERT INTO users VALUES (3, 'Carol', 'carol@example.com', 28)")

# 배치 삽입 (권장)
users = [
    (1, 'Alice', 'alice@example.com', 25),
    (2, 'Bob', 'bob@example.com', 30),
    (3, 'Carol', 'carol@example.com', 28)
]

for user in users:
    db.execute_sql(f"INSERT INTO users VALUES {user}")
```

## 데이터 조회 (SELECT)

### 기본 조회

```python
# 전체 조회
result = db.execute_sql("SELECT * FROM users")
print(result)

# 특정 컬럼
result = db.execute_sql("SELECT name, email FROM users")

# WHERE 조건
result = db.execute_sql("SELECT * FROM users WHERE age > 25")
```

### 정렬 및 제한

```python
# ORDER BY
result = db.execute_sql("SELECT * FROM users ORDER BY age DESC")

# LIMIT
result = db.execute_sql("SELECT * FROM users LIMIT 10")

# OFFSET
result = db.execute_sql("SELECT * FROM users LIMIT 10 OFFSET 20")
```

### 집계 함수

```python
# COUNT
result = db.execute_sql("SELECT COUNT(*) FROM users")

# AVG, SUM, MIN, MAX
result = db.execute_sql("""
    SELECT 
        AVG(age) as avg_age,
        MIN(age) as min_age,
        MAX(age) as max_age
    FROM users
""")

# GROUP BY
result = db.execute_sql("""
    SELECT age, COUNT(*) as count
    FROM users
    GROUP BY age
""")
```

## 데이터 수정 (UPDATE)

```python
# 단일 컬럼 수정
db.execute_sql("UPDATE users SET age = 26 WHERE id = 1")

# 다중 컬럼 수정
db.execute_sql("""
    UPDATE users 
    SET name = 'Alice Smith', email = 'alice.smith@example.com'
    WHERE id = 1
""")

# 조건부 수정
db.execute_sql("UPDATE users SET age = age + 1 WHERE age < 30")
```

## 데이터 삭제 (DELETE)

```python
# 특정 행 삭제
db.execute_sql("DELETE FROM users WHERE id = 1")

# 조건부 삭제
db.execute_sql("DELETE FROM users WHERE age < 18")

# 전체 삭제 (주의!)
db.execute_sql("DELETE FROM users")
```

## 테이블 삭제 (DROP TABLE)

```python
db.execute_sql("DROP TABLE users")
```

## 트랜잭션과 함께 사용

```python
from dbx_py import Database

with Database("mydb.db") as db:
    # 트랜잭션 시작
    tx = db.begin_transaction()
    
    try:
        # SQL 실행
        db.execute_sql("INSERT INTO users VALUES (1, 'Alice', 'alice@example.com', 25)")
        db.execute_sql("INSERT INTO users VALUES (2, 'Bob', 'bob@example.com', 30)")
        
        # 커밋
        tx.commit()
    except Exception as e:
        # 롤백
        tx.rollback()
        print(f"Error: {e}")
```

## 실전 예제

### 사용자 관리 시스템

```python
from dbx_py import Database
import json

class UserManager:
    def __init__(self, db_path: str):
        self.db = Database(db_path)
        self._init_schema()
    
    def _init_schema(self):
        """테이블 초기화"""
        self.db.execute_sql("""
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                username TEXT NOT NULL,
                email TEXT NOT NULL,
                created_at INTEGER
            )
        """)
    
    def create_user(self, username: str, email: str) -> int:
        """사용자 생성"""
        import time
        user_id = int(time.time() * 1000)
        
        self.db.execute_sql(f"""
            INSERT INTO users (id, username, email, created_at)
            VALUES ({user_id}, '{username}', '{email}', {int(time.time())})
        """)
        return user_id
    
    def get_user(self, user_id: int):
        """사용자 조회"""
        result = self.db.execute_sql(f"SELECT * FROM users WHERE id = {user_id}")
        return result
    
    def update_email(self, user_id: int, new_email: str):
        """이메일 수정"""
        self.db.execute_sql(f"""
            UPDATE users SET email = '{new_email}' WHERE id = {user_id}
        """)
    
    def delete_user(self, user_id: int):
        """사용자 삭제"""
        self.db.execute_sql(f"DELETE FROM users WHERE id = {user_id}")
    
    def list_users(self, limit: int = 100):
        """사용자 목록"""
        result = self.db.execute_sql(f"SELECT * FROM users LIMIT {limit}")
        return result
    
    def close(self):
        self.db.close()

# 사용 예제
manager = UserManager("users.db")

# 사용자 생성
user_id = manager.create_user("alice", "alice@example.com")
print(f"Created user: {user_id}")

# 조회
user = manager.get_user(user_id)
print(f"User: {user}")

# 수정
manager.update_email(user_id, "alice.new@example.com")

# 목록
users = manager.list_users()
print(f"All users: {users}")

manager.close()
```

### 로그 분석 시스템

```python
from dbx_py import Database
from datetime import datetime

class LogAnalyzer:
    def __init__(self, db_path: str):
        self.db = Database(db_path)
        self._init_schema()
    
    def _init_schema(self):
        self.db.execute_sql("""
            CREATE TABLE IF NOT EXISTS logs (
                id INTEGER,
                level TEXT,
                message TEXT,
                timestamp INTEGER
            )
        """)
    
    def add_log(self, level: str, message: str):
        """로그 추가"""
        import time
        log_id = int(time.time() * 1000000)
        ts = int(time.time())
        
        self.db.execute_sql(f"""
            INSERT INTO logs VALUES ({log_id}, '{level}', '{message}', {ts})
        """)
    
    def get_errors(self, limit: int = 100):
        """에러 로그 조회"""
        return self.db.execute_sql(f"""
            SELECT * FROM logs 
            WHERE level = 'ERROR' 
            ORDER BY timestamp DESC 
            LIMIT {limit}
        """)
    
    def get_stats(self):
        """로그 통계"""
        return self.db.execute_sql("""
            SELECT level, COUNT(*) as count
            FROM logs
            GROUP BY level
        """)
    
    def cleanup_old_logs(self, days: int = 30):
        """오래된 로그 삭제"""
        import time
        cutoff = int(time.time()) - (days * 24 * 60 * 60)
        
        self.db.execute_sql(f"DELETE FROM logs WHERE timestamp < {cutoff}")
    
    def close(self):
        self.db.close()

# 사용 예제
analyzer = LogAnalyzer("logs.db")

# 로그 추가
analyzer.add_log("INFO", "Application started")
analyzer.add_log("ERROR", "Connection failed")
analyzer.add_log("WARN", "High memory usage")

# 에러 조회
errors = analyzer.get_errors()
print(f"Errors: {errors}")

# 통계
stats = analyzer.get_stats()
print(f"Stats: {stats}")

# 정리
analyzer.cleanup_old_logs(30)

analyzer.close()
```

## 성능 팁

### 1. 배치 삽입 사용

```python
# ❌ 느림
for i in range(1000):
    db.execute_sql(f"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)")

# ✅ 빠름
values = []
for i in range(1000):
    values.append(f"({i}, 'User{i}', 'user{i}@example.com', 25)")

db.execute_sql(f"INSERT INTO users VALUES {','.join(values)}")
```

### 2. 트랜잭션 활용

```python
# ❌ 느림 (각 INSERT마다 커밋)
for i in range(1000):
    db.execute_sql(f"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)")

# ✅ 빠름 (한 번에 커밋)
tx = db.begin_transaction()
for i in range(1000):
    db.execute_sql(f"INSERT INTO users VALUES ({i}, 'User{i}', 'user{i}@example.com', 25)")
tx.commit()
```

### 3. 인덱스 활용

```python
# 자주 조회하는 컬럼에 인덱스 생성
db.execute_sql("CREATE INDEX idx_email ON users(email)")

# 조회 성능 향상
result = db.execute_sql("SELECT * FROM users WHERE email = 'alice@example.com'")
```

## 제한사항

- **JOIN**: 현재 미지원 (향후 지원 예정)
- **서브쿼리**: 제한적 지원
- **외래 키**: 현재 미지원

## 다음 단계

- [고급 기능](advanced) - 트랜잭션, 암호화
- [API 레퍼런스](api-reference) - 전체 API 문서
- [실전 예제](examples) - 더 많은 예제
