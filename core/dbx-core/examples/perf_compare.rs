//! 빠른 성능 비교: DBX vs SQLite3 (1분 이내 실행)
//!
//! 실행: cargo run --release --example perf_compare

use dbx_core::Database;
use rusqlite::Connection;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 DBX vs SQLite3 - Quick Performance Test\n");

    let row_count = 100_000;
    println!("Testing with {} rows...\n", row_count);

    // ========== DBX Test ==========
    println!("📊 DBX Performance:");
    let dbx_start = Instant::now();

    let db = Database::open_in_memory()?;

    // Insert (트랜잭션 + 배치 사용)
    let insert_start = Instant::now();
    let mut tx = db.begin()?;

    // 배치로 묶어서 삽입
    let mut rows = Vec::with_capacity(row_count);
    for i in 0..row_count {
        let key = format!("user_{}", i).into_bytes();
        let value = format!("data_{}", i).into_bytes();
        rows.push((key, value));
    }
    tx.insert_batch("users", rows)?;
    tx.commit()?;
    let dbx_insert = insert_start.elapsed();

    // Get
    let get_start = Instant::now();
    for i in 0..1000 {
        let key = format!("user_{}", i);
        let _ = db.get("users", key.as_bytes())?;
    }
    let dbx_get = get_start.elapsed();

    let dbx_total = dbx_start.elapsed();

    println!(
        "  Insert {}K: {:?} ({:.0} ops/sec)",
        row_count / 1000,
        dbx_insert,
        row_count as f64 / dbx_insert.as_secs_f64()
    );
    println!(
        "  Get 1K:     {:?} ({:.0} ops/sec)",
        dbx_get,
        1000.0 / dbx_get.as_secs_f64()
    );
    println!("  Total:      {:?}\n", dbx_total);

    // ========== SQLite3 Test ==========
    println!("📊 SQLite3 Performance:");
    let sqlite_start = Instant::now();

    let conn = Connection::open_in_memory()?;
    conn.execute("CREATE TABLE users (key TEXT PRIMARY KEY, value TEXT)", [])?;

    // Insert
    let insert_start = Instant::now();
    conn.execute("BEGIN TRANSACTION", [])?;
    for i in 0..row_count {
        conn.execute(
            "INSERT INTO users (key, value) VALUES (?1, ?2)",
            [&format!("user_{}", i), &format!("data_{}", i)],
        )?;
    }
    conn.execute("COMMIT", [])?;
    let sqlite_insert = insert_start.elapsed();

    // Get
    let get_start = Instant::now();
    for i in 0..1000 {
        let key = format!("user_{}", i);
        let mut stmt = conn.prepare("SELECT value FROM users WHERE key = ?1")?;
        let _value: String = stmt.query_row([&key], |row| row.get(0))?;
    }
    let sqlite_get = get_start.elapsed();

    let sqlite_total = sqlite_start.elapsed();

    println!(
        "  Insert {}K: {:?} ({:.0} ops/sec)",
        row_count / 1000,
        sqlite_insert,
        row_count as f64 / sqlite_insert.as_secs_f64()
    );
    println!(
        "  Get 1K:     {:?} ({:.0} ops/sec)",
        sqlite_get,
        1000.0 / sqlite_get.as_secs_f64()
    );
    println!("  Total:      {:?}\n", sqlite_total);

    // ========== Comparison ==========
    println!("📈 Performance Comparison:");

    let insert_ratio = if dbx_insert < sqlite_insert {
        sqlite_insert.as_secs_f64() / dbx_insert.as_secs_f64()
    } else {
        dbx_insert.as_secs_f64() / sqlite_insert.as_secs_f64()
    };

    let get_ratio = if dbx_get < sqlite_get {
        sqlite_get.as_secs_f64() / dbx_get.as_secs_f64()
    } else {
        dbx_get.as_secs_f64() / sqlite_get.as_secs_f64()
    };

    println!(
        "  Insert: DBX is {:.2}x {} than SQLite3",
        insert_ratio,
        if dbx_insert < sqlite_insert {
            "faster"
        } else {
            "slower"
        }
    );

    println!(
        "  Get:    DBX is {:.2}x {} than SQLite3",
        get_ratio,
        if dbx_get < sqlite_get {
            "faster"
        } else {
            "slower"
        }
    );

    println!("\n✨ Test completed in {:?}", dbx_start.elapsed());
    Ok(())
}
