---
layout: default
title: 스케줄러
parent: 한국어
nav_order: 32
---

# 작업 스케줄러 (Job Scheduler)
{: .no_toc }

주기적인 유지보수 작업이나 배치 처리를 자동으로 실행합니다.
{: .fs-6 .fw-300 }

---

## 개요

스케줄러는 Cron 표현식 기반으로 작업을 등록하고 주기적으로 실행하는 시스템입니다.

---

## 작업 등록

```rust
use dbx_core::automation::scheduler::{JobScheduler, Job, Schedule};

let mut scheduler = JobScheduler::new();

// 매 5분마다 실행되는 통계 갱신 작업
scheduler.schedule(Job::new(
    "refresh_stats",
    Schedule::cron("*/5 * * * *"),  // 5분 간격
    || {
        println!("통계 테이블 갱신 중...");
        Ok(())
    },
));

// 매일 자정에 실행되는 정리 작업
scheduler.schedule(Job::new(
    "daily_cleanup",
    Schedule::cron("0 0 * * *"),    // 매일 00:00
    || {
        println!("만료된 세션 정리 중...");
        Ok(())
    },
));
```

---

## Schedule 표현식

| 표현식 | 설명 |
|--------|------|
| `*/5 * * * *` | 5분마다 |
| `0 * * * *` | 매시 정각 |
| `0 0 * * *` | 매일 자정 |
| `0 0 * * 0` | 매주 일요일 자정 |
| `0 0 1 * *` | 매월 1일 자정 |

형식: `분 시 일 월 요일`

---

## 작업 상태 관리

```rust
// 등록된 작업 목록 조회
let jobs = scheduler.list_jobs();
for job in &jobs {
    println!("{}: 다음 실행 {:?}", job.name, job.next_run);
}

// 특정 작업 비활성화
scheduler.disable("refresh_stats");

// 즉시 실행
scheduler.run_now("daily_cleanup")?;
```

---

## 활용 사례

| 작업 | 주기 | 설명 |
|------|------|------|
| WAL 체크포인트 | 5분 | 메모리 데이터를 디스크에 동기화 |
| 인덱스 최적화 | 매일 | 조각난 인덱스 재구축 |
| 통계 갱신 | 1시간 | 쿼리 최적화용 테이블 통계 업데이트 |
| 만료 데이터 정리 | 매일 | TTL 만료된 행 삭제 |

---

## 다음 단계

- [트리거 가이드](triggers) — 이벤트 기반 자동 실행
- [기능 플래그 가이드](feature-flags) — 스케줄러 기능 토글
