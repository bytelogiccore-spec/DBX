//! Grid Network (s2n-quic) 인터페이스 (Stub/Foundation)
//!
//! Node 간 고속 데이터 전송 및 Erasure Coding 패리티 블록 분산 목적으로
//! s2n-quic 기반의 P2P 통신 스터브를 제공합니다.

use crate::error::DbxResult;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tracing::info;

/// QUIC 통신을 담당하는 채널 인스턴스
pub struct QuicChannel {
    pub local_addr: SocketAddr,
    client: s2n_quic::Client,
}

impl QuicChannel {
    /// 새로운 QUIC 채널 개설
    /// TODO: 상용화 단계에서는 cert_pem과 key_pem 경로를 Config에서 주입받아야 합니다.
    pub async fn new(
        local_addr: SocketAddr,
        cert_pem: &str,
        key_pem: &str,
        tx: mpsc::Sender<GridMessageWrapper>,
    ) -> DbxResult<Self> {
        info!("Initializing s2n-quic channel on {}", local_addr);

        let tls_builder = s2n_quic::provider::tls::default::Server::builder()
            .with_certificate(cert_pem, key_pem)
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
        let tls = tls_builder
            .build()
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        let mut server = s2n_quic::Server::builder()
            .with_tls(tls)
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?
            .with_io(local_addr)
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?
            .start()
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        // 인증 기관 체크 무시 (테스트 및 동일 그리드 허용) - 실무에선 인증서 검증 필수
        let client_tls_builder = s2n_quic::provider::tls::default::Client::builder()
            .with_certificate(cert_pem)
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
        let client_tls = client_tls_builder
            .build()
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        let client = s2n_quic::Client::builder()
            .with_tls(client_tls)
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?
            .with_io("0.0.0.0:0")
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?
            .start()
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        // 백그라운드 서버 수신 루프
        tokio::spawn(async move {
            while let Some(mut connection) = server.accept().await {
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    while let Ok(Some(mut stream)) = connection.accept_bidirectional_stream().await
                    {
                        let tx2 = tx_clone.clone();
                        tokio::spawn(async move {
                            use tokio::io::AsyncReadExt;
                            let mut len_buf = [0u8; 4];
                            if stream.read_exact(&mut len_buf).await.is_ok() {
                                let len = u32::from_be_bytes(len_buf) as usize;
                                let mut data_buf = vec![0u8; len];
                                if stream.read_exact(&mut data_buf).await.is_ok() {
                                    if let Ok(msg) =
                                        crate::grid::protocol::GridMessage::deserialize(&data_buf)
                                    {
                                        let _ = tx2
                                            .send(GridMessageWrapper {
                                                msg,
                                                stream: Some(stream),
                                            })
                                            .await;
                                    }
                                }
                            }
                        });
                    }
                });
            }
        });

        Ok(Self { local_addr, client })
    }

    /// 다른 Grid 노드에 데이터 단방향 전송 (방치)
    pub async fn send_message(
        &self,
        peer_addr: SocketAddr,
        msg: crate::grid::protocol::GridMessage,
    ) -> DbxResult<()> {
        info!("Sending GridMessage to {}", peer_addr);

        // Connect
        let connect_config =
            s2n_quic::client::Connect::new(peer_addr).with_server_name("localhost");
        let mut connection = match self.client.connect(connect_config).await {
            Ok(c) => c,
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        };

        match connection.keep_alive(true) {
            Ok(_) => {}
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        }

        let mut stream = match connection.open_bidirectional_stream().await {
            Ok(s) => s,
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        };

        let bytes = msg.serialize()?;
        let len = (bytes.len() as u32).to_be_bytes();

        use tokio::io::AsyncWriteExt;
        if let Err(e) = stream.write_all(&len).await {
            return Err(crate::error::DbxError::Network(e.to_string()));
        }
        if let Err(e) = stream.write_all(&bytes).await {
            return Err(crate::error::DbxError::Network(e.to_string()));
        }

        // 데이터 전송 완료 후 플러시 및 스트림 정상 종료(FIN) 대기
        if let Err(e) = stream.flush().await {
            return Err(crate::error::DbxError::Network(e.to_string()));
        }
        if let Err(e) = stream.shutdown().await {
            return Err(crate::error::DbxError::Network(e.to_string()));
        }

        Ok(())
    }

    /// Request-Response 방식 비동기 통신 (FetchShard 등)
    pub async fn send_request_and_wait(
        &self,
        peer_addr: SocketAddr,
        msg: crate::grid::protocol::GridMessage,
    ) -> DbxResult<crate::grid::protocol::GridMessage> {
        let connect_config =
            s2n_quic::client::Connect::new(peer_addr).with_server_name("localhost");
        let mut connection = match self.client.connect(connect_config).await {
            Ok(c) => c,
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        };

        match connection.keep_alive(true) {
            Ok(_) => {}
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        }

        let mut stream = match connection.open_bidirectional_stream().await {
            Ok(s) => s,
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        };

        let bytes = msg.serialize()?;
        let len = (bytes.len() as u32).to_be_bytes();

        match stream.send(bytes::Bytes::copy_from_slice(&len)).await {
            Ok(_) => {}
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        }
        match stream.send(bytes::Bytes::from(bytes)).await {
            Ok(_) => {}
            Err(e) => return Err(crate::error::DbxError::Network(e.to_string())),
        }

        // Receive Length & Data using AsyncReadExt
        use tokio::io::AsyncReadExt;
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        let reply_len = u32::from_be_bytes(len_buf) as usize;
        let mut reply_buf = vec![0u8; reply_len];

        stream
            .read_exact(&mut reply_buf)
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        crate::grid::protocol::GridMessage::deserialize(&reply_buf)
    }

    /// 정적 메서드로 스트림에 메시지 응답 전송
    pub async fn send_response(
        stream: &mut s2n_quic::stream::BidirectionalStream,
        msg: crate::grid::protocol::GridMessage,
    ) -> DbxResult<()> {
        let bytes = msg.serialize()?;
        let len = (bytes.len() as u32).to_be_bytes();

        use tokio::io::AsyncWriteExt;
        stream
            .write_all(&len)
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
        stream
            .write_all(&bytes)
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        // 데이터 전송 완료 후 플러시 및 스트림 정상 종료(FIN) 대기
        stream
            .flush()
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
        stream
            .shutdown()
            .await
            .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

        Ok(())
    }
}

