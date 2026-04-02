//! Query Builder API 사용 예제
//!
//! 실행: cargo run --example query_builder
//!
//! 주의: Query Builder는 Phase 6에서 완성 예정

use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    println!("=== DBX Query Builder 예제 ===\n");

    let db = Database::open_in_memory()?;

    // 데이터 준비
    db.insert("products", b"prod:1", b"Laptop")?;
    db.insert("products", b"prod:2", b"Mouse")?;
    db.insert("products", b"prod:3", b"Keyboard")?;

    println!("주의: Query Builder는 Phase 6에서 완성 예정입니다.");
    println!("현재는 기본 CRUD API를 사용하세요.\n");

    // 예시 (Phase 6 완성 후):
    // let products = db.query("SELECT * FROM products WHERE price > ?")
    //     .bind(100)
    //     .fetch_all()?;

    println!("=== 예제 완료 ===");
    Ok(())
}
