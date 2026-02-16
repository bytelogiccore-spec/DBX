---
layout: default
title: Stored Procedures
parent: 한국어
nav_order: 33
---

# Stored Procedures
{: .no_toc }

재사용 가능한 SQL 프로시저를 정의하고 호출합니다.
{: .fs-6 .fw-300 }

---

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

Stored Procedure는 여러 SQL 문장을 하나의 논리적 단위로 묶어 재사용할 수 있는 기능입니다.

### 주요 특징

- ✅ 파라미터 지원
- ✅ 다중 SQL 문장 실행
- ✅ 메타데이터 영구 저장
- ✅ Database 재시작 시 자동 등록

---

## CREATE PROCEDURE

### 기본 문법

```sql
CREATE PROCEDURE procedure_name (param1 TYPE, param2 TYPE, ...)
BEGIN
    sql_statement;
    ...
END;
```

### 예제: 잔액 업데이트

```sql
CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
BEGIN
    UPDATE accounts SET balance = balance + amount WHERE id = user_id;
    INSERT INTO transactions VALUES (user_id, amount, NOW());
END;
```

### 예제: 파라미터 없는 프로시저

```sql
CREATE PROCEDURE reset_stats ()
BEGIN
    UPDATE stats SET count = 0;
    DELETE FROM temp_data;
END;
```

---

## CALL PROCEDURE

```sql
CALL update_balance(123, 100.50);
CALL reset_stats();
```

---

## DROP PROCEDURE

```sql
DROP PROCEDURE procedure_name;
```

---

## Rust API

### Procedure 파싱

```rust
use dbx_core::automation::{
    parse_create_procedure, 
    parse_drop_procedure, 
    parse_call_procedure
};

// CREATE PROCEDURE 파싱
let sql = r#"
    CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
    BEGIN
        UPDATE accounts SET balance = balance + amount WHERE id = user_id;
        INSERT INTO transactions VALUES (user_id, amount);
    END;
"#;

let proc = parse_create_procedure(sql)?;
println!("Procedure: {}", proc.name);
println!("Parameters: {}", proc.parameters.len());

// CALL PROCEDURE 파싱
let call_sql = "CALL update_balance(123, 100.50);";
let (name, args) = parse_call_procedure(call_sql)?;
```

### 메타데이터 저장/로드

```rust
use dbx_core::engine::metadata;

// Procedure 저장
metadata::save_procedure(&wos, &proc)?;

// Procedure 로드
let loaded = metadata::load_procedure(&wos, "update_balance")?.unwrap();

// 모든 Procedure 로드
let all_procedures = metadata::load_all_procedures(&wos)?;
```

### 실행

```rust
// Procedure 실행
db.procedure_executor.read().unwrap().execute(
    &db,
    "update_balance",
    &["123".to_string(), "100.50".to_string()],
)?;
```

---

## 파라미터

### 파라미터 정의

```rust
pub struct ProcedureParameter {
    pub name: String,
    pub data_type: String,
}
```

### 예제

```sql
CREATE PROCEDURE transfer_funds (
    from_id INT,
    to_id INT,
    amount DECIMAL
)
BEGIN
    UPDATE accounts SET balance = balance - amount WHERE id = from_id;
    UPDATE accounts SET balance = balance + amount WHERE id = to_id;
    INSERT INTO transfers VALUES (from_id, to_id, amount, NOW());
END;
```

---

## 실전 예제

### 예제 1: 데이터 정리

```sql
CREATE PROCEDURE cleanup_old_data (days_old INT)
BEGIN
    DELETE FROM logs WHERE created_at < NOW() - INTERVAL days_old DAY;
    DELETE FROM temp_files WHERE created_at < NOW() - INTERVAL days_old DAY;
    UPDATE stats SET last_cleanup = NOW();
END;
```

### 예제 2: 통계 갱신

```sql
CREATE PROCEDURE refresh_statistics ()
BEGIN
    UPDATE product_stats SET 
        total_sales = (SELECT SUM(quantity) FROM orders WHERE product_id = products.id),
        avg_price = (SELECT AVG(price) FROM orders WHERE product_id = products.id);
    UPDATE global_stats SET last_updated = NOW();
END;
```

---

## 주의사항

⚠️ **현재 제한사항**
- SQL 실행 로직은 향후 구현 예정
- 현재는 메타데이터 저장/로드 및 파싱만 지원
- 파라미터 타입 검증 미지원

🔧 **향후 개선 예정**
- Procedure body SQL 실제 실행
- 파라미터 타입 검증
- 반환 값 지원
- 트랜잭션 지원

---

## 다음 단계

- [SQL Trigger 가이드](triggers) — 이벤트 기반 자동 실행
- [UDF 가이드](udf) — 사용자 정의 함수
- [트랜잭션 가이드](transactions) — 트랜잭션 관리
