// DDL API Benchmark - DDL API vs SQL 성능 비교
//
// 사용법:
// cargo bench --bench ddl_api_benchmark

use arrow::datatypes::{DataType, Field, Schema};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dbx_core::Database;

// ════════════════════════════════════════════
// Table Management Benchmarks
// ════════════════════════════════════════════

fn bench_api_create_table(c: &mut Criterion) {
    c.bench_function("api_create_table", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("name", DataType::Utf8, true),
                Field::new("age", DataType::Int32, true),
            ]);
            black_box(db.create_table("users", schema).unwrap());
        });
    });
}

fn bench_api_drop_table(c: &mut Criterion) {
    c.bench_function("api_drop_table", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);
            db.create_table("temp", schema).unwrap();
            black_box(db.drop_table("temp").unwrap());
        });
    });
}

fn bench_api_table_exists(c: &mut Criterion) {
    c.bench_function("api_table_exists", |b| {
        let db = Database::open_in_memory().unwrap();
        let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);
        db.create_table("users", schema).unwrap();

        b.iter(|| {
            black_box(db.table_exists("users"));
        });
    });
}

fn bench_api_list_tables(c: &mut Criterion) {
    c.bench_function("api_list_tables", |b| {
        let db = Database::open_in_memory().unwrap();
        let schema = Schema::new(vec![Field::new("id", DataType::Int64, false)]);
        db.create_table("users", schema.clone()).unwrap();
        db.create_table("orders", schema).unwrap();

        b.iter(|| {
            black_box(db.list_tables());
        });
    });
}

// ════════════════════════════════════════════
// Index Management Benchmarks
// ════════════════════════════════════════════

fn bench_api_create_sql_index(c: &mut Criterion) {
    c.bench_function("api_create_sql_index", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("email", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.create_sql_index("users", "idx_email", vec!["email".to_string()]).unwrap());
        });
    });
}

fn bench_api_drop_sql_index(c: &mut Criterion) {
    c.bench_function("api_drop_sql_index", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("email", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            db.create_sql_index("users", "idx_email", vec!["email".to_string()]).unwrap();
            black_box(db.drop_sql_index("users", "idx_email").unwrap());
        });
    });
}

fn bench_api_sql_index_exists(c: &mut Criterion) {
    c.bench_function("api_sql_index_exists", |b| {
        let db = Database::open_in_memory().unwrap();
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("email", DataType::Utf8, true),
        ]);
        db.create_table("users", schema).unwrap();
        db.create_sql_index("users", "idx_email", vec!["email".to_string()]).unwrap();

        b.iter(|| {
            black_box(db.sql_index_exists("idx_email"));
        });
    });
}

fn bench_api_list_sql_indexes(c: &mut Criterion) {
    c.bench_function("api_list_sql_indexes", |b| {
        let db = Database::open_in_memory().unwrap();
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("email", DataType::Utf8, true),
            Field::new("name", DataType::Utf8, true),
        ]);
        db.create_table("users", schema).unwrap();
        db.create_sql_index("users", "idx_email", vec!["email".to_string()]).unwrap();
        db.create_sql_index("users", "idx_name", vec!["name".to_string()]).unwrap();

        b.iter(|| {
            black_box(db.list_sql_indexes("users"));
        });
    });
}

// ════════════════════════════════════════════
// ALTER TABLE Benchmarks
// ════════════════════════════════════════════

fn bench_api_add_column(c: &mut Criterion) {
    c.bench_function("api_add_column", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("name", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.add_column("users", "email", "TEXT").unwrap());
        });
    });
}

fn bench_api_drop_column(c: &mut Criterion) {
    c.bench_function("api_drop_column", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("name", DataType::Utf8, true),
                Field::new("email", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.drop_column("users", "email").unwrap());
        });
    });
}

fn bench_api_rename_column(c: &mut Criterion) {
    c.bench_function("api_rename_column", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("user_name", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.rename_column("users", "user_name", "name").unwrap());
        });
    });
}

// ════════════════════════════════════════════
// SQL vs DDL API Comparison
// ════════════════════════════════════════════

fn bench_sql_create_index(c: &mut Criterion) {
    c.bench_function("sql_create_index", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("email", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.execute_sql("CREATE INDEX idx_email ON users (email)").unwrap());
        });
    });
}

fn bench_sql_alter_table_add(c: &mut Criterion) {
    c.bench_function("sql_alter_table_add", |b| {
        b.iter(|| {
            let db = Database::open_in_memory().unwrap();
            let schema = Schema::new(vec![
                Field::new("id", DataType::Int64, false),
                Field::new("name", DataType::Utf8, true),
            ]);
            db.create_table("users", schema).unwrap();
            black_box(db.execute_sql("ALTER TABLE users ADD COLUMN email TEXT").unwrap());
        });
    });
}

criterion_group!(
    table_management,
    bench_api_create_table,
    bench_api_drop_table,
    bench_api_table_exists,
    bench_api_list_tables
);

criterion_group!(
    index_management,
    bench_api_create_sql_index,
    bench_api_drop_sql_index,
    bench_api_sql_index_exists,
    bench_api_list_sql_indexes,
    bench_sql_create_index
);

criterion_group!(
    alter_table,
    bench_api_add_column,
    bench_api_drop_column,
    bench_api_rename_column,
    bench_sql_alter_table_add
);

criterion_main!(table_management, index_management, alter_table);
