//! 트랜잭션 사용 예제
//!
//! 실행: cargo run --example transactions

use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    println!("=== DBX 트랜잭션 예제 ===\n");

    let db = Database::open_in_memory()?;

    // 1. 기본 트랜잭션
    println!("1. 기본 트랜잭션...");
    {
        let _tx = db.begin()?;
        db.insert("accounts", b"acc:1", b"1000")?;
        db.insert("accounts", b"acc:2", b"2000")?;
        println!("   ✓ 트랜잭션 내 2개 삽입\n");
    }

    // 2. Typestate 패턴 (컴파일 타임 안전성)
    println!("2. Typestate 트랜잭션...");
    let tx = db.begin()?;
    println!("   ✓ 트랜잭션 시작 (Active 상태)");

    // Active 상태에서만 작업 가능
    db.insert("orders", b"order:1", b"item:A")?;

    // 주석: commit() 후에는 insert 불가 (컴파일 에러)
    // let tx = tx.commit()?;
    // tx.insert(...) // ← 컴파일 에러!

    println!("   ✓ Typestate 패턴으로 안전성 보장\n");

    // 트랜잭션 종료 (drop)
    drop(tx);

    // 3. 통계 확인
    let count = db.count("accounts")?;
    println!("3. 계좌 수: {}\n", count);

    println!("=== 예제 완료 ===");
    Ok(())
}
