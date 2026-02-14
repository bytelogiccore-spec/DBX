---
layout: default
title: 실전 예제
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 7
---

# 실전 예제

## Flask 웹 애플리케이션

```python
from flask import Flask, request, jsonify
from dbx_py import Database

app = Flask(__name__)
db = Database("app.db")

# 스키마 초기화
db.execute_sql("CREATE TABLE IF NOT EXISTS users (id INTEGER, name TEXT, email TEXT)")

@app.route('/users', methods=['POST'])
def create_user():
    data = request.json
    user_id = int(time.time())
    db.execute_sql(f"INSERT INTO users VALUES ({user_id}, '{data['name']}', '{data['email']}')")
    return jsonify({"id": user_id}), 201

@app.route('/users/<int:user_id>')
def get_user(user_id):
    result = db.execute_sql(f"SELECT * FROM users WHERE id = {user_id}")
    return jsonify({"result": result})

if __name__ == '__main__':
    app.run(debug=True)
```

## 로그 분석기

```python
import re
from datetime import datetime
from dbx_py import Database

class LogAnalyzer:
    def __init__(self, db_path):
        self.db = Database(db_path)
        self.db.execute_sql("""
            CREATE TABLE IF NOT EXISTS logs (
                timestamp INTEGER,
                level TEXT,
                message TEXT
            )
        """)
    
    def parse_log(self, log_file):
        pattern = r'(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) \[(\w+)\] (.+)'
        
        with open(log_file) as f:
            for line in f:
                match = re.match(pattern, line)
                if match:
                    ts_str, level, message = match.groups()
                    ts = int(datetime.strptime(ts_str, '%Y-%m-%d %H:%M:%S').timestamp())
                    self.db.execute_sql(
                        f"INSERT INTO logs VALUES ({ts}, '{level}', '{message}')"
                    )
    
    def get_errors(self):
        return self.db.execute_sql("SELECT * FROM logs WHERE level = 'ERROR'")
    
    def close(self):
        self.db.close()

# 사용
analyzer = LogAnalyzer("logs.db")
analyzer.parse_log("app.log")
errors = analyzer.get_errors()
print(errors)
analyzer.close()
```

## 캐시 데코레이터

```python
import functools
import json
from dbx_py import Database

class CacheDecorator:
    def __init__(self, db_path, ttl=300):
        self.db = Database(db_path)
        self.ttl = ttl
    
    def __call__(self, func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            # 캐시 키 생성
            key = f"{func.__name__}:{json.dumps(args)}:{json.dumps(kwargs)}"
            
            # 캐시 조회
            cached = self.db.get("cache", key.encode())
            if cached:
                return json.loads(cached.decode())
            
            # 함수 실행
            result = func(*args, **kwargs)
            
            # 캐시 저장
            self.db.insert("cache", key.encode(), json.dumps(result).encode())
            
            return result
        return wrapper

# 사용
cache = CacheDecorator("cache.db")

@cache
def expensive_function(x, y):
    import time
    time.sleep(2)  # 시뮬레이션
    return x + y

print(expensive_function(1, 2))  # 2초 소요
print(expensive_function(1, 2))  # 즉시 반환 (캐시)
```

## 다음 단계

- [API 레퍼런스](api-reference) - 전체 API
- [고급 기능](advanced) - 트랜잭션, 성능 튜닝
