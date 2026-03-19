---
layout: default
title: QUIC 분산 레플리케이션
nav_order: 25
parent: 가이드
grand_parent: 한국어
---

# QUIC 분산 레플리케이션

DBX는 AWS의 `s2n-quic`을 기반으로 프로세스 간 실제 네트워크 레플리케이션을 지원합니다.

---

## InMemory vs QUIC

| 항목 | InMemory | QUIC |
|------|----------|------|
| 사용 시나리오 | 단일 서버, 개발/테스트 | 분산 배포, 멀티 서버 |
| 네트워크 필요 | ❌ | ✅ |
| TLS 보안 | ❌ | ✅ (TLS 1.3 기본) |
| 레이턴시 | 수십 ns | ~수십 µs (LAN) |
| 설정 | 없음 | cert/key 경로 필요 |

---

## 런타임 설정으로 전환

코드 수정 없이 `ReplicationConfig`만 변경하면 됩니다.

```rust
use dbx_core::replication::transport::ReplicationConfig;
use dbx_core::engine::parallel_engine::DbConfig;

// InMemory (기본 — 개발 환경)
let config = DbConfig::default();

// QUIC (운영 환경)
let config = DbConfig {
    replication: ReplicationConfig::quic(
        "0.0.0.0:7878",       // 이 노드가 바인딩할 주소
        "/etc/dbx/cert.pem",  // TLS 인증서
        "/etc/dbx/key.pem",   // TLS 개인키
        3,                     // 클러스터 노드 수 (Quorum 계산용)
    ),
    ..Default::default()
};

// Transport 초기화 (비동기)
let (tx, _) = tokio::sync::broadcast::channel(64);
let transport = config.replication.build_transport_async(tx).await?;
```

---

## TLS 인증서 준비

### 개발 환경 — 자가서명 인증서

```rust
use dbx_core::replication::transport::quic::generate_self_signed_cert;
use std::path::Path;

let (cert, key) = generate_self_signed_cert(Path::new("/tmp/dbx-certs"))?;
// cert = /tmp/dbx-certs/cert.pem
// key  = /tmp/dbx-certs/key.pem
```

또는 openssl CLI로 직접 생성:

```bash
openssl req -x509 -newkey rsa:2048 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj "/CN=dbx-node"
```

### 운영 환경

실제 CA(Let's Encrypt, AWS ACM 등)에서 발급한 인증서를 사용하세요.

---

## QuicNode 직접 사용

서버/클라이언트를 직접 제어해야 하는 경우:

```rust
use dbx_core::replication::transport::quic::QuicNode;
use std::path::Path;

// 서버 시작 (수신 노드)
let (server_node, handle) = QuicNode::server(
    "0.0.0.0:7878",
    Path::new("/etc/dbx/cert.pem"),
    Path::new("/etc/dbx/key.pem"),
).await?;
tokio::spawn(handle);

// 메시지 수신
if let Some(msg) = server_node.try_recv().await {
    // 메시지 처리
}

// 클라이언트 연결 (발신 노드)
let (client_node, handle) = QuicNode::client(
    "10.0.0.2:7878",
    Path::new("/etc/dbx/ca-cert.pem"),
).await?;
tokio::spawn(handle);

// 메시지 발송
client_node.send_msg(msg, "10.0.0.2:7878".to_string()).await;
```

---

## 다중 노드 클러스터 구성 예시

```
서버 A (Master)          서버 B (Slave 1)       서버 C (Slave 2)
┌─────────────────┐      ┌─────────────────┐    ┌─────────────────┐
│ QuicNode::server│ QUIC │ QuicNode::server│    │ QuicNode::server│
│ 0.0.0.0:7878   │◄────►│ 0.0.0.0:7878   │◄──►│ 0.0.0.0:7878   │
└─────────────────┘      └─────────────────┘    └─────────────────┘
```

각 서버에서 동일한 코드를 실행하되 `bind_addr`만 다르게 설정합니다.

---

## 성능 특성

| 항목 | 수치 |
|------|------|
| 동일 데이터센터 레이턴시 | ~0.3ms |
| 스트림 다중화 | ✅ (HoL Blocking 없음) |
| TLS 핸드쉐이크 | 1-RTT (재연결 시 0-RTT) |
| 기반 라이브러리 | `s2n-quic` v1.76 (AWS) |

---

## 관련 모듈

| 파일 | 역할 |
|------|------|
| `replication/transport.rs` | Transport trait, ReplicationConfig, QuicTransport |
| `replication/transport.rs::quic` | QuicNode 서버/클라이언트, 인증서 헬퍼 |
