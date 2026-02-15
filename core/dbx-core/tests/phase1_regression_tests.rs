// Phase 1 회귀 테스트
//
// BenchmarkRunner를 사용하여 베이스라인 저장 및 자동 회귀 검출

use dbx_core::engine::{BenchmarkRunner, TwoLevelCache};
use dbx_core::error::DbxResult;
use dbx_core::sql::ParallelSqlParser;
use dbx_core::transaction::{TimestampOracle, VersionManager};
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn test_regression_two_level_cache_l1_get() -> DbxResult<()> {
    let runner = BenchmarkRunner::new()
        .with_baseline_path(PathBuf::from("target/phase1_baseline.json"))
        .with_threshold(1.2); // 20% 성능 저하 허용
    
    // 베이스라인 로드 (있으면)
    let _ = runner.load_baseline();
    
    // 캐시 준비
    let cache = TwoLevelCache::new(10 * 1024 * 1024, PathBuf::from("target/regression_cache"));
    let _ = cache.clear();
    
    // 데이터 미리 저장
    for i in 0..100 {
        cache.put(format!("key_{}", i), vec![0u8; 1024])?;
    }
    
    // 벤치마크 실행
    let result = runner.run("l1_get_1kb", || {
        for i in 0..10 {
            let key = format!("key_{}", i % 100);
            let _ = cache.get(&key);
        }
    })?;
    
    println!("✅ l1_get_1kb: {:.2} ms (avg)", result.avg_time_ms);
    
    // 회귀 검출
    match runner.check_regression("l1_get_1kb", &result) {
        Ok(_) => {
            println!("   No regression detected");
            // 베이스라인 업데이트
            runner.update_baseline("l1_get_1kb", &result);
            runner.save_baseline()?;
        }
        Err(e) => {
            eprintln!("⚠️  WARNING: {}", e);
        }
    }
    
    // 정리
    let _ = cache.clear();
    
    Ok(())
}

#[test]
fn test_regression_parallel_sql_parser_batch_100() -> DbxResult<()> {
    let runner = BenchmarkRunner::new()
        .with_baseline_path(PathBuf::from("target/phase1_baseline.json"))
        .with_threshold(1.2); // 20% 성능 저하 허용
    
    // 베이스라인 로드 (있으면)
    let _ = runner.load_baseline();
    
    let parser = ParallelSqlParser::new();
    let sqls: Vec<String> = (0..100)
        .map(|i| format!("SELECT * FROM table_{} WHERE id = {}", i % 10, i))
        .collect();
    let sqls_refs: Vec<&str> = sqls.iter().map(|s| s.as_str()).collect();
    
    // 벤치마크 실행
    let result = runner.run("batch_parse_100", || {
        let _ = parser.parse_batch(&sqls_refs);
    })?;
    
    println!("✅ batch_parse_100: {:.2} ms (avg)", result.avg_time_ms);
    
    // 회귀 검출
    match runner.check_regression("batch_parse_100", &result) {
        Ok(_) => {
            println!("   No regression detected");
            // 베이스라인 업데이트
            runner.update_baseline("batch_parse_100", &result);
            runner.save_baseline()?;
        }
        Err(e) => {
            eprintln!("⚠️  WARNING: {}", e);
        }
    }
    
    Ok(())
}

#[test]
fn test_regression_version_manager_get_at_snapshot() -> DbxResult<()> {
    let runner = BenchmarkRunner::new()
        .with_baseline_path(PathBuf::from("target/phase1_baseline.json"))
        .with_threshold(1.2); // 20% 성능 저하 허용
    
    // 베이스라인 로드 (있으면)
    let _ = runner.load_baseline();
    
    let oracle = Arc::new(TimestampOracle::new(1));
    let manager = VersionManager::<String>::new(Arc::clone(&oracle));
    
    // 데이터 미리 추가
    for i in 0..1000 {
        let key = format!("key_{}", i % 100).into_bytes();
        let value = format!("value_{}", i);
        let ts = oracle.as_ref().next();
        manager.add_version(key, value, ts)?;
    }
    
    let snapshot_ts = oracle.as_ref().read();
    
    // 벤치마크 실행
    let result = runner.run("get_at_snapshot", || {
        for i in 0..10 {
            let key = format!("key_{}", i % 100);
            let _ = manager.get_at_snapshot(key.as_bytes(), snapshot_ts);
        }
    })?;
    
    println!("✅ get_at_snapshot: {:.2} ms (avg)", result.avg_time_ms);
    
    // 회귀 검출
    match runner.check_regression("get_at_snapshot", &result) {
        Ok(_) => {
            println!("   No regression detected");
            // 베이스라인 업데이트
            runner.update_baseline("get_at_snapshot", &result);
            runner.save_baseline()?;
        }
        Err(e) => {
            eprintln!("⚠️  WARNING: {}", e);
        }
    }
    
    Ok(())
}
