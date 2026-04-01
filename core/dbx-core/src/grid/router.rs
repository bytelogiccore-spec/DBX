//! Grid Router
//!
//! 네트워크 수신부에서 `GridMessage`를 받아 `Replication`, `Lock`으로 변환해주고,
//! 송신 시 `ReplicationMessage`를 `GridMessage`로 래핑하여 전송합니다.

use crate::grid::protocol::{GridMessage, LockMessage};
use crate::replication::protocol::ReplicationMessage;
use tokio::sync::broadcast;

pub struct GridRouter {
    /// 외부 네트워크(Transport)와 연결된 메인 그리드 채널
    pub grid_tx: broadcast::Sender<GridMessage>,

    // Inbound (Network -> Node)
    pub repl_in_tx: broadcast::Sender<ReplicationMessage>,
    pub lock_in_tx: broadcast::Sender<LockMessage>,

    // Outbound (Node -> Network)
    pub repl_out_tx: broadcast::Sender<ReplicationMessage>,
    pub lock_out_tx: broadcast::Sender<LockMessage>,
    pub storage_out_tx: broadcast::Sender<crate::grid::protocol::StorageMessage>,
}

impl GridRouter {
    pub fn new(grid_tx: broadcast::Sender<GridMessage>) -> Self {
        let (repl_in_tx, _) = broadcast::channel(1024);
        let (lock_in_tx, _) = broadcast::channel(1024);
        let (repl_out_tx, _) = broadcast::channel(1024);
        let (lock_out_tx, _) = broadcast::channel(1024);
        let (storage_out_tx, _) = broadcast::channel(1024);

        Self {
            grid_tx,
            repl_in_tx,
            lock_in_tx,
            repl_out_tx,
            lock_out_tx,
            storage_out_tx,
        }
    }

    /// 라우터를 백그라운드 태스크로 시작합니다.
    pub fn start(&self) {
        // 1. Grid -> Subsystems (Inbound)
        let mut grid_rx = self.grid_tx.subscribe();
        let repl_in_tx = self.repl_in_tx.clone();
        let lock_in_tx = self.lock_in_tx.clone();

        tokio::spawn(async move {
            while let Ok(msg) = grid_rx.recv().await {
                match msg {
                    GridMessage::Replication(r) => {
                        let _ = repl_in_tx.send(r);
                    }
                    GridMessage::Lock(l) => {
                        let _ = lock_in_tx.send(l);
                    }
                    GridMessage::Storage(_) => {
                        // TODO: Implement Storage handling
                    }
                    GridMessage::Query(_) => {
                        // Query 메시지는 GridManager가 직접 핸들링하므로 라우터 브로드캐스트에서는 무시.
                    }
                }
            }
        });

        // 2. Subsystems -> Grid (Outbound Replication)
        let mut repl_out_rx = self.repl_out_tx.subscribe();
        let grid_tx_repl = self.grid_tx.clone();
        tokio::spawn(async move {
            while let Ok(msg) = repl_out_rx.recv().await {
                let _ = grid_tx_repl.send(GridMessage::Replication(msg));
            }
        });

        // 3. Subsystems -> Grid (Outbound Lock)
        let mut lock_out_rx = self.lock_out_tx.subscribe();
        let grid_tx_lock = self.grid_tx.clone();
        tokio::spawn(async move {
            while let Ok(msg) = lock_out_rx.recv().await {
                let _ = grid_tx_lock.send(GridMessage::Lock(msg));
            }
        });

        // 4. Subsystems -> Grid (Outbound Storage)
        let mut storage_out_rx = self.storage_out_tx.subscribe();
        let grid_tx_storage = self.grid_tx.clone();
        tokio::spawn(async move {
            while let Ok(msg) = storage_out_rx.recv().await {
                let _ = grid_tx_storage.send(GridMessage::Storage(msg));
            }
        });
    }
}
