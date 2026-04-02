// Phase 0.3: HTAP 벤치마크
//
// HTAP(Hybrid Transactional/Analytical Processing) 워크로드에서의 DBX 성능을 측정합니다.
//
// Section 1: OLTP 단독 처리량 (INSERT TPS)
// Section 2: OLAP 단독 처리량 (SQL 집계 QPS)
// Section 3: HTAP 동시 워크로드 (OLTP + OLAP 병렬 실행 간섭도)
// Section 4: WorkloadAnalyzer 전환 오버헤드
// Section 5: 모니터링 메트릭 기록 비용

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use dbx_core::engine::workload_analyzer::{QueryPattern, WorkloadAnalyzer};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ═══════════════════════════════════════════════════════════════════════════
// 공통 헬퍼
// ═══════════════════════════════════════════════════════════════════════════

/// 인메모리 DB에 테이블 세팅 + N개 행 사전 삽입
fn setup_db_with_rows(n: usize) -> Database {
    let db = Database::open_in_memory().expect("open_in_memory failed");
    for i in 0..n {
        db.insert(
            "bench",
            format!("key:{i:08}").as_bytes(),
            b"value_data_here",
        )
        .expect("insert failed");
    }
    db
}

/// SQL 집계용 DB: CREATE TABLE + N행 INSERT
fn setup_sql_db(n: usize) -> Database {
    let db = Database::open_in_memory().expect("open_in_memory failed");
    db.execute_sql("CREATE TABLE metrics (id INTEGER, score INTEGER)")
        .expect("CREATE TABLE failed");
    for i in 0..n as i64 {
        db.execute_sql(&format!("INSERT INTO metrics VALUES ({i}, {})", i * 3))
            .expect("INSERT failed");
    }
    db
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1: OLTP 단독 처리량 (INSERT TPS)
// ═══════════════════════════════════════════════════════════════════════════

fn bench_oltp_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("htap/oltp_throughput");

    for &n in &[1_000usize, 10_000] {
        group.bench_with_input(BenchmarkId::new("insert", n), &n, |b, &n| {
            b.iter(|| {
                let db = Database::open_in_memory().expect("db open");
                for i in 0..n {
                    db.insert(
                        black_box("events"),
                        black_box(format!("k:{i:010}").as_bytes()),
                        black_box(b"payload_data"),
                    )
                    .expect("insert");
                }
            });
        });
    }

    // 배치 INSERT 비교
    group.bench_function("insert_batch_1k", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().expect("db open");
            let rows: Vec<(Vec<u8>, Vec<u8>)> = (0..1_000)
                .map(|i| (format!("k:{i:010}").into_bytes(), b"payload".to_vec()))
                .collect();
            db.insert_batch(black_box("events"), black_box(rows))
                .expect("batch insert");
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2: OLAP 단독 처리량 (SQL 집계 QPS)
// ═══════════════════════════════════════════════════════════════════════════

fn bench_olap_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("htap/olap_throughput");
    group.measurement_time(Duration::from_secs(5));

    // COUNT(*) — 1,000행
    {
        let db = setup_sql_db(1_000);
        group.bench_function("sql_count_1k", |b| {
            b.iter(|| {
                let _ = db
                    .execute_sql(black_box("SELECT COUNT(*) FROM metrics"))
                    .expect("count");
            });
        });
    }

    // COUNT(*) + SUM — 5,000행 (개별 쿼리로 분리)
    {
        let db = setup_sql_db(5_000);
        group.bench_function("sql_count_5k", |b| {
            b.iter(|| {
                let _ = db
                    .execute_sql(black_box("SELECT COUNT(*) FROM metrics"))
                    .expect("count");
            });
        });

        // WHERE 조건부 COUNT — OLAP 필터링 패턴 (현재 DBX SQL 지원 집계 확인)
        group.bench_function("sql_count_where_5k", |b| {
            b.iter(|| {
                let _ = db
                    .execute_sql(black_box("SELECT COUNT(*) FROM metrics WHERE id > 2500"))
                    .expect("count where");
            });
        });

        group.bench_function("sql_sum_10k_error_test", |b| {
            b.iter(|| {
                let _ = db
                    .execute_sql(black_box("SELECT SUM(score) FROM metrics"))
                    .expect("sum error");
            });
        });
    }

    // KV 전체 스캔 — 10,000행 (OLAP 패턴)
    {
        let db = setup_db_with_rows(10_000);
        group.bench_function("kv_scan_10k", |b| {
            b.iter(|| {
                let _ = db.scan(black_box("bench")).expect("scan");
            });
        });
    }

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3: HTAP 동시 워크로드 (OLTP 쓰기 + OLAP 읽기 병렬)
//
// OLTP 스레드: 연속으로 KV INSERT
// OLAP 스레드: 연속으로 SQL COUNT(*) 실행
// 두 워크로드를 Arc<Database>로 공유하며 동시 실행하여 간섭도 측정
// ═══════════════════════════════════════════════════════════════════════════

fn bench_htap_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("htap/concurrent");
    // 동시성 벤치는 측정 시간을 충분히 확보
    group.measurement_time(Duration::from_secs(8));

    // ── 3-1. 1 OLTP 스레드 + 1 OLAP 스레드 ──────────────────────────────
    group.bench_function("1oltp_1olap", |b| {
        b.iter(|| {
            // SQL 테이블 준비
            let db = Arc::new(setup_sql_db(2_000));

            let db_oltp = Arc::clone(&db);
            let db_olap = Arc::clone(&db);

            // OLTP 쓰기 스레드: 200건 KV INSERT
            let oltp = thread::spawn(move || {
                for i in 0..200usize {
                    let _ = db_oltp.insert(
                        "events",
                        format!("htap:k:{i:06}").as_bytes(),
                        b"concurrent_write",
                    );
                }
            });

            // OLAP 읽기 스레드: COUNT(*) 10회
            let olap = thread::spawn(move || {
                for _ in 0..10 {
                    let _ = db_olap.execute_sql("SELECT COUNT(*) FROM metrics");
                }
            });

            oltp.join().expect("oltp thread");
            olap.join().expect("olap thread");
        });
    });

    // ── 3-2. 4 OLTP 스레드 + 1 OLAP 스레드 ──────────────────────────────
    group.bench_function("4oltp_1olap", |b| {
        b.iter(|| {
            let db = Arc::new(setup_sql_db(2_000));

            // 4개 OLTP 스레드 생성
            let mut oltp_handles = Vec::new();
            for t in 0..4usize {
                let db_t = Arc::clone(&db);
                let h = thread::spawn(move || {
                    for i in 0..50usize {
                        let key = format!("t{t}:k:{i:04}");
                        let _ = db_t.insert("events", key.as_bytes(), b"write");
                    }
                });
                oltp_handles.push(h);
            }

            // 1개 OLAP 스레드
            let db_olap = Arc::clone(&db);
            let olap = thread::spawn(move || {
                for _ in 0..10 {
                    let _ = db_olap.execute_sql("SELECT COUNT(*) FROM metrics");
                }
            });

            for h in oltp_handles {
                h.join().expect("oltp thread");
            }
            olap.join().expect("olap thread");
        });
    });

    // ── 3-3. 읽기/쓰기 비율 비교: write-heavy vs read-heavy ──────────────
    group.bench_function("htap_write_heavy", |b| {
        // OLTP 90% / OLAP 10%
        b.iter(|| {
            let db = Arc::new(setup_db_with_rows(500));
            let db_w = Arc::clone(&db);
            let db_r = Arc::clone(&db);

            let writer = thread::spawn(move || {
                for i in 0..900usize {
                    let _ = db_w.insert("bench", format!("wh:{i}").as_bytes(), b"v");
                }
            });
            let reader = thread::spawn(move || {
                for i in 0..100usize {
                    let _ = db_r.get("bench", format!("key:{i:08}").as_bytes());
                }
            });
            writer.join().expect("writer");
            reader.join().expect("reader");
        });
    });

    group.bench_function("htap_read_heavy", |b| {
        // OLTP 10% / OLAP 90%
        b.iter(|| {
            let db = Arc::new(setup_db_with_rows(500));
            let db_w = Arc::clone(&db);
            let db_r = Arc::clone(&db);

            let writer = thread::spawn(move || {
                for i in 0..100usize {
                    let _ = db_w.insert("bench", format!("rh:{i}").as_bytes(), b"v");
                }
            });
            let reader = thread::spawn(move || {
                for i in 0..900usize {
                    // 순환 접근 (캐시 히트 패턴)
                    let _ = db_r.get("bench", format!("key:{:08}", i % 500).as_bytes());
                }
            });
            writer.join().expect("writer");
            reader.join().expect("reader");
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4: WorkloadAnalyzer 전환 오버헤드
//
// record() + recommended_config() 호출 비용 측정
// ═══════════════════════════════════════════════════════════════════════════

fn bench_workload_switch(c: &mut Criterion) {
    let mut group = c.benchmark_group("htap/workload_analyzer");

    // record() 단독 오버헤드
    group.bench_function("record_1k_mixed", |b| {
        let mut analyzer = WorkloadAnalyzer::new(1_000);
        let mut counter = 0usize;
        b.iter(|| {
            // OLTP / OLAP 교차 기록 (실제 HTAP 패턴)
            let pattern = if counter % 3 == 0 {
                QueryPattern::Aggregation
            } else {
                QueryPattern::PointQuery
            };
            analyzer.record(black_box(pattern));
            counter += 1;
        });
    });

    // recommended_config() 오버헤드 (ratio 계산 포함)
    group.bench_function("recommended_config", |b| {
        let mut analyzer = WorkloadAnalyzer::new(1_000);
        // 미리 워밍업
        for i in 0..1_000 {
            if i % 4 == 0 {
                analyzer.record(QueryPattern::Aggregation);
            } else {
                analyzer.record(QueryPattern::PointQuery);
            }
        }
        b.iter(|| {
            let _ = black_box(analyzer.recommended_config());
        });
    });

    // record + config 풀 사이클 (실제 쿼리당 호출 패턴)
    group.bench_function("full_cycle_100", |b| {
        b.iter(|| {
            let mut analyzer = WorkloadAnalyzer::new(1_000);
            for i in 0..100usize {
                let pattern = if i % 3 == 0 {
                    QueryPattern::RangeScan
                } else {
                    QueryPattern::PointQuery
                };
                analyzer.record(black_box(pattern));
                // 매 10회마다 설정 추천 확인 (실제 패턴)
                if i % 10 == 0 {
                    let _ = black_box(analyzer.recommended_config());
                }
            }
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5: 모니터링 메트릭 기록 비용
//
// Phase 0.4 모니터링 모듈의 원자 연산 오버헤드를 측정합니다.
// ═══════════════════════════════════════════════════════════════════════════

fn bench_monitoring_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("htap/monitoring");

    // metrics_snapshot() 비용 (AtomicU64 로드 × N)
    group.bench_function("snapshot_read", |b| {
        let db = setup_db_with_rows(100);
        b.iter(|| {
            let _ = black_box(db.metrics_snapshot());
        });
    });

    // export_metrics() — Prometheus 텍스트 포맷 생성 비용
    group.bench_function("prometheus_export", |b| {
        let db = setup_db_with_rows(100);
        // INSERT 몇 개 실행해서 카운터에 값이 있는 상태로 측정
        for i in 0..50usize {
            let _ = db.get("bench", format!("key:{i:08}").as_bytes());
        }
        b.iter(|| {
            let _ = black_box(db.export_metrics());
        });
    });

    // insert() 포함 단위 비용 vs 순수 KV 비용
    // (메트릭 카운터 inc_inserts()가 총 비용에 미치는 영향)
    group.bench_function("insert_with_metrics_1k", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().expect("open");
            for i in 0..1_000usize {
                let _ = db.insert(
                    black_box("bench"),
                    black_box(format!("m:{i:08}").as_bytes()),
                    black_box(b"val"),
                );
            }
            // 최종 메트릭 스냅샷 확인 (각 DB 인스턴스는 독립 카운터)
            let snap = db.metrics_snapshot();
            assert!(snap.inserts_total >= 1_000, "expected >=1000 inserts");
        });
    });

    // reset_metrics() 비용
    group.bench_function("reset_metrics", |b| {
        let db = setup_db_with_rows(1_000);
        b.iter(|| {
            db.reset_metrics();
        });
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// Criterion 등록
// ═══════════════════════════════════════════════════════════════════════════

criterion_group!(
    benches,
    bench_oltp_throughput,
    bench_olap_throughput,
    bench_htap_concurrent,
    bench_workload_switch,
    bench_monitoring_overhead,
);
criterion_main!(benches);
