---
layout: default
title: 스트리밍 수집 (Streaming Ingestion)
parent: 한국어
nav_order: 23
description: "고성능 실시간 데이터 파이프라인 구축 가이드"
---

# 스트리밍 수집 (Streaming Ingestion)
{: .no_toc }

스트리밍 수집은 외부 데이터 소스나 실시간 이벤트 스트림으로부터 DBX로 데이터를 끊김 없이 흘려보내는 고성능 파이프라인 기능입니다.
{: .fs-6 .fw-300 }

## 목차
{: .no_toc .text-delta }

1. TOC
{:toc}

---

## 개요

DBX의 **StreamIngester**는 배치 처리를 최적화하여 쓰기 성능을 극대화합니다. 개별 `insert()` 호출보다 훨씬 높은 처리량(Throughput)을 제공하며, CDC(Change Data Capture) 패턴을 지원합니다.

### 주요 특징
- **MPSC 파이프라인**: 멀티 프로듀서-싱글 컨슈머 모델로 여러 소스에서 안전하게 수집합니다.
- **자동 배치 (Automatic Batching)**: 설정된 `batch_size`에 도달하거나 `max_latency` 시간이 지나면 자동으로 flush를 수행합니다.
- **Full DML 지원**: `Insert`뿐만 아니라 `Update`, `Delete` 이벤트를 실시간으로 처리할 수 있습니다.

---

## 시작하기

`Database` 인스턴스에서 직접 `StreamIngester`를 생성할 수 있습니다.

### 예제: 실시간 원격 분석 데이터 수집

```rust
use dbx_core::{Database, engine::stream_ingester::StreamEvent};
use std::time::Duration;

fn main() -> dbx_core::DbxResult<()> {
    let db = Database::open("./data")?;
    
    // 테이블 'telemetry'에 대해 500건 단위 혹은 최대 100ms 지연으로 수집기 생성
    let ingester = db.create_stream_ingester("telemetry", 500, 100);
    let sender = ingester.sender();

    // 여러 스레드나 비동기 태스크에서 sender를 clone하여 사용 가능
    let tx = sender.clone();
    std::thread::spawn(move || {
        for i in 0..1000 {
            let event = StreamEvent::Insert {
                key: format!("device:{}", i).into_bytes(),
                value: format!("{{\"temp\": {}}}", 20 + (i % 10)).into_bytes(),
            };
            tx.send(vec![event]).unwrap();
        }
    });

    // 모든 작업이 끝나면 flush를 호출하여 남아있는 버퍼를 저장하고 종료합니다.
    ingester.flush()?;
    
    Ok(())
}
```

---

## 이벤트 유형 (StreamEvent)

스트리밍 엔진은 세 가지 유형의 이벤트를 처리합니다.

- **`Insert`**: 새로운 키-값 쌍을 추가합니다.
- **`Update`**: 기존 키의 값을 갱신합니다. (내부적으로 Upsert로 동작)
- **`Delete`**: 특정 키를 삭제합니다.

---

## 성능 최적화 옵션

| 옵션 | 설명 | 권장 설정 |
|:---|:---|:---|
| `batch_size` | 한 번에 저장할 레코드 수 | 1,000 ~ 10,000 |
| `max_latency` | flush가 발생하기 전 최대 대기 시간 (ms) | 100ms ~ 1,000ms |

> [!TIP]
> 처리량을 최대화하려면 `batch_size`를 키우고, 실시간성이 중요하다면 `max_latency`를 낮게 설정하세요.

---

## 내부 동작

1. **채널 수집**: `sender`를 통해 들어온 이벤트들은 백그라운드 스레드의 `mpsc` 채널로 들어갑니다.
2. **버퍼링**: 백그라운드 스레드는 이벤트를 로컬 벡터에 쌓습니다.
3. **자동 Flush**: 다음 조건 중 하나라도 만족하면 `Database::insert_batch()` 등을 호출하여 Tier 1(Delta Store)에 기록합니다.
   - 벡터 크기 >= `batch_size`
   - 마지막 flush 이후 대기 시간 >= `max_latency`
4. **종료**: `ingester.flush()` 호출 시 남아있는 모든 데이터를 처리하고 백그라운드 스레드를 안전하게 종료(join)합니다.

---

## 다음 단계

- [CRUD 작업](crud-operations) — 일반적인 데이터 삽입 방법
- [구체화된 뷰](materialized-views) — 수집된 데이터에 대한 실시간 분석 뷰 구축
- [DB 설정](db-config) — 내구성(Durability) 및 동기화 설정 최적화
