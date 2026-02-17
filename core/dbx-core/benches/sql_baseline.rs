// SQL DDL Baseline Benchmark - DDL API 구현 전 SQL 성능 측정
//
// 사용법:
// cargo bench --bench sql_baseline

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use dbx_core::Database;

// ════════════════════════════════════════════
// SQL DDL Benchmarks
// ════════════════════════════════════════════

fn bench_sql_create_table(c: &mut Criterion) {
    c.bench_function("sql_create_table", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            black_box(
                db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT)")
                    .unwrap(),
            );
        });
    });
}

fn bench_sql_create_index(c: &mut Criterion) {
    c.bench_function("sql_create_index", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            db.execute_sql("CREATE TABLE users (id INT, name TEXT, age INT)")
                .unwrap();
            black_box(
                db.execute_sql("CREATE INDEX idx_name ON users (name)")
                    .unwrap(),
            );
        });
    });
}

fn bench_sql_drop_table(c: &mut Criterion) {
    c.bench_function("sql_drop_table", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            db.execute_sql("CREATE TABLE users (id INT, name TEXT)")
                .unwrap();
            black_box(db.execute_sql("DROP TABLE users").unwrap());
        });
    });
}

fn bench_sql_select(c: &mut Criterion) {
    c.bench_function("sql_select", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            db.execute_sql("CREATE TABLE users (id INT, name TEXT)")
                .unwrap();
            db.execute_sql("INSERT INTO users VALUES (1, 'Alice')")
                .unwrap();
            db.execute_sql("INSERT INTO users VALUES (2, 'Bob')")
                .unwrap();
            db.execute_sql("SELECT * FROM users").unwrap();
            black_box(());
            // Note: No DROP TABLE needed, db is dropped at end of iteration
        });
    });
}

criterion_group!(
    sql_baseline,
    bench_sql_create_table,
    bench_sql_create_index,
    bench_sql_drop_table,
    bench_sql_select
);
criterion_main!(sql_baseline);
