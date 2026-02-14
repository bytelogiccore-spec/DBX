// Minimal DELETE test - just try to compile with different field patterns
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();

    // Try to match and see what fields exist
    match &delete_ast[0] {
        Statement::Delete {
            tables,
            using,
            selection,
            returning,
            order_by,
            limit,
            ..
        } => {
            println!("tables: {:?}", tables);
            println!("using: {:?}", using);
            println!("selection: {:?}", selection);
            println!("returning: {:?}", returning);
            println!("order_by: {:?}", order_by);
            println!("limit: {:?}", limit);
        }
        _ => println!("Not DELETE"),
    }
}
