//! 로깅 시스템 사용 예제
//!
//! 실행: RUST_LOG=debug cargo run --example logging --features logging

use dbx_core::Database;

fn main() -> dbx_core::DbxResult<()> {
    // 로깅 초기화
    #[cfg(feature = "logging")]
    dbx_core::logging::init();

    println!("=== DBX 로깅 예제 ===\n");
    println!("환경 변수 RUST_LOG로 로그 레벨 조정 가능:");
    println!("  RUST_LOG=trace  - 모든 로그");
    println!("  RUST_LOG=debug  - 디버그 이상");
    println!("  RUST_LOG=info   - 정보 이상 (기본값)");
    println!("  RUST_LOG=warn   - 경고 이상");
    println!("  RUST_LOG=error  - 에러만\n");

    let db = Database::open_in_memory()?;

    // 로그가 출력되는 작업들
    println!("데이터 삽입 중...");
    db.insert("users", b"user:1", b"Alice")?;
    db.insert("users", b"user:2", b"Bob")?;
    db.insert("users", b"user:3", b"Charlie")?;

    println!("\n데이터 조회 중...");
    db.get("users", b"user:1")?;

    println!("\nFlush 실행 중...");
    db.flush()?;

    println!("\n=== 예제 완료 ===");
    println!("\n주의: logging feature가 활성화되어야 로그가 출력됩니다.");
    println!("실행: RUST_LOG=debug cargo run --example logging --features logging");

    Ok(())
}
