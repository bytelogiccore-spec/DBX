use arrow::datatypes::Schema;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum StorageTier {
    #[default]
    MemoryWOS,
    DiskROS,
    ColdEC,
}

#[derive(Debug, Clone)]
pub struct PartitionMeta {
    pub min_key: Vec<u8>,
    pub max_key: Vec<u8>,
    pub file_path: String,
    pub row_count: usize,
    pub tier: StorageTier,
    /// 해당 파티션을 물리적으로 보유한 클러스터 워커 주소 (e.g. "192.168.0.2:15690")
    pub node_addr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TableMeta {
    pub schema: Arc<Schema>,
    pub partitions: Arc<DashMap<String, PartitionMeta>>,
}

impl TableMeta {
    pub fn new(schema: Arc<Schema>) -> Self {
        Self {
            schema,
            partitions: Arc::new(DashMap::new()),
        }
    }
}

/// 글로벌 파티션 및 스토리지 계층 상태 캐시 시스템.
/// LifecycleWorker 가 동기화하며 Optimizer와 분산 스케줄러가 구독합니다.
#[derive(Clone)]
pub struct MetadataRegistry {
    pub tables: Arc<DashMap<String, Arc<TableMeta>>>,
}

impl Default for MetadataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataRegistry {
    pub fn new() -> Self {
        Self {
            tables: Arc::new(DashMap::new()),
        }
    }

    pub fn get_table(&self, table_name: &str) -> Option<Arc<TableMeta>> {
        self.tables.get(table_name).map(|r| Arc::clone(r.value()))
    }

    pub fn register_table(&self, table_name: String, schema: Arc<Schema>) {
        self.tables
            .insert(table_name, Arc::new(TableMeta::new(schema)));
    }

    /// 특정 파티션의 위치와 계층 상태를 업데이트합니다. LifecycleWorker 컴팩션 이후 호출됩니다.
    pub fn update_partition(&self, table_name: &str, partition_id: String, meta: PartitionMeta) {
        if let Some(table) = self.get_table(table_name) {
            table.partitions.insert(partition_id, meta);
        }
    }

    /// 해당 파티션을 스캔 범위에서 완전 제거합니다. (삭제나 Time Travel, 만료 등으로 파기될 때)
    pub fn drop_partition(&self, table_name: &str, partition_id: &str) {
        if let Some(table) = self.get_table(table_name) {
            table.partitions.remove(partition_id);
        }
    }
}
