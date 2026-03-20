//! Transport 추상화 계층 — s2n-quic 기반, 런타임 설정 지원
//!
//! ## 런타임으로 Transport 선택
//! ```rust,no_run
//! use dbx_core::replication::transport::{ReplicationConfig, TransportMode};
//! use dbx_core::engine::parallel_engine::DbConfig;
//!
//! // InMemory (기본 — 단일 프로세스)
//! let config = DbConfig::default();  // replication: InMemory
//!
//! // QUIC (분산 배포)
//! let config = DbConfig {
//!     replication: ReplicationConfig::quic(
//!         "0.0.0.0:7878",
//!         "/etc/dbx/cert.pem",
//!         "/etc/dbx/key.pem",
//!         3, // cluster_size
//!     ),
//!     ..Default::default()
//! };
//! ```

use std::time::Duration;

use crate::replication::protocol::ReplicationMessage;

// ─────────────────────────────────────────────────────────────────────────────
// 런타임 Transport 설정
// ─────────────────────────────────────────────────────────────────────────────

/// Transport 동작 모드
#[derive(Debug, Clone, Default)]
pub enum TransportMode {
    /// 단일 프로세스 내 인메모리 통신 (기본)
    #[default]
    InMemory,
    /// QUIC 기반 네트워크 통신 (프로세스 간 / 분산 배포)
    Quic {
        /// 이 노드가 리스닝할 주소 (예: "0.0.0.0:7878")
        bind_addr: String,
        /// TLS 인증서 경로 (.pem)
        cert_path: String,
        /// TLS 개인키 경로 (.pem)
        key_path: String,
    },
}

/// 레플리케이션 전체 설정
///
/// `DbConfig::replication` 필드로 전달합니다.
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Transport 모드: `InMemory`(default) 또는 `Quic`
    pub mode: TransportMode,
    /// 클러스터 노드 수 (Quorum 계산용, 기본 1)
    pub cluster_size: usize,
    /// 레플리케이션 활성화 여부 (기본 false)
    pub enabled: bool,
    /// Quorum Write 타임아웃 (기본 5초)
    ///
    /// Master가 이 시간 내에 quorum ACK를 받지 못하면 `replicate()` 에러 반환.
    pub quorum_write_timeout: Duration,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: TransportMode::InMemory,
            cluster_size: 1,
            enabled: false,
            quorum_write_timeout: Duration::from_secs(5),
        }
    }
}

impl ReplicationConfig {
    /// InMemory 모드 (단일 부트 개발/테스트용)
    pub fn in_memory(cluster_size: usize) -> Self {
        Self {
            mode: TransportMode::InMemory,
            cluster_size,
            enabled: true,
            quorum_write_timeout: Duration::from_secs(5),
        }
    }

    /// QUIC 모드 (실제 분산 배포용)
    pub fn quic(
        bind_addr: impl Into<String>,
        cert_path: impl Into<String>,
        key_path: impl Into<String>,
        cluster_size: usize,
    ) -> Self {
        Self {
            mode: TransportMode::Quic {
                bind_addr: bind_addr.into(),
                cert_path: cert_path.into(),
                key_path: key_path.into(),
            },
            cluster_size,
            enabled: true,
            quorum_write_timeout: Duration::from_secs(5),
        }
    }

