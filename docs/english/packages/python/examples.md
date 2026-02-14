---
layout: default
title: Examples
parent: Python (dbx-py)
grand_parent: Packages
great_grand_parent: English
nav_order: 7
---

# Real-World Examples

## Flask Web Application

```python
from flask import Flask, request, jsonify
from dbx_py import Database

app = Flask(__name__)
db = Database("app.db")

# Initialize schema
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

## Log Analyzer

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

# Usage
analyzer = LogAnalyzer("logs.db")
analyzer.parse_log("app.log")
errors = analyzer.get_errors()
print(errors)
analyzer.close()
```

## Cache Decorator

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
            # Generate cache key
            key = f"{func.__name__}:{json.dumps(args)}:{json.dumps(kwargs)}"
            
            # Check cache
            cached = self.db.get("cache", key.encode())
            if cached:
                return json.loads(cached.decode())
            
            # Execute function
            result = func(*args, **kwargs)
            
            # Save to cache
            self.db.insert("cache", key.encode(), json.dumps(result).encode())
            
            return result
        return wrapper

# Usage
cache = CacheDecorator("cache.db")

@cache
def expensive_function(x, y):
    import time
    time.sleep(2)  # Simulation
    return x + y

print(expensive_function(1, 2))  # Takes 2 seconds
print(expensive_function(1, 2))  # Returns immediately (cached)
```

## Next Steps

- [API Reference](api-reference) - Complete API
- [Advanced](advanced) - Transactions, performance tuning
