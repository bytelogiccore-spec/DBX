// Simpler test - just print the statement type
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    
    // Test UPDATE
    println!("=== UPDATE Statement ===");
    let update_sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
    let update_ast = Parser::parse_sql(&dialect, update_sql).unwrap();
    println!("{:#?}", update_ast[0]);
    
    // Test DELETE
    println!("\n=== DELETE Statement ===");
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    println!("{:#?}", delete_ast[0]);
}