    /// 현재 모드 이름 (로그용)
    pub fn mode_name(&self) -> &'static str {
        match &self.mode {
            TransportMode::InMemory => "InMemory",
            TransportMode::Quic { .. } => "QUIC (s2n-quic)",
        }
    }

    /// Transport 빌드 (비동기)
    ///
    /// - `InMemory` → InMemoryTransport 즉시 반환
    /// - `Quic`    → s2n-quic 서버 소켓을 열고 QuicTransport 반환
    pub async fn build_transport_async(
        &self,
        broadcast_tx: tokio::sync::broadcast::Sender<ReplicationMessage>,
    ) -> Result<Box<dyn Transport>, quic::QuicError> {
        match &self.mode {
            TransportMode::InMemory => Ok(Box::new(InMemoryTransport::new(broadcast_tx))),
            TransportMode::Quic {
                bind_addr,
                cert_path,
                key_path,
            } => {
                tracing::info!(
                    "QUIC Transport 초기화: bind={} cert={}",
                    bind_addr,
                    cert_path
                );
                let (node, handle) = quic::QuicNode::server(
                    bind_addr,
                    std::path::Path::new(cert_path),
                    std::path::Path::new(key_path),
                )
                .await?;
                // 백그라운드 수신 루프 유지
                tokio::spawn(handle);
                Ok(Box::new(quic::QuicTransport::new(node)))
            }
        }
    }

    /// 동기 편의 함수 — InMemory 전용
    ///
    /// Quic 모드에서는 `build_transport_async()`를 사용하세요.
    pub fn build_inmemory(
        broadcast_tx: tokio::sync::broadcast::Sender<ReplicationMessage>,
    ) -> Box<dyn Transport> {
        Box::new(InMemoryTransport::new(broadcast_tx))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 공통 Transport Trait
// ─────────────────────────────────────────────────────────────────────────────

/// 노드 간 메시지 전송 추상화 Trait
///
/// `InMemoryTransport`(단일 프로세스)와 `QuicTransport`(분산 프로세스) 모두
/// 이 trait을 구현하므로 상위 코드는 Transport 구현체를 교체해도 수정 불필요.
pub trait Transport: Send + Sync {
    /// 특정 노드에게 메시지 전송
    fn send(&self, target_node_id: u32, msg: ReplicationMessage);

    /// 메시지 수신 (비블로킹, 없으면 None)  
    fn recv(&self) -> Option<ReplicationMessage>;
}

// ─────────────────────────────────────────────────────────────────────────────
// InMemoryTransport — 단일 프로세스용 (현재 MVP)
// ─────────────────────────────────────────────────────────────────────────────

/// 인메모리 Transport (단일 프로세스 내 노드 간 통신)
///
/// 테스트 및 단일 서버 시나리오에서 사용합니다.
/// `QuicTransport`로 교체 시 이 구조체만 대체하면 됩니다.
pub struct InMemoryTransport {
    sender: tokio::sync::broadcast::Sender<ReplicationMessage>,
    receiver: std::sync::Mutex<tokio::sync::broadcast::Receiver<ReplicationMessage>>,
}

impl InMemoryTransport {
    pub fn new(sender: tokio::sync::broadcast::Sender<ReplicationMessage>) -> Self {
        let receiver = sender.subscribe();
        Self {
            sender,
            receiver: std::sync::Mutex::new(receiver),
        }
    }
}

impl Transport for InMemoryTransport {
    fn send(&self, _target_node_id: u32, msg: ReplicationMessage) {
        let _ = self.sender.send(msg);
    }

    fn recv(&self) -> Option<ReplicationMessage> {
        self.receiver.lock().unwrap().try_recv().ok()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// QuicTransport — 프로세스 간 통신 (s2n-quic 기반)
// ─────────────────────────────────────────────────────────────────────────────

pub mod quic {
    //! s2n-quic 기반 실제 네트워크 Transport 구현
    //!
    //! ## 사용법
    //! ```rust,ignore
    //! use dbx_core::replication::transport::quic::QuicNode;
    //! use std::path::Path;
    //!
    //! // 서버 (수신 노드)
    //! let (server, handle) = QuicNode::server(
    //!     "0.0.0.0:7878",
    //!     Path::new("/etc/dbx/cert.pem"),
    //!     Path::new("/etc/dbx/key.pem"),
    //! ).await?;
    //! tokio::spawn(handle);
    //!
    //! // 클라이언트 (발신 노드)
    //! let (client, _handle) = QuicNode::client(
    //!     "10.0.0.2:7878",
    //!     Path::new("/etc/dbx/ca.pem"),
    //! ).await?;
    //! ```

    use s2n_quic::provider::tls;
    use s2n_quic::{Client, Server};
    use std::path::Path;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    use crate::replication::protocol::ReplicationMessage;

    /// 메시지 길이 프리픽스 크기 (4바이트 little-endian u32)
    const LEN_PREFIX_BYTES: usize = 4;

    /// QUIC 노드 에러
    #[derive(Debug, thiserror::Error)]
    pub enum QuicError {
        #[error("QUIC 연결 실패: {0}")]
        ConnectionError(String),
        #[error("직렬화 실패: {0}")]
        SerializeError(String),
        #[error("IO 오류: {0}")]
        IoError(#[from] std::io::Error),
    }

    /// QUIC 기반 노드 (Server/Client 겸용)
    pub struct QuicNode {
        /// 수신 채널 (recv_loop에서 디코딩된 메시지가 들어옴)
        rx: Arc<tokio::sync::Mutex<mpsc::Receiver<ReplicationMessage>>>,
        /// 발신 채널 (send_msg를 통해 메시지 전송)
        tx_out: mpsc::Sender<(ReplicationMessage, String)>,
    }

    impl QuicNode {
        /// 서버 모드 시작
        ///
        /// `bind_addr`: 바인드할 주소, 예: "0.0.0.0:7878"
        /// `cert_pem` / `key_pem`: TLS 인증서/키 (PEM 형식)
        pub async fn server(
            bind_addr: &str,
            cert_pem: &Path,
            key_pem: &Path,
        ) -> Result<(Self, tokio::task::JoinHandle<()>), QuicError> {
            let (msg_tx, msg_rx) = mpsc::channel::<ReplicationMessage>(256);
            let (out_tx, _out_rx) = mpsc::channel::<(ReplicationMessage, String)>(256);

            let tls = tls::default::Server::builder()
                .with_certificate(cert_pem, key_pem)
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .build()
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?;

            let mut server = Server::builder()
                .with_tls(tls)
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .with_io(bind_addr)
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .start()
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?;

            // 백그라운드 수신 루프
            let handle = tokio::spawn(async move {
                while let Some(mut conn) = server.accept().await {
                    let msg_tx = msg_tx.clone();
                    tokio::spawn(async move {
                        while let Ok(Some(mut stream)) = conn.accept_bidirectional_stream().await {
                            // 메시지 수신: [4바이트 길이][메시지 바이트]
                            use tokio::io::AsyncReadExt;
                            let mut len_buf = [0u8; LEN_PREFIX_BYTES];
                            if stream.read_exact(&mut len_buf).await.is_err() {
                                break;
                            }
                            let len = u32::from_le_bytes(len_buf) as usize;
                            let mut buf = vec![0u8; len];
                            if stream.read_exact(&mut buf).await.is_err() {
                                break;
                            }
                            if let Ok(msg) = bincode::deserialize::<ReplicationMessage>(&buf) {
                                let _ = msg_tx.send(msg).await;
                            }
                        }
                    });
                }
            });

            Ok((
                Self {
                    rx: Arc::new(tokio::sync::Mutex::new(msg_rx)),
                    tx_out: out_tx,
                },
                handle,
            ))
        }

        /// 클라이언트 모드 — 상대 노드 주소로 연결
        ///
        /// `peer_addr`: 연결할 주소, 예: "10.0.0.2:7878"
        /// `ca_cert_pem`: 서버 인증서 검증용 CA 인증서
        pub async fn client(
            peer_addr: &str,
            ca_cert_pem: &Path,
        ) -> Result<(Self, tokio::task::JoinHandle<()>), QuicError> {
            let (msg_tx, msg_rx) = mpsc::channel::<ReplicationMessage>(256);
            let (out_tx, mut out_rx) = mpsc::channel::<(ReplicationMessage, String)>(256);

            let tls = tls::default::Client::builder()
                .with_certificate(ca_cert_pem)
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .build()
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?;

            let client = Client::builder()
                .with_tls(tls)
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .with_io("0.0.0.0:0") // 임의 포트
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?
                .start()
                .map_err(|e| QuicError::ConnectionError(e.to_string()))?;

            let peer = peer_addr.to_string();

            // 백그라운드 발신 루프
            let handle = tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let connect =
                    s2n_quic::client::Connect::new(peer.parse::<std::net::SocketAddr>().unwrap())
                        .with_server_name("dbx-node");

                if let Ok(mut conn) = client.connect(connect).await {
                    conn.keep_alive(true).ok();
                    while let Some((msg, _addr)) = out_rx.recv().await {
                        let Ok(bytes) = bincode::serialize(&msg) else {
                            continue;
                        };
                        let len = bytes.len() as u32;
                        if let Ok(mut stream) = conn.open_bidirectional_stream().await {
                            let _ = stream.write_all(&len.to_le_bytes()).await;
                            let _ = stream.write_all(&bytes).await;
                        }
                    }
                }
                // 연결 끊기면 수신 채널도 닫힘
                drop(msg_tx);
            });

            Ok((
                Self {
                    rx: Arc::new(tokio::sync::Mutex::new(msg_rx)),
                    tx_out: out_tx,
                },
                handle,
            ))
        }

        /// 메시지 발송 (비동기)
        pub async fn send_msg(&self, msg: ReplicationMessage, peer_addr: String) {
            let _ = self.tx_out.send((msg, peer_addr)).await;
        }

        /// 메시지 수신 (비블로킹)
        pub async fn try_recv(&self) -> Option<ReplicationMessage> {
            self.rx.lock().await.try_recv().ok()
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // QuicTransport — Transport trait 구현체 (QuicNode 래퍼)
    // ─────────────────────────────────────────────────────────────────────

    /// `Transport` trait을 구현하는 QUIC 래퍼
    ///
    /// `QuicNode`(비동기)를 동기 `Transport` trait에 연결합니다.
    /// 내부적으로 현재 tokio 런타임 핸들을 사용하여 비동기 호출을 브릿지합니다.
    pub struct QuicTransport {
        node: Arc<QuicNode>,
        /// 동기 컨텍스트에서 비동기 호출용 런타임 핸들
        rt: tokio::runtime::Handle,
        /// 연결할 피어 주소 (클라이언트 측 발신 전용)
        peer_addr: String,
    }

    impl QuicTransport {
        pub fn new(node: QuicNode) -> Self {
            Self {
                node: Arc::new(node),
                rt: tokio::runtime::Handle::current(),
                peer_addr: String::new(),
            }
        }

        pub fn with_peer(node: QuicNode, peer_addr: impl Into<String>) -> Self {
            Self {
                node: Arc::new(node),
                rt: tokio::runtime::Handle::current(),
                peer_addr: peer_addr.into(),
            }
        }
    }

    impl crate::replication::transport::Transport for QuicTransport {
        fn send(&self, _target_node_id: u32, msg: ReplicationMessage) {
            let node = Arc::clone(&self.node);
            let addr = self.peer_addr.clone();
            self.rt.spawn(async move {
                node.send_msg(msg, addr).await;
            });
        }

        fn recv(&self) -> Option<ReplicationMessage> {
            let node = Arc::clone(&self.node);
            self.rt.block_on(async move { node.try_recv().await })
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // 개발용 자가서명 인증서 생성 헬퍼
    // ─────────────────────────────────────────────────────────────────────

    /// 개발 환경용 자가서명 TLS 인증서/키를 임시 디렉토리에 생성
    ///
    /// 프로덕션에서는 실제 CA 인증서를 사용하세요.
    pub fn generate_self_signed_cert(
        output_dir: &Path,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), QuicError> {
        let cert_path = output_dir.join("cert.pem");
        let key_path = output_dir.join("key.pem");

        let status = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-keyout",
                key_path.to_str().unwrap(),
                "-out",
                cert_path.to_str().unwrap(),
                "-days",
                "365",
                "-nodes",
                "-subj",
                "/CN=dbx-node",
            ])
            .status()
            .map_err(QuicError::IoError)?;

        if !status.success() {
            return Err(QuicError::ConnectionError(
                "openssl 실행 실패 (openssl이 설치되어 있어야 함)".to_string(),
            ));
        }

        Ok((cert_path, key_path))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 테스트
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_transport_roundtrip() {
        let (tx, _) = tokio::sync::broadcast::channel(16);
        let t1 = InMemoryTransport::new(tx.clone());
        let t2 = InMemoryTransport::new(tx.clone());

        let msg = ReplicationMessage::Heartbeat {
            node_id: 1,
            lsn: 42,
        };
        t1.send(2, msg.clone());

        // t2는 같은 채널을 구독하므로 메시지를 받을 수 있음
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let received = t2.recv();
        assert!(received.is_some());
        assert_eq!(received.unwrap(), msg);
    }

    #[test]
    fn test_transport_trait_object() {
        // Transport trait object로 사용 가능한지 컴파일 타임 검증
        let (tx, _) = tokio::sync::broadcast::channel::<ReplicationMessage>(16);
        let _t: Box<dyn Transport> = Box::new(InMemoryTransport::new(tx));
    }
}
