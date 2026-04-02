// Cleanup performance benchmarks — measures impact of Phase 1 cleanup changes
// Targets: insert, get, scan, range, table_row_count, batch_insert

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;

fn bench_crud_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("crud_cleanup");
    group.sample_size(200);

    // ── INSERT ──────────────────────────────────────────────
    group.bench_function("insert_1k", |b| {
        let db = Database::open_in_memory().unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            for _ in 0..1000 {
                let key = counter.to_le_bytes();
                db.insert(
                    black_box("bench"),
                    black_box(&key),
                    black_box(b"value_data_here"),
                )
                .unwrap();
                counter += 1;
            }
        })
    });

    // ── GET (hit) ──────────────────────────────────────────
    group.bench_function("get_hit_1k", |b| {
        let db = Database::open_in_memory().unwrap();
        // Pre-populate
        for i in 0u64..1000 {
            db.insert("bench", &i.to_le_bytes(), b"value_data_here")
                .unwrap();
        }
        let mut counter = 0u64;
        b.iter(|| {
            for _ in 0..1000 {
                let key = (counter % 1000).to_le_bytes();
                let _ = db.get(black_box("bench"), black_box(&key)).unwrap();
                counter += 1;
            }
        })
    });

    // ── SCAN ───────────────────────────────────────────────
    group.bench_function("scan_1k_rows", |b| {
        let db = Database::open_in_memory().unwrap();
        for i in 0u64..1000 {
            db.insert("bench", &i.to_le_bytes(), b"value_data_here")
                .unwrap();
        }
        b.iter(|| {
            let _ = db.scan(black_box("bench")).unwrap();
        })
    });

    // ── RANGE ──────────────────────────────────────────────
    group.bench_function("range_subset", |b| {
        let db = Database::open_in_memory().unwrap();
        for i in 0u64..1000 {
            db.insert("bench", &i.to_le_bytes(), b"value_data_here")
                .unwrap();
        }
        let start = 100u64.to_le_bytes();
        let end = 200u64.to_le_bytes();
        b.iter(|| {
            let _ = db
                .range(black_box("bench"), black_box(&start), black_box(&end))
                .unwrap();
        })
    });

    // ── TABLE_ROW_COUNT ────────────────────────────────────
    group.bench_function("table_row_count_1k", |b| {
        let db = Database::open_in_memory().unwrap();
        for i in 0u64..1000 {
            db.insert("bench", &i.to_le_bytes(), b"value_data_here")
                .unwrap();
        }
        b.iter(|| {
            let _ = db.table_row_count(black_box("bench")).unwrap();
        })
    });

    // ── BATCH INSERT ───────────────────────────────────────
    group.bench_function("batch_insert_1k", |b| {
        let db = Database::open_in_memory().unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            let rows: Vec<(Vec<u8>, Vec<u8>)> = (0..1000)
                .map(|i| {
                    let key = (counter + i as u64).to_le_bytes().to_vec();
                    (key, b"value_data_here".to_vec())
                })
                .collect();
            db.insert_batch(black_box("bench"), black_box(rows))
                .unwrap();
            counter += 1000;
        })
    });

    // ── COUNT ──────────────────────────────────────────────
    group.bench_function("count_1k", |b| {
        let db = Database::open_in_memory().unwrap();
        for i in 0u64..1000 {
            db.insert("bench", &i.to_le_bytes(), b"value_data_here")
                .unwrap();
        }
        b.iter(|| {
            let _ = db.count(black_box("bench")).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_crud_operations);
criterion_main!(benches);
