//! 기본 CRUD 작업 예제
//!
//! 실행: cargo run --example basic_crud

use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    println!("=== DBX 기본 CRUD 예제 ===\n");

    // 1. 인메모리 데이터베이스 생성
    println!("1. 데이터베이스 생성...");
    let db = Database::open_in_memory()?;
    println!("   ✓ 인메모리 데이터베이스 생성 완료\n");

    // 2. 데이터 삽입 (Create)
    println!("2. 데이터 삽입...");
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    db.insert("users", b"user:3", b"Charlie")?;
    println!("   ✓ 3개 레코드 삽입 완료\n");

    // 3. 데이터 조회 (Read)
    println!("3. 데이터 조회...");
    if let Some(value) = db.get("users", b"user:1")? {
        println!("   user:1 = {}", String::from_utf8_lossy(&value));
    }
    if let Some(value) = db.get("users", b"user:2")? {
        println!("   user:2 = {}", String::from_utf8_lossy(&value));
    }
    println!();

    // 4. 데이터 수정 (Update) — 같은 키로 다시 삽입
    println!("4. 데이터 수정...");
    db.insert("users", b"user:1", b"Alice Updated")?;
    if let Some(value) = db.get("users", b"user:1")? {
        println!("   user:1 = {} (수정됨)", String::from_utf8_lossy(&value));
    }
    println!();

    // 5. 데이터 삭제 (Delete)
    println!("5. 데이터 삭제...");
    db.delete("users", b"user:3")?;
    println!("   ✓ user:3 삭제 완료");

    if db.get("users", b"user:3")?.is_none() {
        println!("   ✓ user:3이 존재하지 않음 확인\n");
    }

    // 6. 통계 확인
    println!("6. 통계 확인...");
    let count = db.count("users")?;
    println!("   총 레코드 수: {}\n", count);

    // 7. Flush (Delta → WOS)
    println!("7. Flush 실행...");
    db.flush()?;
    println!("   ✓ Delta Store → WOS 이동 완료\n");

    println!("=== 예제 완료 ===");
    Ok(())
}
