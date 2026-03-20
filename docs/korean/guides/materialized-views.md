---
layout: default
title: 구체화된 뷰 (Materialized Views)
parent: 한국어
nav_order: 22
description: "사전 계산된 쿼리 결과 저장 및 자동 갱신 가이드"
---

# 구체화된 뷰 (Materialized Views)
{: .no_toc }

구체화된 뷰는 복잡한 SQL 쿼리의 결과를 미리 계산하여 캐시로 저장해두는 기능입니다. 대규모 데이터셋에 대한 분석 쿼리 성능을 획기적으로 향상시킬 수 있습니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

일반적인 뷰(View)가 쿼리 실행 시점에 매번 기본 테이블을 참조하는 것과 달리, **구체화된 뷰(Materialized View)**는 물리적인 캐시에 결과를 저장합니다.

### 주요 특징
- **쿼리 성능 가속**: 복잡한 JOIN이나 집계가 포함된 SELECT 결과를 즉시 반환합니다.
- **자동 갱신 (Auto-Refresh)**: 설정된 시간(초)마다 백그라운드에서 자동으로 결과를 최신화합니다.
- **투명한 캐싱**: 사용자가 `SELECT` 쿼리를 실행할 때, 구체화된 뷰가 정의되어 있고 캐시가 유효하면 자동으로 캐시를 사용합니다.

---

## 구체화된 뷰 생성

### 기본 문법

```sql
CREATE MATERIALIZED VIEW [뷰_이름] 
[REFRESH EVERY [초]] 
AS [SELECT_쿼리]
```

### 예제: 60초마다 자동 갱신되는 관리자용 통계 뷰

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // 60초마다 자동으로 매출 통계를 갱신하는 뷰 생성
    db.execute_sql(
        "CREATE MATERIALIZED VIEW sales_summary 
         REFRESH EVERY 60 
         AS SELECT category, SUM(price) FROM orders GROUP BY category"
    )?;
    
    Ok(())
}
```

---

## 구체화된 뷰 사용

구체화된 뷰가 생성되면, 동일한 SQL 쿼리를 실행할 때 DBX 엔진이 내부적으로 캐시를 확인합니다.

```rust
// 일반 SELECT 문을 실행하면, sales_summary 뷰에 저장된 캐시가 있으면 이를 즉시 반환합니다.
let results = db.execute_sql("SELECT category, SUM(price) FROM orders GROUP BY category")?;
```

> [!NOTE]
> `REFRESH EVERY` 설정이 없으면 뷰는 수동으로 갱신될 때까지 기존 캐시를 유지합니다.

---

## 관리 명령어

### 수동 갱신 (Refresh)

특정 시점에 즉시 최신 데이터로 동기화하고 싶을 때 사용합니다.

```sql
REFRESH MATERIALIZED VIEW sales_summary
```

### 삭제 (Drop)

```sql
DROP MATERIALIZED VIEW sales_summary
```

---

## 내부 동작 방식

1. **등록**: `CREATE` 명령 시 SQL 문과 갱신 주기가 `MaterializedViewRegistry`에 저장됩니다.
2. **백그라운드 스레드**: `Database` 오픈 시 생성된 전용 스레드가 주기적으로(현재 60초 단위 체크) `is_fresh()`를 검사하여 만료된 뷰를 다시 계산합니다.
3. **인터셉트**: `execute_sql` 호출 시 파싱된 SQL이 등록된 뷰 중 하나와 일치하고 캐시가 존재하면, 실제 쿼리 플래너를 돌리지 않고 캐시된 값을 즉시 반환합니다.

---

## 다음 단계

- [SQL 레퍼런스](sql-reference) — 지원되는 SQL 구문 확인
- [스트리밍 수집](streaming-ingestion) — 실시간 데이터 파이프라인 구축
- [컬럼형 캐시](storage-layers) — 분석 쿼리 최적화 계층 이해
