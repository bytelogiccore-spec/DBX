---
layout: default
title: 고급 기능
parent: Python (dbx-py)
grand_parent: 패키지
great_grand_parent: 한국어
nav_order: 5
---

# 고급 기능
{: .no_toc }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

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

---

## SQL Trigger

SQL 표준 문법으로 데이터 변경 시 자동 실행되는 로직을 정의합니다.

### CREATE TRIGGER

```python
# 감사 로그 Trigger
db.execute_sql("""
    CREATE TRIGGER audit_trigger
    AFTER INSERT ON users
    FOR EACH ROW
    BEGIN
        INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', datetime('now'));
    END;
""")

# 데이터 검증 Trigger (WHEN 조건 사용)
db.execute_sql("""
    CREATE TRIGGER validate_price
    BEFORE UPDATE ON products
    FOR EACH ROW
    WHEN (NEW.price < 0)
    BEGIN
        SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '가격은 0 이상이어야 합니다';
    END;
""")
```

### DROP TRIGGER

```python
db.execute_sql("DROP TRIGGER audit_trigger;")
```

### 실전 예제

#### 변경 이력 추적

```python
# 변경 이력 테이블 생성
db.execute_sql("""
    CREATE TABLE change_log (
        id INT,
        old_price DECIMAL,
        new_price DECIMAL,
        changed_at TIMESTAMP
    )
""")

# UPDATE 시 변경 이력 기록
db.execute_sql("""
    CREATE TRIGGER track_price_changes
    AFTER UPDATE ON products
    FOR EACH ROW
    BEGIN
        INSERT INTO change_log VALUES (
            OLD.id, OLD.price, NEW.price, datetime('now')
        );
    END;
""")
```

#### 조건부 알림

```python
# 고액 주문 알림
db.execute_sql("""
    CREATE TRIGGER high_value_alert
    AFTER INSERT ON orders
    FOR EACH ROW
    WHEN (NEW.total > 10000)
    BEGIN
        INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER', datetime('now'));
    END;
""")
```

---

## Stored Procedure

재사용 가능한 SQL 프로시저를 정의하고 호출합니다.

### CREATE PROCEDURE

```python
# 잔액 업데이트 프로시저
db.execute_sql("""
    CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
    BEGIN
        UPDATE accounts SET balance = balance + amount WHERE id = user_id;
        INSERT INTO transactions VALUES (user_id, amount, datetime('now'));
    END;
""")

# 데이터 정리 프로시저
db.execute_sql("""
    CREATE PROCEDURE cleanup_old_data (days_old INT)
    BEGIN
        DELETE FROM logs WHERE created_at < datetime('now', '-' || days_old || ' days');
        DELETE FROM temp_files WHERE created_at < datetime('now', '-' || days_old || ' days');
        UPDATE stats SET last_cleanup = datetime('now');
    END;
""")
```

### CALL PROCEDURE

```python
# 프로시저 호출
db.execute_sql("CALL update_balance(123, 100.50);")
db.execute_sql("CALL cleanup_old_data(30);")
```

### DROP PROCEDURE

```python
db.execute_sql("DROP PROCEDURE update_balance;")
```

---

## Scheduler

Cron 표현식 기반으로 주기적인 작업을 자동 실행합니다.

### CREATE SCHEDULE

```python
# 매 5분마다 통계 갱신
db.execute_sql("""
    CREATE SCHEDULE refresh_stats
    CRON '*/5 * * * *'
    BEGIN
        UPDATE product_stats SET 
            total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id);
    END;
""")

# 매일 자정에 정리 작업
db.execute_sql("""
    CREATE SCHEDULE daily_cleanup
    CRON '0 0 * * *'
    BEGIN
        CALL cleanup_old_data(30);
    END;
""")
```

### Cron 표현식

| 표현식 | 설명 |
|--------|------|
| `*/5 * * * *` | 5분마다 |
| `0 * * * *` | 매시 정각 |
| `0 0 * * *` | 매일 자정 |
| `0 0 * * 0` | 매주 일요일 자정 |
| `0 0 1 * *` | 매월 1일 자정 |

형식: `분 시 일 월 요일`

### DROP SCHEDULE

```python
db.execute_sql("DROP SCHEDULE refresh_stats;")
```

---

## UDF (사용자 정의 함수)

### CREATE FUNCTION (SQL)

```python
# SQL 표준 문법으로 함수 정의
db.execute_sql("""
    CREATE FUNCTION add_numbers (a INT, b INT) RETURNS INT
    BEGIN
        RETURN a + b;
    END;
""")
```

> **참고**: 현재는 메타데이터 파싱만 지원하며, 실제 함수 로직은 Python 코드로 등록해야 합니다.

### Python 함수 등록

```python
# 스칼라 UDF 등록
def double_value(x):
    return x * 2

db.register_scalar_udf("double", double_value)

# SQL에서 사용
results = db.execute_sql("SELECT double(price) FROM products")
```

### 집계 UDF

```python
# 중앙값 계산
def median_accumulate(state, value):
    state.append(value)
    return state

def median_finalize(state):
    sorted_state = sorted(state)
    return sorted_state[len(sorted_state) // 2]

db.register_aggregate_udf("median", [], median_accumulate, median_finalize)

# SQL에서 사용
results = db.execute_sql("SELECT median(score) FROM students")
```

---

## Event Hook (Rust 클로저 기반)

Python에서는 SQL Trigger를 사용하는 것을 권장하지만, 향후 Python 콜백 지원 예정입니다.

```python
# 향후 지원 예정
# def on_user_insert(event):
#     print(f"새 사용자: {event.new_values}")
# 
# db.register_event_hook("users", "after_insert", on_user_insert)
```

---

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

---

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

---

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

---

## 쿼리 플랜 캐시

```python
# 플랜 캐시 활성화 후 동일 SQL 반복 실행 시 자동으로 캐시됨
db.enable_feature("query_plan_cache")

# 동일 쿼리 반복 시 파싱을 건너뜀 (7.3x 빠름)
for _ in range(100):
    results = db.execute_sql("SELECT * FROM users WHERE age > 20")
```

---

## 스키마 버저닝

```python
# 테이블 스키마 변경 이력이 자동으로 관리됨
db.execute_sql("CREATE TABLE users (id INT, name TEXT)")      # v1
db.execute_sql("ALTER TABLE users ADD COLUMN email TEXT")      # v2

# 스키마 버전 조회
version = db.schema_version("users")  # → 2
```

---

## 암호화

```python
# 향후 지원 예정
# db = Database.open_encrypted("mydb.db", password="secret")
```

---

## 다음 단계

- [실전 예제](examples) - 더 많은 예제
- [API 레퍼런스](api-reference) - 전체 API
