//! SQL vs KV API 성능 벤치마크
//!
//! UPDATE와 DELETE의 SQL vs KV API 성능 비교

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;
use tempfile::tempdir;

/// 테스트용 데이터베이스 생성 및 초기 데이터 삽입
fn setup_db_with_data(num_rows: usize) -> Database {
    let db = Database::open_in_memory().unwrap();

    // CREATE TABLE
    db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT, score FLOAT)")
        .unwrap();

    // INSERT 데이터
    for i in 0..num_rows {
        let sql = format!(
            "INSERT INTO users VALUES ({}, 'User{}', {}, {})",
            i,
            i,
            20 + (i % 50),
            50.0 + (i as f64 * 0.1)
        );
        db.execute_sql(&sql).unwrap();
    }

    db
}

/// 인덱스가 있는 데이터베이스 생성
fn setup_db_with_index(num_rows: usize) -> Database {
    let db = setup_db_with_data(num_rows);

    // CREATE INDEX
    db.execute_sql("CREATE INDEX idx_age ON users(age)")
        .unwrap();

    db
}

// ============================================================================
// UPDATE 벤치마크
// ============================================================================

fn bench_update_sql_single(c: &mut Criterion) {
    c.bench_function("update_sql_single", |b| {
        let db = setup_db_with_data(1000);
        let mut counter = 0;

        b.iter(|| {
            let sql = format!(
                "UPDATE users SET score = {} WHERE id = {}",
                100.0 + counter as f64,
                counter % 1000
            );
            db.execute_sql(&sql).unwrap();
            counter += 1;
        });
    });
}

fn bench_update_kv_single(c: &mut Criterion) {
    c.bench_function("update_kv_single", |b| {
        let db = setup_db_with_data(1000);
        let mut counter = 0;

        b.iter(|| {
            let key = format!("user:{}", counter % 1000);
            let value = format!(
                "{{\"id\":{},\"name\":\"User{}\",\"age\":{},\"score\":{}}}",
                counter % 1000,
                counter % 1000,
                25,
                100.0 + counter as f64
            );
            db.put(black_box(key.as_bytes()), black_box(value.as_bytes()))
                .unwrap();
            counter += 1;
        });
    });
}

fn bench_update_sql_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_sql_batch");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_with_setup(
                || setup_db_with_data(size),
                |db| {
                    for i in 0..size {
                        let sql = format!(
                            "UPDATE users SET score = {} WHERE id = {}",
                            200.0 + i as f64,
                            i
                        );
                        db.execute_sql(&sql).unwrap();
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_update_kv_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("update_kv_batch");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_with_setup(
                || setup_db_with_data(size),
                |db| {
                    for i in 0..size {
                        let key = format!("user:{}", i);
                        let value = format!(
                            "{{\"id\":{},\"name\":\"User{}\",\"age\":{},\"score\":{}}}",
                            i,
                            i,
                            25,
                            200.0 + i as f64
                        );
                        db.put(key.as_bytes(), value.as_bytes()).unwrap();
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_update_sql_with_index(c: &mut Criterion) {
    c.bench_function("update_sql_with_index", |b| {
        let db = setup_db_with_index(1000);
        let mut counter = 0;

        b.iter(|| {
            let sql = format!(
                "UPDATE users SET score = {} WHERE age = {}",
                150.0 + counter as f64,
                20 + (counter % 50)
            );
            db.execute_sql(&sql).unwrap();
            counter += 1;
        });
    });
}

fn bench_update_kv_with_index(c: &mut Criterion) {
    c.bench_function("update_kv_with_index", |b| {
        let db = setup_db_with_index(1000);
        let mut counter = 0;

        b.iter(|| {
            // KV API는 인덱스를 직접 사용하지 않으므로 전체 스캔 필요
            let key = format!("user:{}", counter % 1000);
            let value = format!(
                "{{\"id\":{},\"name\":\"User{}\",\"age\":{},\"score\":{}}}",
                counter % 1000,
                counter % 1000,
                25,
                150.0 + counter as f64
            );
            db.put(key.as_bytes(), value.as_bytes()).unwrap();
            counter += 1;
        });
    });
}

// ============================================================================
// DELETE 벤치마크
// ============================================================================

fn bench_delete_sql_single(c: &mut Criterion) {
    c.bench_function("delete_sql_single", |b| {
        b.iter_with_setup(
            || setup_db_with_data(1000),
            |db| {
                db.execute_sql("DELETE FROM users WHERE id = 500").unwrap();
            },
        );
    });
}

fn bench_delete_kv_single(c: &mut Criterion) {
    c.bench_function("delete_kv_single", |b| {
        b.iter_with_setup(
            || setup_db_with_data(1000),
            |db| {
                db.delete(b"user:500").unwrap();
            },
        );
    });
}

fn bench_delete_sql_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_sql_batch");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_with_setup(
                || setup_db_with_data(size),
                |db| {
                    for i in 0..size {
                        let sql = format!("DELETE FROM users WHERE id = {}", i);
                        db.execute_sql(&sql).unwrap();
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_delete_kv_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_kv_batch");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_with_setup(
                || setup_db_with_data(size),
                |db| {
                    for i in 0..size {
                        let key = format!("user:{}", i);
                        db.delete(key.as_bytes()).unwrap();
                    }
                },
            );
        });
    }

    group.finish();
}

fn bench_delete_sql_with_index(c: &mut Criterion) {
    c.bench_function("delete_sql_with_index", |b| {
        b.iter_with_setup(
            || setup_db_with_index(1000),
            |db| {
                db.execute_sql("DELETE FROM users WHERE age = 25").unwrap();
            },
        );
    });
}

fn bench_delete_kv_with_index(c: &mut Criterion) {
    c.bench_function("delete_kv_with_index", |b| {
        b.iter_with_setup(
            || setup_db_with_index(1000),
            |db| {
                // KV API는 인덱스를 직접 사용하지 않으므로 개별 삭제
                for i in 0..1000 {
                    let key = format!("user:{}", i);
                    db.delete(key.as_bytes()).unwrap();
                }
            },
        );
    });
}

criterion_group!(
    update_benches,
    bench_update_sql_single,
    bench_update_kv_single,
    bench_update_sql_batch,
    bench_update_kv_batch,
    bench_update_sql_with_index,
    bench_update_kv_with_index
);

criterion_group!(
    delete_benches,
    bench_delete_sql_single,
    bench_delete_kv_single,
    bench_delete_sql_batch,
    bench_delete_kv_batch,
    bench_delete_sql_with_index,
    bench_delete_kv_with_index
);

criterion_main!(update_benches, delete_benches);
