// 병렬 SQL 파서 최적화 벤치마크
//
// 1단계: 복잡한 쿼리 테스트

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use dbx_core::sql::ParallelSqlParser;

// ═══════════════════════════════════════════════════════════════════════════
// 복잡한 쿼리 정의
// ═══════════════════════════════════════════════════════════════════════════

const SIMPLE_SELECT: &str = "SELECT * FROM users WHERE id = 1";

const COMPLEX_JOIN: &str = "
    SELECT u.id, u.name, o.order_id, o.total, p.product_name, c.category_name
    FROM users u
    INNER JOIN orders o ON u.id = o.user_id
    INNER JOIN order_items oi ON o.order_id = oi.order_id
    INNER JOIN products p ON oi.product_id = p.id
    INNER JOIN categories c ON p.category_id = c.id
    WHERE u.created_at > '2024-01-01' 
      AND o.status = 'completed'
      AND o.total > 100
    ORDER BY o.total DESC 
    LIMIT 100
";

const COMPLEX_WITH_CTE: &str = "
    WITH sales_summary AS (
        SELECT 
            product_id, 
            SUM(amount) as total_sales,
            COUNT(*) as order_count,
            AVG(amount) as avg_sale
        FROM sales 
        WHERE date > '2024-01-01' AND status = 'completed'
        GROUP BY product_id
        HAVING SUM(amount) > 1000
    ),
    top_products AS (
        SELECT 
            product_id,
            total_sales,
            RANK() OVER (ORDER BY total_sales DESC) as sales_rank
        FROM sales_summary
    )
    SELECT 
        p.name, 
        p.category,
        tp.total_sales,
        tp.sales_rank,
        ss.order_count,
        ss.avg_sale
    FROM products p
    JOIN top_products tp ON p.id = tp.product_id
    JOIN sales_summary ss ON p.id = ss.product_id
    WHERE tp.sales_rank <= 10
    ORDER BY tp.sales_rank
";

const COMPLEX_UNION: &str = "
    SELECT id, name, email, 'active' as status, created_at
    FROM customers 
    WHERE status = 'active' AND last_login > '2024-01-01'
    UNION ALL
    SELECT id, name, email, 'inactive' as status, created_at
    FROM customers 
    WHERE status = 'inactive' AND created_at > '2023-01-01'
    UNION ALL
    SELECT id, name, email, 'archived' as status, archived_date as created_at
    FROM archived_customers 
    WHERE archived_date > '2022-01-01'
    ORDER BY created_at DESC
    LIMIT 1000
";

const COMPLEX_NESTED_SUBQUERY: &str = "
    SELECT 
        o.*,
        (SELECT COUNT(*) FROM order_items WHERE order_id = o.id) as item_count,
        (SELECT SUM(amount) FROM payments WHERE order_id = o.id) as total_paid
    FROM orders o
    WHERE o.user_id IN (
        SELECT id FROM users 
        WHERE country IN (
            SELECT code FROM countries 
            WHERE region = 'APAC' AND gdp > 1000000000
        )
        AND status = 'active'
        AND created_at > '2023-01-01'
    )
    AND o.status IN ('pending', 'processing', 'completed')
    AND o.total > (
        SELECT AVG(total) FROM orders 
        WHERE created_at > '2024-01-01'
    )
    ORDER BY o.created_at DESC
";

// ═══════════════════════════════════════════════════════════════════════════
// 벤치마크: 단일 쿼리 파싱 (복잡도별)
// ═══════════════════════════════════════════════════════════════════════════

fn bench_single_query_by_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_query_complexity");
    let parser = ParallelSqlParser::new();

    group.bench_function("simple_select", |b| {
        b.iter(|| parser.parse(black_box(SIMPLE_SELECT)).unwrap())
    });

    group.bench_function("complex_join", |b| {
        b.iter(|| parser.parse(black_box(COMPLEX_JOIN)).unwrap())
    });

    group.bench_function("complex_with_cte", |b| {
        b.iter(|| parser.parse(black_box(COMPLEX_WITH_CTE)).unwrap())
    });

    group.bench_function("complex_union", |b| {
        b.iter(|| parser.parse(black_box(COMPLEX_UNION)).unwrap())
    });

    group.bench_function("complex_nested_subquery", |b| {
        b.iter(|| parser.parse(black_box(COMPLEX_NESTED_SUBQUERY)).unwrap())
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 벤치마크: 배치 파싱 (복잡한 쿼리)
// ═══════════════════════════════════════════════════════════════════════════

fn bench_batch_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_complex_queries");
    let parser = ParallelSqlParser::new();

    // 100개 복잡한 JOIN 쿼리
    let complex_joins: Vec<String> = (0..100)
        .map(|i| COMPLEX_JOIN.replace("users", &format!("users_{}", i % 10)))
        .collect();
    let complex_joins_refs: Vec<&str> = complex_joins.iter().map(|s| s.as_str()).collect();

    group.bench_function("batch_100_complex_joins", |b| {
        b.iter(|| parser.parse_batch(black_box(&complex_joins_refs)).unwrap())
    });

    // 1000개 복잡한 JOIN 쿼리
    let complex_joins_1000: Vec<String> = (0..1000)
        .map(|i| COMPLEX_JOIN.replace("users", &format!("users_{}", i % 100)))
        .collect();
    let complex_joins_1000_refs: Vec<&str> =
        complex_joins_1000.iter().map(|s| s.as_str()).collect();

    group.bench_function("batch_1000_complex_joins", |b| {
        b.iter(|| {
            parser
                .parse_batch(black_box(&complex_joins_1000_refs))
                .unwrap()
        })
    });

    // 10000개 복잡한 JOIN 쿼리
    let complex_joins_10000: Vec<String> = (0..10000)
        .map(|i| COMPLEX_JOIN.replace("users", &format!("users_{}", i % 1000)))
        .collect();
    let complex_joins_10000_refs: Vec<&str> =
        complex_joins_10000.iter().map(|s| s.as_str()).collect();

    group.bench_function("batch_10000_complex_joins", |b| {
        b.iter(|| {
            parser
                .parse_batch(black_box(&complex_joins_10000_refs))
                .unwrap()
        })
    });

    group.finish();
}

// ═══════════════════════════════════════════════════════════════════════════
// 벤치마크: 혼합 복잡도 배치
// ═══════════════════════════════════════════════════════════════════════════

fn bench_batch_mixed_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_mixed_complexity");
    let parser = ParallelSqlParser::new();

    // 1000개 혼합 쿼리 (20% 단순, 80% 복잡)
    let mut mixed_queries = Vec::new();
    for i in 0..1000 {
        if i % 5 == 0 {
            // 20% 단순 쿼리
            mixed_queries.push(format!("SELECT * FROM table_{} WHERE id = {}", i % 10, i));
        } else {
            // 80% 복잡 쿼리
            mixed_queries.push(COMPLEX_JOIN.replace("users", &format!("users_{}", i % 100)));
        }
    }
    let mixed_refs: Vec<&str> = mixed_queries.iter().map(|s| s.as_str()).collect();

    group.bench_function("batch_1000_mixed", |b| {
        b.iter(|| parser.parse_batch(black_box(&mixed_refs)).unwrap())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_query_by_complexity,
    bench_batch_complex_queries,
    bench_batch_mixed_complexity
);
criterion_main!(benches);
