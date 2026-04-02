use dbx_core::error::DbxResult;
use dbx_core::grid::manager::GridManager;
use dbx_core::grid::protocol::{GridMessage, QueryMessage};
use dbx_core::grid::quic::QuicChannel;
use dbx_core::sharding::router::ShardRouter;
use dbx_core::sql::executor::operators::{GridExchangeOperator, PhysicalOperator};
use dbx_core::storage::erasure_coding::distributed_store::DistributedErasureCodingStore;
use dbx_core::storage::erasure_coding::store::ErasureCodingStore;

use arrow::array::Int32Array;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::mpsc;

fn make_dummy_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![Field::new("val", DataType::Int32, false)]));
    let array = Int32Array::from(vec![1, 2, 3, 4, 5]);
    RecordBatch::try_new(schema, vec![Arc::new(array)]).unwrap()
}

fn batch_to_ipc_bytes(batch: &RecordBatch) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buf, &batch.schema()).unwrap();
        writer.write(batch).unwrap();
        writer.finish().unwrap();
    }
    buf
}

#[tokio::test]
async fn test_grid_exchange_streaming_backpressure() -> DbxResult<()> {
    // 디버그 로깅 활성화
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,dbx_core=debug")
        .try_init();

    // 임시 디렉토리 및 의존성
    let dir1 = tempdir()?;
    let dir2 = tempdir()?;

    // 포트 할당
    let addr_coord = "127.0.0.1:15890";
    let addr_worker = "127.0.0.1:15891";
    let coord_sock = SocketAddr::from_str(addr_coord).unwrap();
    let worker_sock = SocketAddr::from_str(addr_worker).unwrap();

    // 1. SSL 인증서 생성 (rcgen)
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert = rcgen::generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // 2. Quic 채널 생성
    let (tx_coord, rx_coord) = mpsc::channel(100);
    // coord
    let quic_coord = QuicChannel::new(coord_sock, &cert_pem, &key_pem, tx_coord).await?;
    let quic_coord = Arc::new(quic_coord);
    // QuicChannel::new 내부에서 이미 수신 루프가 spawn 됨.

    let (tx_worker, rx_worker) = mpsc::channel(100);
    // worker
    let quic_worker = QuicChannel::new(worker_sock, &cert_pem, &key_pem, tx_worker).await?;
    let quic_worker = Arc::new(quic_worker);

    // 더미 의존성
    let local_store1 = ErasureCodingStore::new(dir1.path(), 4, 2);
    let local_store2 = ErasureCodingStore::new(dir2.path(), 4, 2);
    let router = Arc::new(ShardRouter::new_with_addresses(vec![
        addr_coord.to_string(),
        addr_worker.to_string(),
    ]));

    let ec_store1 = Arc::new(DistributedErasureCodingStore::new(
        local_store1,
        Arc::clone(&router),
        Some(Arc::clone(&quic_coord)),
    ));
    let ec_store2 = Arc::new(DistributedErasureCodingStore::new(
        local_store2,
        Arc::clone(&router),
        Some(Arc::clone(&quic_worker)),
    ));

    // 2. GridManager 생성 및 실행
    let manager_coord = GridManager::new(Arc::clone(&quic_coord), ec_store1, rx_coord);
    let coord_streams = manager_coord.get_query_streams();
    tokio::spawn(manager_coord.run());

    let manager_worker = GridManager::new(Arc::clone(&quic_worker), ec_store2, rx_worker);
    tokio::spawn(manager_worker.run());

    let execution_id = "test_exe_uuid_1234".to_string();

    // 3. 코디네이터 노드: 스트리밍 수신 큐(배압을 위해 Bounded 2 설정!) 등록
    // 채널 사이즈가 2이므로, worker가 엄청 빨리 쏴도 Coordinator가 천천히 꺼내가면 backpressure가 발생함.
    let (sender, receiver) = mpsc::channel(2);
    coord_streams.insert((execution_id.clone(), 0), sender);

    // 4. GridExchangeOperator (코디네이터 측 물리 오퍼레이터) 생성
    let dummy_schema = make_dummy_batch().schema();
    let mut operator = GridExchangeOperator::new(dummy_schema, receiver);

    // 5. 워커 노드 구동 흉내내기: 10개의 RecordBatch 스트리밍 발송!
    let worker_channel = Arc::clone(&quic_worker);
    let worker_exe_id = execution_id.clone();

    let worker_task = tokio::spawn(async move {
        let dummy = make_dummy_batch();
        let bytes = batch_to_ipc_bytes(&dummy);

        for _ in 0..10 {
            let msg = GridMessage::Query(QueryMessage::ExchangeData {
                execution_id: worker_exe_id.clone(),
                node_id: 1,
                is_eof: false,
                batch_data: bytes.clone(),
                exchange_id: 0,
            });
            // QUIC 통신으로 쏘기 (Stream reset 에러는 무시)
            if let Err(e) = worker_channel.send_message(coord_sock, msg).await {
                let err_str = e.to_string();
                if !err_str.contains("Stream had been reset")
                    && !err_str.contains("application::Error")
                {
                    panic!("Unexpected network error: {}", e);
                }
            }

            // 약간의 지연(현실적인 IO 딜레이 흉내)
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }

        // 스트림 전송 완료 (EOF)
        let eof_msg = GridMessage::Query(QueryMessage::ExchangeData {
            execution_id: worker_exe_id,
            node_id: 1,
            is_eof: true,
            batch_data: vec![],
            exchange_id: 0,
        });
        let _ = worker_channel.send_message(coord_sock, eof_msg).await;
    });

    // 6. 코디네이터 오퍼레이터 실행: 루프를 돌며 배치가 정확히 10번 출력되는지 확인.
    // blocking_recv() 호출을 흉내내기 위해 tokio spawn blocking 사용.
    let count = tokio::task::spawn_blocking(move || {
        let mut total_batches = 0;
        loop {
            // next()는 내부적으로 receive.blocking_recv()를 수행!
            match operator.next().unwrap() {
                Some(batch) => {
                    total_batches += 1;
                    assert_eq!(batch.num_rows(), 5);
                    // 연산이 느리다고 가정 (배압 유발!)
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                None => {
                    // EOF 도달
                    break;
                }
            }
        }
        total_batches
    })
    .await
    .unwrap();

    assert_eq!(
        count, 10,
        "Should exactly receive 10 RecordBatches over Grid network"
    );

    worker_task.await.unwrap();

    Ok(())
}
