---
layout: default
title: 트리거
parent: 한국어
nav_order: 31
---

# 트리거 (Triggers)
{: .no_toc }

데이터 변경 이벤트에 자동으로 반응하는 로직을 등록할 수 있습니다.
{: .fs-6 .fw-300 }

---

## 개요

트리거는 특정 테이블에서 INSERT, UPDATE, DELETE가 발생할 때 자동으로 실행되는 콜백 함수입니다.

| 실행 시점 | 설명 |
|----------|------|
| **Before** | 데이터 변경 전에 실행 (검증, 변환 용도) |
| **After** | 데이터 변경 후에 실행 (감사 로그, 알림 용도) |

---

## 트리거 등록

```rust
use dbx_core::automation::trigger::{Trigger, TriggerEvent, TriggerTiming};

let audit_trigger = Trigger::new(
    "audit_log",                    // 트리거 이름
    "users",                        // 대상 테이블
    TriggerEvent::Insert,           // INSERT 시 작동
    TriggerTiming::After,           // 변경 후 실행
    |event| {
        println!("새 사용자 추가: {:?}", event.new_values);
        Ok(())
    },
);

// 레지스트리에 등록
let mut registry = TriggerRegistry::new();
registry.register(audit_trigger);
```

---

## 이벤트 종류

```rust
pub enum TriggerEvent {
    Insert,     // 새 행 삽입 시
    Update,     // 기존 행 수정 시
    Delete,     // 행 삭제 시
    Any,        // 모든 변경 시
}
```

---

## 활용 예시

### 데이터 검증 (Before 트리거)

```rust
Trigger::new(
    "validate_age", "users",
    TriggerEvent::Insert, TriggerTiming::Before,
    |event| {
        let age = event.get_field::<i64>("age")?;
        if age < 0 || age > 150 {
            return Err(DbxError::Validation("나이가 유효하지 않습니다".into()));
        }
        Ok(())
    },
);
```

### 변경 이력 추적 (After 트리거)

```rust
Trigger::new(
    "log_changes", "orders",
    TriggerEvent::Any, TriggerTiming::After,
    |event| {
        log::info!("[{}] {}: {:?}", event.table, event.event_type, event.timestamp);
        Ok(())
    },
);
```

---

## 다음 단계

- [스케줄러 가이드](scheduler) — 주기적으로 실행되는 작업 등록
- [UDF 가이드](udf) — 커스텀 함수 정의
