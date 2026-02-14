//! Quick DB Comparison Test (1분 이내 실행)
//!
//! db_comparison.rs 기반, 워밍업/반복 최소화
//! DBX vs SQLite3 vs Sled vs Redb
//!
//! 실행: cargo run --release --example quick_db_compare

use dbx_core::Database;
use redb::{Database as RedbDatabase, ReadableTable, TableDefinition};
use rusqlite::Connection;
use std::time::Instant;
use tempfile::TempDir;

const TEST_SIZE: usize = 10_000;
const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("bench");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Quick DB Comparison Test (10K rows)\n");
    println!("Testing: DBX vs SQLite3 vs Sled vs Redb\n");

    // ========== DBX ==========
    println!("📊 DBX (In-Memory):");
    let start = Instant::now();
    let db = Database::open_in_memory()?;
    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_data_{}", i).into_bytes();
        db.insert("bench", &key, &value)?;
    }
    db.flush()?;
    let dbx_insert = start.elapsed();

    let start = Instant::now();
    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i).into_bytes();
        let _ = db.get("bench", &key)?;
    }
    let dbx_get = start.elapsed();

    println!(
        "  Insert: {:?} ({:.0} ops/sec)",
        dbx_insert,
        TEST_SIZE as f64 / dbx_insert.as_secs_f64()
    );
    println!(
        "  Get:    {:?} ({:.0} ops/sec)\n",
        dbx_get,
        TEST_SIZE as f64 / dbx_get.as_secs_f64()
    );

    // ========== SQLite3 ==========
    println!("📊 SQLite3 (In-Memory):");
    let start = Instant::now();
    let conn = Connection::open_in_memory()?;
    conn.execute("CREATE TABLE bench (key TEXT PRIMARY KEY, value TEXT)", [])?;

    let tx = conn.unchecked_transaction()?;
    for i in 0..TEST_SIZE {
        tx.execute(
            "INSERT INTO bench (key, value) VALUES (?1, ?2)",
            [&format!("key_{}", i), &format!("value_data_{}", i)],
        )?;
    }
    tx.commit()?;
    let sqlite_insert = start.elapsed();

    let start = Instant::now();
    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i);
        let mut stmt = conn.prepare_cached("SELECT value FROM bench WHERE key = ?1")?;
        let _: String = stmt.query_row([&key], |row| row.get(0))?;
    }
    let sqlite_get = start.elapsed();

    println!(
        "  Insert: {:?} ({:.0} ops/sec)",
        sqlite_insert,
        TEST_SIZE as f64 / sqlite_insert.as_secs_f64()
    );
    println!(
        "  Get:    {:?} ({:.0} ops/sec)\n",
        sqlite_get,
        TEST_SIZE as f64 / sqlite_get.as_secs_f64()
    );

    // ========== Sled ==========
    println!("📊 Sled (In-Memory):");
    let start = Instant::now();
    let config = sled::Config::new().temporary(true);
    let sled_db = config.open()?;

    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_data_{}", i).into_bytes();
        sled_db.insert(&key, value)?;
    }
    sled_db.flush()?;
    let sled_insert = start.elapsed();

    let start = Instant::now();
    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i).into_bytes();
        let _ = sled_db.get(&key)?;
    }
    let sled_get = start.elapsed();

    println!(
        "  Insert: {:?} ({:.0} ops/sec)",
        sled_insert,
        TEST_SIZE as f64 / sled_insert.as_secs_f64()
    );
    println!(
        "  Get:    {:?} ({:.0} ops/sec)\n",
        sled_get,
        TEST_SIZE as f64 / sled_get.as_secs_f64()
    );

    // ========== Redb ==========
    println!("📊 Redb (File-based):");
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("redb_bench.db");

    let start = Instant::now();
    let redb = RedbDatabase::create(&db_path)?;
    let write_txn = redb.begin_write()?;
    {
        let mut table = write_txn.open_table(TABLE)?;
        for i in 0..TEST_SIZE {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_data_{}", i).into_bytes();
            table.insert(key.as_slice(), value.as_slice())?;
        }
    }
    write_txn.commit()?;
    let redb_insert = start.elapsed();

    let start = Instant::now();
    let read_txn = redb.begin_read()?;
    let table = read_txn.open_table(TABLE)?;
    for i in 0..TEST_SIZE {
        let key = format!("key_{}", i).into_bytes();
        let _ = table.get(key.as_slice())?;
    }
    let redb_get = start.elapsed();

    println!(
        "  Insert: {:?} ({:.0} ops/sec)",
        redb_insert,
        TEST_SIZE as f64 / redb_insert.as_secs_f64()
    );
    println!(
        "  Get:    {:?} ({:.0} ops/sec)\n",
        redb_get,
        TEST_SIZE as f64 / redb_get.as_secs_f64()
    );

    // ========== Comparison ==========
    println!("📈 Performance Comparison (vs DBX):");

    println!("\n  INSERT:");
    println!(
        "    SQLite3: {:.2}x {}",
        dbx_insert.as_secs_f64() / sqlite_insert.as_secs_f64(),
        if dbx_insert < sqlite_insert {
            "faster"
        } else {
            "slower"
        }
    );
    println!(
        "    Sled:    {:.2}x {}",
        dbx_insert.as_secs_f64() / sled_insert.as_secs_f64(),
        if dbx_insert < sled_insert {
            "faster"
        } else {
            "slower"
        }
    );
    println!(
        "    Redb:    {:.2}x {}",
        dbx_insert.as_secs_f64() / redb_insert.as_secs_f64(),
        if dbx_insert < redb_insert {
            "faster"
        } else {
            "slower"
        }
    );

    println!("\n  GET:");
    println!(
        "    SQLite3: {:.2}x {}",
        dbx_get.as_secs_f64() / sqlite_get.as_secs_f64(),
        if dbx_get < sqlite_get {
            "faster"
        } else {
            "slower"
        }
    );
    println!(
        "    Sled:    {:.2}x {}",
        dbx_get.as_secs_f64() / sled_get.as_secs_f64(),
        if dbx_get < sled_get {
            "faster"
        } else {
            "slower"
        }
    );
    println!(
        "    Redb:    {:.2}x {}",
        dbx_get.as_secs_f64() / redb_get.as_secs_f64(),
        if dbx_get < redb_get {
            "faster"
        } else {
            "slower"
        }
    );

    println!("\n✨ Test completed!");
    Ok(())
}
