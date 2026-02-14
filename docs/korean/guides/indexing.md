---
layout: default
title: 인덱싱
parent: Guides
nav_order: 10
---

# 인덱싱 (Indexing)
{: .no_toc }

DBX는 Bloom Filter 기반의 인덱싱 시스템을 제공하여 데이터 조회 성능을 극대화합니다.
{: .fs-6 .fw-300 }

---

## 개요

DBX의 인덱스는 **확률적 자료구조(Bloom Filter)**를 사용하여 메모리 효율성을 극대화하면서도 조회 속도를 수백 배 향상시킵니다.

### 주요 특징

- **O(1) 시간 복잡도**: 데이터의 양과 상관없이 일정한 속도로 존재 여부 확인
- **메모리 효율성**: 개당 약 10바이트 내외의 매우 적은 메모리 사용
- **높은 정확도**: False Positive(오탐) 확률을 1% 미만으로 유지

---

## 기본 사용법

### 인덱스 생성 및 활용

특정 컬럼에 인덱스를 생성하면 `get`이나 SQL 쿼리 시 자동으로 성능이 향상됩니다.

```rust
use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open_in_memory()?;
    
    // 'users' 테이블의 'email' 컬럼에 인덱스 생성
    db.create_index("users", "email")?;
    
    // 데이터 삽입 (자동으로 인덱스 업데이트)
    db.insert("users", b"key:1", b"alice@example.com")?;
    
    // 인덱스를 활용한 빠른 조회
    let results = db.index_lookup("users", "email", b"alice@example.com")?;
    
    Ok(())
}
```

---

## 성능 벤치마크 (1만 건 기준)

| 방식 | 소요 시간 | 속도 차이 |
|------|------------|--------------|
| 전체 스캔 (Full Scan) | 약 5ms | 1배 |
| **인덱스 조회 (Index)** | **약 12μs** | **약 400배 빨름** |

---

## 관리 및 최적화

### 인덱스 재구축 (Rebuild)

대량의 데이터 삽입이나 삭제 후에는 인덱스를 재구축하여 정확도와 성능을 최적화할 수 있습니다.

```rust
db.rebuild_index("users")?;
```

### 주의 사항 및 한계

- **확률적 구조**: Bloom Filter 특성상 "존재하지 않음"은 확실히 알 수 있으나, "존재함"은 100% 확실하지 않을 수 있습니다(오탐 발생 가능). 하지만 조회 속도 향상을 위한 필터로서 매우 유용합니다.
- **지원 쿼리**: 일치(Equality) 조회는 매우 빠르지만, 범위(Range) 조회나 정렬(Sorting)에는 사용되지 않습니다.
- **메모리 사용**: 인덱스 수가 많아질수록 서버 메모리 사용량이 늘어나므로, 자주 조회되는 컬럼 위주로 생성하세요.

---

## 다음 단계

- [SQL 레퍼런스](sql-reference) — SQL 쿼리에서 인덱스 자동 활용
- [GPU 가속](gpu-acceleration) — 대용량 인덱싱 데이터의 가속 처리
- [저장소 계층](storage-layers) — 인덱스가 위치한 Tier 4 계층 이해
