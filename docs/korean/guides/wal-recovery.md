---
layout: default
title: WAL 복구
parent: Guides
nav_order: 11
---

# WAL 복구 (WAL Recovery)
{: .no_toc }

DBX는 정전이나 시스템 장애 시에도 데이터 무결성을 유지하기 위해 **Write-Ahead Log (WAL)** 시스템을 사용합니다.
{: .fs-6 .fw-300 }

---

## 개요

WAL은 모든 데이터 변경 사항을 실제 데이터베이스 파일에 반영하기 전에 먼저 로그 파일에 기록하는 기술입니다. 이를 통해 시스템이 비정상적으로 종료되더라도 재시작 시 로그를 다시 실행(Replay)하여 데이터를 완벽하게 복구할 수 있습니다.

### 주요 특징

- **크래시 복구**: 비정상 종료 후 재시작 시 자동 복구 수행
- **원자성 보장**: 트랜잭션 도중 장애 발생 시 부분적인 데이터 쓰기를 방지
- **최적화된 쓰기**: 랜덤 I/O 대신 순차 I/O를 사용하여 성능 저하 최소화

---

## 작동 방식

1. **로그 기록**: 사용자가 데이터를 삽입/수정하면 먼저 디스크의 `wal.log` 파일에 순차적으로 기록합니다.
2. **메모리 갱신**: 로그 기록 후 인메모리 **Delta Store**를 갱신합니다.
3. **체크포인트**: 주기적으로 또는 시스템 여유 시점에 WAL의 내용을 **WOS(영구 저장소)**에 반영하고 로그를 정리합니다.

---

## 내구성 수준 (Durability Levels)

데이터의 중요도에 따라 내구성 수준을 선택할 수 있습니다.

| 레벨 | 특징 | 성능 | 권장 용도 |
|------|-----------|------|----------|
| **None** | WAL 미사용 | 최고 | 임시 캐시 데이터 |
| **Lazy** | 주기적 디스크 동기화 | 높음 | **기본값**, 일반적인 앱 |
| **Full** | 매 쓰기마다 즉시 동기화 | 중간 | 금융, 의료 등 치명적 데이터 |

---

## 복구 시뮬레이션

```rust
use dbx_core::Database;

// 1. 데이터 삽입 중 시스템 장애 발생 가정
{
    let db = Database::open("./my_db")?;
    db.insert("users", b"user:1", b"Alice")?;
    // 비정상 종료 (flush/checkpoint 호출 전)
}

// 2. 재시작 시 자동 복구
{
    let db = Database::open("./my_db")?;
    let value = db.get("users", b"user:1")?;
    assert_eq!(value, Some(b"Alice".to_vec())); // 데이터가 안전하게 복구됨
}
```

---

## 다음 단계

- [트랜잭션 가이드](transactions) — WAL과 MVCC 조화
- [저장소 계층](storage-layers) — WOS와 ROS 간의 데이터 이동 이해
- [암호화 가이드](encryption) — 암호화된 WAL 설정
