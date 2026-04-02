use dbx_core::error::DbxResult;
use dbx_core::grid::manager::GridManager;
use dbx_core::grid::quic::QuicChannel;
use dbx_core::sharding::router::ShardRouter;
use dbx_core::storage::erasure_coding::distributed_store::DistributedErasureCodingStore;
use dbx_core::storage::erasure_coding::store::ErasureCodingStore;
use rcgen::generate_simple_self_signed;
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_grid_ec_network_integration() -> DbxResult<()> {
    // 0. 로거 초기화
    let _ = tracing_subscriber::fmt()
        .with_env_filter("dbx_core=debug,grid_ec_integration_test=debug")
        .try_init();

    // 1. 공통 인증서 생성 (테스트용)
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // 2. 노드 1 설정 (Port 15690)
    let dir1 = tempdir().unwrap();
    let addr1: SocketAddr = "127.0.0.1:15690".parse().unwrap();
    let (tx1, rx1) = mpsc::channel(100);
    let quic1 = Arc::new(QuicChannel::new(addr1, &cert_pem, &key_pem, tx1).await?);

    let local_store1 = ErasureCodingStore::new(dir1.path(), 2, 1); // K=2, M=1
    let router = Arc::new(ShardRouter::new_with_addresses(vec![
        "127.0.0.1:15690".to_string(),
        "127.0.0.1:15691".to_string(),
    ]));

    let dist_store1 = Arc::new(DistributedErasureCodingStore::new(
        local_store1,
        Arc::clone(&router),
        Some(Arc::clone(&quic1)),
    ));
    let manager1 = GridManager::new(Arc::clone(&quic1), Arc::clone(&dist_store1), rx1);
    tokio::spawn(manager1.run());

    // 3. 노드 2 설정 (Port 15691)
    let dir2 = tempdir().unwrap();
    let addr2: SocketAddr = "127.0.0.1:15691".parse().unwrap();
    let (tx2, rx2) = mpsc::channel(100);
    let quic2 = Arc::new(QuicChannel::new(addr2, &cert_pem, &key_pem, tx2).await?);

    let local_store2 = ErasureCodingStore::new(dir2.path(), 2, 1);
    let dist_store2 = Arc::new(DistributedErasureCodingStore::new(
        local_store2,
        Arc::clone(&router),
        Some(Arc::clone(&quic2)),
    ));
    let manager2 = GridManager::new(Arc::clone(&quic2), Arc::clone(&dist_store2), rx2);
    tokio::spawn(manager2.run());

    // 4. 데이터 저장 테스트 (노드 1에서 수행)
    let key = "network_ec_test_key";
    let data = b"Hello from Grid Node 1 - Distributed EC verified!";

    println!("Node 1: Storing data with EC...");
    dist_store1.store(key, data).await?;

    // 네트워크 전송 시간 대기
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // 5. 데이터 복구 테스트 (노드 2에서 수행)
    // 노드 2는 로컬에 데이터가 없으므로 노드 1(또는 자기 샤드)로부터 복구해야 함
    println!("Node 2: Retrieving data from Grid...");
    let recovered = dist_store2
        .retrieve(key)
        .await?
        .expect("Data should be recovered");

    assert_eq!(recovered, data);
    println!("Success: Data recovered correctly via network shards!");

    Ok(())
}
