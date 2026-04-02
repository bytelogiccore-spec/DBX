use dbx_core::error::DbxError;
use dbx_core::grid::manager::GridManager;
use dbx_core::grid::quic::QuicChannel;
use dbx_core::sharding::router::ShardRouter;
use dbx_core::sql::executor::distributed_executor::DistributedExecutor;
use dbx_core::sql::executor::local_executor::LocalExecutor;
use dbx_core::sql::planner::types::PhysicalPlan;
use dbx_core::storage::erasure_coding::distributed_store::DistributedErasureCodingStore;
use dbx_core::storage::erasure_coding::store::ErasureCodingStore;
use rcgen::generate_simple_self_signed;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tempfile::tempdir;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::test]
async fn test_fault_tolerance_timeout_retry() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("dbx_core=info,fault_tolerance_test=debug")
        .try_init();

    // 1. 공통 인증서 생성
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // 2. 코디네이터 노드 설정 (Port 16690)
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:16690".parse().unwrap();
    let (tx, rx) = mpsc::channel(100);
    let quic = Arc::new(
        QuicChannel::new(addr, &cert_pem, &key_pem, tx)
            .await
            .unwrap(),
    );

    let local_store = ErasureCodingStore::new(dir.path(), 2, 1);
    let router = Arc::new(ShardRouter::new_with_addresses(vec![
        "127.0.0.1:16690".to_string(),
    ]));
    let dist_store = Arc::new(DistributedErasureCodingStore::new(
        local_store,
        Arc::clone(&router),
        Some(Arc::clone(&quic)),
    ));
    let manager = Arc::new(GridManager::new(Arc::clone(&quic), dist_store, rx));

    // 환경 변수 설정으로 빠른 타임아웃(1초)
    unsafe {
        std::env::set_var("DBX_WORKER_TIMEOUT_SECS", "1");
    }

    let ts = Arc::new(RwLock::new(HashMap::new()));
    let ss = Arc::new(RwLock::new(HashMap::new()));
    let local_executor = Arc::new(LocalExecutor::new(ts, ss));

    let dummy_worker: SocketAddr = "127.0.0.1:29999".parse().unwrap();
    let registry = Arc::new(dbx_core::storage::metadata::MetadataRegistry::new());
    let dist_exec = DistributedExecutor::new(
        Arc::clone(&quic),
        Arc::clone(&manager),
        local_executor,
        vec![dummy_worker],
        registry,
    );

    // 분산 처리를 유발하기 위해 HashJoin 연산 계획 생성
    let plan = PhysicalPlan::HashJoin {
        left: Box::new(PhysicalPlan::TableScan {
            table: "left_table".to_string(),
            projection: vec![],
            filter: None,
            ros_files: vec![],
        }),
        right: Box::new(PhysicalPlan::TableScan {
            table: "right_table".to_string(),
            projection: vec![],
            filter: None,
            ros_files: vec![],
        }),
        on: vec![(0, 0)],
        join_type: dbx_core::sql::planner::types::JoinType::Inner,
    };

    info!("Starting deliberate fault injection test...");
    let start = std::time::Instant::now();
    let result = dist_exec.execute(plan).await;
    let elapsed = start.elapsed();

    // 에러 발생 확인
    assert!(result.is_err(), "Expected Network Error due to timeout");

    match result {
        Err(DbxError::Network(msg)) => {
            info!("Received expected network timeout error: {}", msg);
            assert!(
                msg.contains("timed out after 3 retries"),
                "Not the expected retry count message"
            );
        }
        _ => panic!("Expected Error::Network, got {:?}", result),
    }

    // timeout 1초 x (초회 1 + 재시도 3회) = 약 4초 대기!
    info!("Total Elapsed Time: {:?}", elapsed);
    assert!(
        elapsed.as_secs() >= 3,
        "Retry loop did not wait enough iterations"
    );
}