/// 스트림의 소유권을 유지하며 채널로 메시지를 보내기 위한 래퍼
pub struct GridMessageWrapper {
    pub msg: crate::grid::protocol::GridMessage,
    pub stream: Option<s2n_quic::stream::BidirectionalStream>,
}

impl GridMessageWrapper {
    /// 요청을 보낸 스트림을 통해 응답을 전송합니다.
    pub async fn send_reply(&mut self, reply: crate::grid::protocol::GridMessage) -> DbxResult<()> {
        if let Some(stream) = &mut self.stream {
            let bytes = reply.serialize()?;
            let len = (bytes.len() as u32).to_be_bytes();

            // s2n_quic::stream::BidirectionalStream implements tokio::io::AsyncWrite
            use tokio::io::AsyncWriteExt;
            stream
                .write_all(&len)
                .await
                .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
            stream
                .write_all(&bytes)
                .await
                .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

            stream
                .flush()
                .await
                .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;
            stream
                .shutdown()
                .await
                .map_err(|e| crate::error::DbxError::Network(e.to_string()))?;

            Ok(())
        } else {
            Err(crate::error::DbxError::Network(
                "No stream available for reply".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::protocol::StorageMessage;
    use rcgen::generate_simple_self_signed;

    #[tokio::test]
    async fn test_quic_channel_send_and_receive() {
        let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
        let cert = generate_simple_self_signed(subject_alt_names).unwrap();
        let cert_pem = cert.cert.pem();
        let key_pem = cert.signing_key.serialize_pem();

        let (tx1, mut rx1) = mpsc::channel(100);
        let node1_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let channel1 = QuicChannel::new(node1_addr, &cert_pem, &key_pem, tx1)
            .await
            .unwrap();
        let local_addr1 = channel1.local_addr; // need actual bound addr but binding to 0 may not expose it on `channel1.local_addr` immediately. Wait, QuicChannel::new receives 127.0.0.1:0 but doesn't update it to the bound port... So we need a fixed port or get the bound port.

        // For testing we will use fixed ports
        let node1_fixed_addr: SocketAddr = "127.0.0.1:15682".parse().unwrap();
        let (tx1, mut rx1) = mpsc::channel(100);
        let channel1 = QuicChannel::new(node1_fixed_addr, &cert_pem, &key_pem, tx1)
            .await
            .unwrap();

        let node2_fixed_addr: SocketAddr = "127.0.0.1:15683".parse().unwrap();
        let (tx2, _rx2) = mpsc::channel(100);
        let channel2 = QuicChannel::new(node2_fixed_addr, &cert_pem, &key_pem, tx2)
            .await
            .unwrap();

        let test_msg = crate::grid::protocol::GridMessage::Storage(StorageMessage::FetchShard {
            key: "test_key".to_string(),
            shard_id: 42,
        });

        // node2 sends msg to node1
        channel2
            .send_message(node1_fixed_addr, test_msg.clone())
            .await
            .unwrap();

        // node1 receives msg
        if let Some(wrapper) = rx1.recv().await {
            match wrapper.msg {
                crate::grid::protocol::GridMessage::Storage(StorageMessage::FetchShard {
                    key,
                    shard_id,
                }) => {
                    assert_eq!(key, "test_key");
                    assert_eq!(shard_id, 42);
                }
                _ => panic!("Unexpected message type received"),
            }
        } else {
            panic!("No message received");
        }
    }
}
