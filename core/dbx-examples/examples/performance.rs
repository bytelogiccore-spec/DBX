//! 성능 벤치마크 예제
//!
//! 실행: cargo run --example performance --release

use dbx_core::Database;
use std::time::Instant;

fn main() -> dbx_core::DbxResult<()> {
    println!("=== DBX 성능 벤치마크 예제 ===\n");

    let db = Database::open_in_memory()?;

    // 1. 대량 삽입 성능
    println!("1. 대량 삽입 (10,000개)...");
    let start = Instant::now();
    for i in 0..10_000 {
        let key = format!("key:{}", i);
        let value = format!("value:{}", i);
        db.insert("bench", key.as_bytes(), value.as_bytes())?;
    }
    let elapsed = start.elapsed();
    println!("   시간: {:?}", elapsed);
    println!(
        "   처리량: {:.0} ops/sec\n",
        10_000.0 / elapsed.as_secs_f64()
    );

    // 2. Flush 성능
    println!("2. Flush (Delta → WOS)...");
    let start = Instant::now();
    db.flush()?;
    let elapsed = start.elapsed();
    println!("   시간: {:?}\n", elapsed);

    // 3. 조회 성능
    println!("3. 조회 (10,000개)...");
    let start = Instant::now();
    for i in 0..10_000 {
        let key = format!("key:{}", i);
        db.get("bench", key.as_bytes())?;
    }
    let elapsed = start.elapsed();
    println!("   시간: {:?}", elapsed);
    println!(
        "   처리량: {:.0} ops/sec\n",
        10_000.0 / elapsed.as_secs_f64()
    );

    println!("=== 예제 완료 ===");
    Ok(())
}
