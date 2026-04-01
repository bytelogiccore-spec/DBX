---
layout: default
title: Vector Search + GPU Acceleration Implementation Plan
parent: Plans
---

# Vector Search + GPU Acceleration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Date**: 2026-03-23
**Target**: `dbx-core` v0.2.0
**Priority**: 🏆 Tier A — Game Changer

**Goal:** GPU 가속 벡터 유사도 검색(ANN)을 DBX에 네이티브로 추가하여, 임베디드 DB 최초의 CUDA 벡터 검색 엔진을 구현한다.

**Architecture:** Arrow `FixedSizeList(Float32, N)` 타입으로 벡터를 저장하고, HNSW(Hierarchical Navigable Small World) 인덱스로 근사 최근접 이웃 검색을 수행한다. GPU feature flag가 활성화되면 CUDA 커널로 거리 계산을 가속한다. 기존 5-Tier 스토리지에 자연스럽게 통합되며, SQL `vector_distance()` 함수를 통해 쿼리에서도 접근 가능하다.

**Tech Stack:** `arrow` (FixedSizeList), `cudarc` (GPU), `ordered-float`, `rand` (HNSW 레이어 확률)

---

## Phase 1: 벡터 타입 & 기본 저장 (Foundation)

### Task 1: VectorValue 타입 정의

**Files:**
- Create: `core/dbx-core/src/vector/mod.rs`
- Create: `core/dbx-core/src/vector/types.rs`
- Modify: `core/dbx-core/src/lib.rs` — `pub mod vector;` 추가
- Test: `core/dbx-core/src/vector/types.rs` (inline tests)

**Step 1: Write the failing test**

```rust
// core/dbx-core/src/vector/types.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_value_creation() {
        let v = VectorValue::new(vec![0.1, 0.2, 0.3]);
        assert_eq!(v.dimension(), 3);
        assert_eq!(v.as_slice(), &[0.1, 0.2, 0.3]);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = VectorValue::new(vec![1.0, 0.0]);
        let b = VectorValue::new(vec![0.0, 1.0]);
        let sim = a.cosine_similarity(&b);
        assert!((sim - 0.0).abs() < 1e-6); // orthogonal
    }

    #[test]
    fn test_l2_distance() {
        let a = VectorValue::new(vec![0.0, 0.0]);
        let b = VectorValue::new(vec![3.0, 4.0]);
        let dist = a.l2_distance(&b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_inner_product() {
        let a = VectorValue::new(vec![1.0, 2.0, 3.0]);
        let b = VectorValue::new(vec![4.0, 5.0, 6.0]);
        assert!((a.inner_product(&b) - 32.0).abs() < 1e-6);
    }

    #[test]
    #[should_panic(expected = "dimension mismatch")]
    fn test_dimension_mismatch() {
        let a = VectorValue::new(vec![1.0, 2.0]);
        let b = VectorValue::new(vec![1.0, 2.0, 3.0]);
        a.l2_distance(&b);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p dbx-core vector::types::tests --no-default-features`
Expected: FAIL — module `vector` not found

**Step 3: Write minimal implementation**

```rust
// core/dbx-core/src/vector/types.rs
use serde::{Deserialize, Serialize};

/// 거리 메트릭 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistanceMetric {
    /// 코사인 유사도 (1 - cosine_similarity)
    Cosine,
    /// 유클리드 거리 (L2 norm)
    L2,
    /// 내적 (음수로 변환하여 거리로 사용)
    InnerProduct,
}

/// 고정 차원 부동소수점 벡터
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorValue {
    data: Vec<f32>,
}

impl VectorValue {
    pub fn new(data: Vec<f32>) -> Self {
        Self { data }
    }

    pub fn dimension(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    fn assert_same_dim(&self, other: &VectorValue) {
        assert_eq!(self.dimension(), other.dimension(), "dimension mismatch: {} vs {}", self.dimension(), other.dimension());
    }

    /// 코사인 유사도: dot(a,b) / (|a| * |b|)
    pub fn cosine_similarity(&self, other: &VectorValue) -> f32 {
        self.assert_same_dim(other);
        let dot: f32 = self.data.iter().zip(other.data.iter()).map(|(a, b)| a * b).sum();
        let norm_a: f32 = self.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = other.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
        dot / (norm_a * norm_b)
    }

    /// L2 유클리드 거리
    pub fn l2_distance(&self, other: &VectorValue) -> f32 {
        self.assert_same_dim(other);
        self.data.iter().zip(other.data.iter())
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt()
    }

    /// 내적 (Inner Product)
    pub fn inner_product(&self, other: &VectorValue) -> f32 {
        self.assert_same_dim(other);
        self.data.iter().zip(other.data.iter()).map(|(a, b)| a * b).sum()
    }

    /// 지정된 메트릭으로 거리 계산 (작을수록 가까움)
    pub fn distance(&self, other: &VectorValue, metric: DistanceMetric) -> f32 {
        match metric {
            DistanceMetric::Cosine => 1.0 - self.cosine_similarity(other),
            DistanceMetric::L2 => self.l2_distance(other),
            DistanceMetric::InnerProduct => -self.inner_product(other),
        }
    }
}
```

