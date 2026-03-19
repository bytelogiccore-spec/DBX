---
layout: default
title: QUIC Replication
parent: English
nav_order: 25
---

# QUIC Distributed Replication

DBX supports real network replication between processes using AWS's `s2n-quic` library.

---

## InMemory vs QUIC

| | InMemory | QUIC |
|--|----------|------|
| Use case | Single server, dev/test | Distributed deployment, multi-server |
| Network required | ❌ | ✅ |
| TLS security | ❌ | ✅ (TLS 1.3 built-in) |
| Latency | ~tens of ns | ~tens of µs (LAN) |
| Configuration | None | cert/key paths required |

---

## Runtime Configuration Switch

Change only `ReplicationConfig` — no code changes required.

```rust
use dbx_core::replication::transport::ReplicationConfig;
use dbx_core::engine::parallel_engine::DbConfig;

// InMemory (default — development)
let config = DbConfig::default();

// QUIC (production)
let config = DbConfig {
    replication: ReplicationConfig::quic(
        "0.0.0.0:7878",       // address this node binds to
        "/etc/dbx/cert.pem",  // TLS certificate
        "/etc/dbx/key.pem",   // TLS private key
        3,                     // cluster node count (for quorum)
    ),
    ..Default::default()
};

// Initialize transport (async)
let (tx, _) = tokio::sync::broadcast::channel(64);
let transport = config.replication.build_transport_async(tx).await?;
```

---

## TLS Certificate Setup

### Development — Self-Signed Certificate

```rust
use dbx_core::replication::transport::quic::generate_self_signed_cert;
use std::path::Path;

let (cert, key) = generate_self_signed_cert(Path::new("/tmp/dbx-certs"))?;
// cert = /tmp/dbx-certs/cert.pem
// key  = /tmp/dbx-certs/key.pem
```

Or generate with openssl CLI:

```bash
openssl req -x509 -newkey rsa:2048 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes -subj "/CN=dbx-node"
```

### Production

Use certificates issued by a real CA (Let's Encrypt, AWS ACM, etc.).

---

## Direct QuicNode Usage

For cases requiring direct server/client control:

```rust
use dbx_core::replication::transport::quic::QuicNode;
use std::path::Path;

// Start server (receiving node)
let (server_node, handle) = QuicNode::server(
    "0.0.0.0:7878",
    Path::new("/etc/dbx/cert.pem"),
    Path::new("/etc/dbx/key.pem"),
).await?;
tokio::spawn(handle);

// Receive messages
if let Some(msg) = server_node.try_recv().await {
    // handle message
}

// Connect client (sending node)
let (client_node, handle) = QuicNode::client(
    "10.0.0.2:7878",
    Path::new("/etc/dbx/ca-cert.pem"),
).await?;
tokio::spawn(handle);

// Send message
client_node.send_msg(msg, "10.0.0.2:7878".to_string()).await;
```

---

## Multi-Node Cluster Example

```
Server A (Master)         Server B (Slave 1)      Server C (Slave 2)
┌─────────────────┐      ┌─────────────────┐    ┌─────────────────┐
│ QuicNode::server│ QUIC │ QuicNode::server│    │ QuicNode::server│
│ 0.0.0.0:7878   │◄────►│ 0.0.0.0:7878   │◄──►│ 0.0.0.0:7878   │
└─────────────────┘      └─────────────────┘    └─────────────────┘
```

Run the same code on each server, varying only the `bind_addr`.

---

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Same-datacenter latency | ~0.3ms |
| Stream multiplexing | ✅ (No HoL Blocking) |
| TLS handshake | 1-RTT (0-RTT on reconnect) |
| Underlying library | `s2n-quic` v1.76 (AWS) |

---

## Related Modules

| File | Role |
|------|------|
| `replication/transport.rs` | Transport trait, ReplicationConfig, QuicTransport |
| `replication/transport.rs::quic` | QuicNode server/client, certificate helper |
