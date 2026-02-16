---
layout: default
title: SQL 트리거
parent: 한국어
nav_order: 32
---

# SQL 트리거 (SQL Triggers)
{: .no_toc }

SQL 표준 문법으로 데이터 변경 시 자동 실행되는 로직을 정의합니다.
{: .fs-6 .fw-300 }

---

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

SQL Trigger는 SQL 표준 문법을 사용하여 INSERT, UPDATE, DELETE 이벤트 발생 시 자동으로 SQL 문장을 실행하는 기능입니다.

> **참고**: Rust 클로저 기반 EventHook은 [EventHook 가이드](event-hook)를 참조하세요.

### 주요 특징

- ✅ SQL 표준 문법 지원
- ✅ 메타데이터 영구 저장
- ✅ Database 재시작 시 자동 등록
- ✅ BEFORE/AFTER 실행 시점 지원

---

## CREATE TRIGGER

### 기본 문법

```sql
CREATE TRIGGER trigger_name
{BEFORE | AFTER} {INSERT | UPDATE | DELETE} ON table_name
FOR EACH ROW
[WHEN (condition)]
BEGIN
    sql_statement;
    ...
END;
```

### 예제: 감사 로그

```sql
CREATE TRIGGER audit_trigger
AFTER INSERT ON users
FOR EACH ROW
BEGIN
    INSERT INTO audit_logs VALUES (NEW.id, 'INSERT', NOW());
END;
```

### 예제: 데이터 검증

```sql
CREATE TRIGGER validate_price
BEFORE UPDATE ON products
FOR EACH ROW
WHEN (NEW.price < 0)
BEGIN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = '가격은 0 이상이어야 합니다';
END;
```

---

## DROP TRIGGER

```sql
DROP TRIGGER trigger_name;
```

---

## Rust API

### Trigger 파싱

```rust
use dbx_core::automation::{parse_create_trigger, parse_drop_trigger};

// CREATE TRIGGER 파싱
let sql = r#"
    CREATE TRIGGER audit_trigger
    AFTER INSERT ON users
    FOR EACH ROW
    BEGIN
        INSERT INTO audit_logs VALUES (NEW.id, 'INSERT');
    END;
"#;

let trigger = parse_create_trigger(sql)?;
println!("Trigger: {}", trigger.name);

// DROP TRIGGER 파싱
let drop_sql = "DROP TRIGGER audit_trigger;";
let name = parse_drop_trigger(drop_sql)?;
```

### 메타데이터 저장/로드

```rust
use dbx_core::engine::metadata;

// Trigger 저장
metadata::save_trigger(&wos, &trigger)?;

// Trigger 로드
let loaded = metadata::load_trigger(&wos, "audit_trigger")?.unwrap();

// 모든 Trigger 로드
let all_triggers = metadata::load_all_triggers(&wos)?;
```

### 자동 등록

Database를 열 때 저장된 Trigger가 자동으로 등록됩니다:

```rust
let db = Database::open("./my_db")?;

// 저장된 Trigger 자동 등록됨
let trigger_count = db.trigger_executor.read().unwrap().list_triggers().len();
println!("Loaded {} triggers", trigger_count);
```

---

## 실행 시점

| 시점 | 설명 | 용도 |
|------|------|------|
| **BEFORE** | 데이터 변경 전 | 검증, 데이터 변환 |
| **AFTER** | 데이터 변경 후 | 감사 로그, 통계 업데이트 |

---

## 이벤트 종류

| 이벤트 | 설명 |
|--------|------|
| **INSERT** | 새 행 삽입 시 |
| **UPDATE** | 기존 행 수정 시 |
| **DELETE** | 행 삭제 시 |

---

## NEW/OLD 참조

Trigger 내부에서 변경 전후 값을 참조할 수 있습니다:

| 참조 | 사용 가능 시점 | 설명 |
|------|---------------|------|
| **NEW** | INSERT, UPDATE | 새로운 값 |
| **OLD** | UPDATE, DELETE | 이전 값 |

```sql
CREATE TRIGGER track_changes
AFTER UPDATE ON products
FOR EACH ROW
BEGIN
    INSERT INTO change_log VALUES (
        OLD.id, OLD.price, NEW.price, NOW()
    );
END;
```

---

## 조건부 실행 (WHEN)

```sql
CREATE TRIGGER high_value_alert
AFTER INSERT ON orders
FOR EACH ROW
WHEN (NEW.total > 10000)
BEGIN
    INSERT INTO alerts VALUES (NEW.id, 'HIGH_VALUE_ORDER');
END;
```

---

## 주의사항

⚠️ **현재 제한사항**
- SQL 실행 로직은 향후 구현 예정
- 현재는 메타데이터 저장/로드 및 파싱만 지원

🔧 **향후 개선 예정**
- Trigger body SQL 실제 실행
- WHEN 조건 평가
- 트랜잭션 지원
- 에러 핸들링 강화

---

## 다음 단계

- [Stored Procedure 가이드](stored-procedures) — 재사용 가능한 SQL 프로시저
- [EventHook 가이드](event-hook) — Rust 클로저 기반 이벤트 처리
- [UDF 가이드](udf) — 사용자 정의 함수
