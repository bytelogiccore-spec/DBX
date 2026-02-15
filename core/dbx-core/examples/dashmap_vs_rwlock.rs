// 멀티스레드 DashMap vs RwLock 성능 비교
//
// 1, 2, 4, 8 스레드에서 동시 스키마 조회 처리량(ops/sec)을 비교
// Criterion은 싱글스레드이므로 직접 측정

use arrow::datatypes::{DataType, Field, Schema};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

const OPS_PER_THREAD: usize = 1_000_000;
const TABLES: usize = 50;

fn make_schema(n: usize) -> Arc<Schema> {
    Arc::new(Schema::new(
        (0..n)
            .map(|i| Field::new(format!("col_{i}"), DataType::Int64, true))
            .collect::<Vec<_>>(),
    ))
}

fn bench_rwlock(threads: usize) -> Duration {
    let map: Arc<RwLock<HashMap<String, Arc<Schema>>>> = Arc::new(RwLock::new(HashMap::new()));
    for i in 0..TABLES {
        map.write()
            .unwrap()
            .insert(format!("table_{i}"), make_schema(3));
    }

    let start = Instant::now();
    std::thread::scope(|s| {
        for t in 0..threads {
            let map = Arc::clone(&map);
            s.spawn(move || {
                for i in 0..OPS_PER_THREAD {
                    let key = format!("table_{}", (t + i) % TABLES);
                    let _schema = map.read().unwrap().get(&key).cloned();
                }
            });
        }
    });
    start.elapsed()
}

fn bench_dashmap(threads: usize) -> Duration {
    let map: Arc<DashMap<String, Arc<Schema>>> = Arc::new(DashMap::new());
    for i in 0..TABLES {
        map.insert(format!("table_{i}"), make_schema(3));
    }

    let start = Instant::now();
    std::thread::scope(|s| {
        for t in 0..threads {
            let map = Arc::clone(&map);
            s.spawn(move || {
                for i in 0..OPS_PER_THREAD {
                    let key = format!("table_{}", (t + i) % TABLES);
                    let _schema = map.get(&key).map(|r| r.value().clone());
                }
            });
        }
    });
    start.elapsed()
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   멀티스레드 스키마 조회: RwLock<HashMap> vs DashMap        ║");
    println!(
        "║   각 스레드당 {} ops                           ║",
        OPS_PER_THREAD
    );
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Threads │ RwLock        │ DashMap       │ 속도 향상         ║");
    println!("╠═════════╪═══════════════╪═══════════════╪═══════════════════╣");

    for threads in [1, 2, 4, 8] {
        // 워밍업
        bench_rwlock(threads);
        bench_dashmap(threads);

        // 3회 측정 평균
        let mut rwlock_total = Duration::ZERO;
        let mut dashmap_total = Duration::ZERO;
        let runs = 3;
        for _ in 0..runs {
            rwlock_total += bench_rwlock(threads);
            dashmap_total += bench_dashmap(threads);
        }
        let rwlock_avg = rwlock_total / runs;
        let dashmap_avg = dashmap_total / runs;

        let total_ops = (threads * OPS_PER_THREAD) as f64;
        let rwlock_ops = total_ops / rwlock_avg.as_secs_f64();
        let dashmap_ops = total_ops / dashmap_avg.as_secs_f64();
        let speedup = dashmap_ops / rwlock_ops;

        println!(
            "║   {:>2}    │ {:>9.2} Mops│ {:>9.2} Mops│ {:>6.2}x {:>10} ║",
            threads,
            rwlock_ops / 1_000_000.0,
            dashmap_ops / 1_000_000.0,
            speedup,
            if speedup > 1.5 {
                "🔥"
            } else if speedup > 1.0 {
                "✅"
            } else {
                "⚠️"
            }
        );
    }
    println!("╚══════════════════════════════════════════════════════════════╝");
}