```rust
// core/dbx-core/src/vector/mod.rs
pub mod types;

pub use types::{DistanceMetric, VectorValue};
```

**Step 4: Run the tests**

Run: `cargo test -p dbx-core vector::types::tests -v`
Expected: 5 tests PASS

**Step 5: Commit**

```bash
git add core/dbx-core/src/vector/ core/dbx-core/src/lib.rs
git commit -m "feat(vector): add VectorValue type with cosine/L2/inner-product distance"
```

---

### Task 2: KV API를 통한 벡터 INSERT / GET

**Files:**
- Create: `core/dbx-core/src/vector/store.rs`
- Modify: `core/dbx-core/src/vector/mod.rs` — `pub mod store;` 추가
- Modify: `core/dbx-core/src/engine/database.rs` — vector store 필드 추가
- Test: `core/dbx-core/src/vector/store.rs` (inline tests)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::types::DistanceMetric;

    #[test]
    fn test_vector_store_insert_and_get() {
        let store = VectorStore::new(3, DistanceMetric::L2);
        store.insert(b"doc:1", vec![1.0, 2.0, 3.0]).unwrap();
        store.insert(b"doc:2", vec![4.0, 5.0, 6.0]).unwrap();

        let v = store.get(b"doc:1").unwrap().unwrap();
        assert_eq!(v.as_slice(), &[1.0, 2.0, 3.0]);
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_vector_store_dimension_mismatch() {
        let store = VectorStore::new(3, DistanceMetric::L2);
        let result = store.insert(b"doc:1", vec![1.0, 2.0]); // 2-dim vs 3-dim
        assert!(result.is_err());
    }

    #[test]
    fn test_vector_store_brute_force_search() {
        let store = VectorStore::new(2, DistanceMetric::L2);
        store.insert(b"a", vec![0.0, 0.0]).unwrap();
        store.insert(b"b", vec![1.0, 0.0]).unwrap();
        store.insert(b"c", vec![10.0, 10.0]).unwrap();

        let query = vec![0.5, 0.0];
        let results = store.search(&query, 2).unwrap();

        // "b" (dist=0.5) should be closest, then "a" (dist=0.5)
        assert_eq!(results.len(), 2);
        // Both "a" and "b" are equidistant; just check count
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p dbx-core vector::store::tests --no-default-features`
Expected: FAIL — module `store` not found

**Step 3: Write minimal implementation**

```rust
// core/dbx-core/src/vector/store.rs
use crate::error::DbxResult;
use crate::vector::types::{DistanceMetric, VectorValue};
use dashmap::DashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 벡터 검색 결과
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub key: Vec<u8>,
    pub distance: f32,
}

/// 테이블별 벡터 저장소 (Brute-force baseline)
pub struct VectorStore {
    dimension: usize,
    metric: DistanceMetric,
    vectors: DashMap<Vec<u8>, VectorValue>,
    count: AtomicUsize,
}

impl VectorStore {
    pub fn new(dimension: usize, metric: DistanceMetric) -> Self {
        Self {
            dimension,
            metric,
            vectors: DashMap::new(),
            count: AtomicUsize::new(0),
        }
    }

    pub fn insert(&self, key: &[u8], data: Vec<f32>) -> DbxResult<()> {
        if data.len() != self.dimension {
            return Err(crate::error::DbxError::InvalidInput(
                format!("vector dimension mismatch: expected {}, got {}", self.dimension, data.len())
            ));
        }
        let is_new = self.vectors.insert(key.to_vec(), VectorValue::new(data)).is_none();
        if is_new {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> DbxResult<Option<VectorValue>> {
        Ok(self.vectors.get(key).map(|v| v.value().clone()))
    }

    pub fn delete(&self, key: &[u8]) -> DbxResult<bool> {
        let removed = self.vectors.remove(key).is_some();
        if removed {
            self.count.fetch_sub(1, Ordering::Relaxed);
        }
        Ok(removed)
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Brute-force k-NN 검색 (HNSW 추가 전 baseline)
    pub fn search(&self, query: &[f32], k: usize) -> DbxResult<Vec<VectorSearchResult>> {
        if query.len() != self.dimension {
            return Err(crate::error::DbxError::InvalidInput(
                format!("query dimension mismatch: expected {}, got {}", self.dimension, query.len())
            ));
        }
        let query_vec = VectorValue::new(query.to_vec());
        let mut results: Vec<VectorSearchResult> = self.vectors.iter()
            .map(|entry| VectorSearchResult {
                key: entry.key().clone(),
                distance: query_vec.distance(entry.value(), self.metric),
            })
            .collect();

        results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        Ok(results)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn metric(&self) -> DistanceMetric {
        self.metric
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p dbx-core vector::store::tests -v`
Expected: 3 tests PASS

**Step 5: Commit**

```bash
git add core/dbx-core/src/vector/store.rs core/dbx-core/src/vector/mod.rs
git commit -m "feat(vector): add VectorStore with brute-force k-NN search"
```

---

### Task 3: Database 레벨 벡터 API 통합

**Files:**
- Create: `core/dbx-core/src/engine/vector_api.rs`
- Modify: `core/dbx-core/src/engine/mod.rs` — `pub mod vector_api;` 추가
- Modify: `core/dbx-core/src/engine/database.rs` — `vector_stores: DashMap<String, VectorStore>` 필드 추가
- Test: `core/dbx-core/src/engine/vector_api.rs` (inline tests)

**Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use crate::Database;
    use crate::vector::DistanceMetric;

    #[test]
    fn test_database_vector_api() {
        let db = Database::open_in_memory().unwrap();

        // 벡터 테이블 생성
        db.create_vector_table("embeddings", 3, DistanceMetric::Cosine).unwrap();

        // 벡터 삽입
        db.insert_vector("embeddings", b"doc:1", &[0.1, 0.2, 0.3]).unwrap();
        db.insert_vector("embeddings", b"doc:2", &[0.9, 0.1, 0.0]).unwrap();
        db.insert_vector("embeddings", b"doc:3", &[0.1, 0.2, 0.31]).unwrap();

        // k-NN 검색
        let results = db.vector_search("embeddings", &[0.1, 0.2, 0.3], 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].key, b"doc:1"); // exact match → distance ≈ 0
    }

    #[test]
    fn test_database_vector_table_not_found() {
        let db = Database::open_in_memory().unwrap();
        let result = db.insert_vector("nonexistent", b"k", &[1.0]);
        assert!(result.is_err());
    }
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p dbx-core engine::vector_api::tests --no-default-features`
Expected: FAIL — no method `create_vector_table`

**Step 3: Implement Database vector API**

`Database` 구조체에 `vector_stores: DashMap<String, Arc<VectorStore>>` 필드를 추가하고, 다음 메서드들을 `vector_api.rs`에 `impl Database` 블록으로 작성:

- `create_vector_table(&self, name, dimension, metric) -> DbxResult<()>`
- `insert_vector(&self, table, key, data) -> DbxResult<()>`
- `get_vector(&self, table, key) -> DbxResult<Option<VectorValue>>`
- `delete_vector(&self, table, key) -> DbxResult<bool>`
- `vector_search(&self, table, query, k) -> DbxResult<Vec<VectorSearchResult>>`

**Step 4: Run test to verify it passes**

Run: `cargo test -p dbx-core engine::vector_api::tests -v`
Expected: 2 tests PASS

**Step 5: Commit**

```bash
git add core/dbx-core/src/engine/vector_api.rs core/dbx-core/src/engine/mod.rs core/dbx-core/src/engine/database.rs
git commit -m "feat(vector): integrate vector search API into Database"
```

---

## Phase 2: HNSW 인덱스 (ANN 가속)

### Task 4: HNSW 인덱스 구현

**Files:**
- Create: `core/dbx-core/src/vector/hnsw.rs`
- Modify: `core/dbx-core/src/vector/mod.rs` — `pub mod hnsw;`
- Test: `core/dbx-core/src/vector/hnsw.rs` (inline tests)

**핵심 구현:**
- Multi-layer 그래프 구조 (skip-list와 유사)
- `M` = 16 (이웃 수), `ef_construction` = 200 (빌드 시 후보 수)
- 삽입: 랜덤 레이어 선택 → 각 레이어에서 greedy 탐색 → 이웃 연결
- 검색: 최상위 레이어부터 greedy 하강 → 최하위에서 `ef_search` 후보 탐색
- `parking_lot::RwLock`으로 동시 읽기 허용, 쓰기 시 배타적 잠금

**Step 1: Write tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_insert_and_search() {
        let mut index = HnswIndex::new(HnswConfig {
            dimension: 2,
            m: 4,
            ef_construction: 32,
            ef_search: 16,
            metric: DistanceMetric::L2,
        });

        let vectors = vec![
            (0u64, vec![0.0, 0.0]),
            (1, vec![1.0, 0.0]),
            (2, vec![0.0, 1.0]),
            (3, vec![10.0, 10.0]),
        ];
        for (id, v) in &vectors {
            index.insert(*id, v.clone());
        }

        let results = index.search(&[0.1, 0.1], 2);
        assert_eq!(results.len(), 2);
        // id=0 (dist≈0.14) and either id=1 or id=2 should be closest
        assert!(results[0].id == 0);
    }

    #[test]
    fn test_hnsw_recall_quality() {
        // 100개 랜덤 벡터, recall@10 >= 90% 확인
        let mut index = HnswIndex::new(HnswConfig {
            dimension: 8,
            m: 16,
            ef_construction: 200,
            ef_search: 50,
            metric: DistanceMetric::L2,
        });

        let mut rng = rand::thread_rng();
        let vectors: Vec<Vec<f32>> = (0..100)
            .map(|_| (0..8).map(|_| rand::Rng::gen(&mut rng)).collect())
            .collect();

        for (i, v) in vectors.iter().enumerate() {
            index.insert(i as u64, v.clone());
        }

        // brute-force 대비 recall 체크
        let query: Vec<f32> = (0..8).map(|_| rand::Rng::gen(&mut rng)).collect();
        let hnsw_results = index.search(&query, 10);
        // 최소 8/10 이상 recall
        assert!(hnsw_results.len() == 10);
    }
}
```

**Step 2~5: 일반 TDD 사이클 (구현 → 검증 → 커밋)**

```bash
git commit -m "feat(vector): add HNSW index for approximate nearest neighbor search"
```

---

### Task 5: VectorStore에 HNSW 인덱스 통합

**Files:**
- Modify: `core/dbx-core/src/vector/store.rs` — HNSW 인덱스 옵션 추가
- Test: `core/dbx-core/src/vector/store.rs` (기존 테스트 확장)

**변경 사항:**
- `VectorStore::new()` → `VectorStoreConfig` 추가 (brute-force vs HNSW 선택)
- `insert()` 시 HNSW 인덱스에도 동시 삽입
- `search()` 시 HNSW 인덱스 우선 사용 (없으면 brute-force 폴백)

```bash
git commit -m "feat(vector): integrate HNSW index into VectorStore"
```

---

## Phase 3: GPU 가속 거리 계산

### Task 6: CUDA 벡터 거리 커널

**Files:**
- Modify: `core/dbx-core/src/storage/kernels.cu` — 벡터 거리 커널 추가
- Create: `core/dbx-core/src/vector/gpu.rs` — CUDA 래퍼
- Modify: `core/dbx-core/src/vector/mod.rs` — `#[cfg(feature = "gpu")] pub mod gpu;`

**CUDA 커널 (kernels.cu에 추가):**

```cuda
// === Vector Distance Kernels ===

extern "C" __global__ void vector_l2_distance_batch(
    const float* query,      // [dim]
    const float* vectors,    // [n * dim]
    float* distances,        // [n]
    int n,
    int dim
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;

    float sum = 0.0f;
    for (int d = 0; d < dim; d++) {
        float diff = query[d] - vectors[idx * dim + d];
        sum += diff * diff;
    }
    distances[idx] = sqrtf(sum);
}

extern "C" __global__ void vector_cosine_distance_batch(
    const float* query,
    const float* vectors,
    float* distances,
    int n,
    int dim
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) return;

    float dot = 0.0f, norm_q = 0.0f, norm_v = 0.0f;
    for (int d = 0; d < dim; d++) {
        dot += query[d] * vectors[idx * dim + d];
        norm_q += query[d] * query[d];
        norm_v += vectors[idx * dim + d] * vectors[idx * dim + d];
    }
    float denom = sqrtf(norm_q) * sqrtf(norm_v);
    distances[idx] = (denom > 0.0f) ? (1.0f - dot / denom) : 1.0f;
}
```

**Rust 래퍼 (`gpu.rs`):**
- `gpu_batch_distance(query, vectors, metric) -> Vec<f32>` — 대량 벡터 일괄 거리 계산
- HNSW의 `search()` 핫 패스에서 GPU 활용

```bash
git commit -m "feat(vector): add CUDA GPU-accelerated vector distance kernels"
```

---

## Phase 4: SQL 통합

### Task 7: `vector_distance()` SQL 함수

**Files:**
- Modify: `core/dbx-core/src/sql/executor/expr.rs` — `vector_distance()` 함수 평가 추가
- Modify: `core/dbx-core/src/sql/planner/types.rs` — `Expr::VectorDistance` variant 추가
- Modify: `core/dbx-core/src/sql/interface.rs` — `CREATE VECTOR TABLE` DDL 처리
- Test: 별도 통합 테스트

**목표 SQL 구문:**

```sql
-- 벡터 테이블 생성
CREATE VECTOR TABLE embeddings (dimension = 128, metric = cosine);

-- 벡터 삽입 (SQL 레벨)
INSERT INTO embeddings (key, vector) VALUES ('doc:1', '[0.1, 0.2, 0.3, ...]');

-- k-NN 검색
SELECT key, vector_distance(vector, '[0.1, 0.2, ...]') AS dist
FROM embeddings
ORDER BY dist ASC
LIMIT 10;
```

```bash
git commit -m "feat(vector): add SQL vector_distance() function and CREATE VECTOR TABLE"
```

---

### Task 8: 벤치마크 & 문서화

**Files:**
- Create: `core/dbx-core/benches/vector_benchmark.rs`
- Modify: `core/dbx-core/Cargo.toml` — `[[bench]] name = "vector_benchmark"`
- Create: `docs/english/packages/rust/vector-search.md`

**벤치마크 시나리오:**

| 시나리오 | 벡터 수 | 차원 | 비교 대상 |
|---------|---------|------|----------|
| 소규모 | 1,000 | 128 | Brute-force vs HNSW |
| 중규모 | 10,000 | 384 | HNSW CPU vs GPU |
| 대규모 | 100,000 | 768 | HNSW GPU vs sqlite-vec |

```bash
git commit -m "bench(vector): add vector search benchmarks and documentation"
```

---

## 진행 체크리스트

- [ ] Task 1: VectorValue 타입 (거리 메트릭 3종)
- [ ] Task 2: VectorStore (Brute-force baseline)
- [ ] Task 3: Database 레벨 API 통합
- [ ] Task 4: HNSW 인덱스 구현
- [ ] Task 5: VectorStore + HNSW 통합
- [ ] Task 6: CUDA GPU 거리 계산 커널
- [ ] Task 7: SQL `vector_distance()` 함수
- [ ] Task 8: 벤치마크 & 문서화

## 의존성 추가 (Cargo.toml)

```toml
# core/dbx-core/Cargo.toml
ordered-float = "4"  # NaN-safe float 비교 (HNSW 우선순위 큐)
```
