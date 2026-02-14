// Test to check sqlparser Statement::Update and Statement::Delete structure
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    
    // Test UPDATE
    let update_sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
    let update_ast = Parser::parse_sql(&dialect, update_sql).unwrap();
    println!("UPDATE statement: {:#?}", update_ast[0]);
    
    // Test DELETE
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    println!("\nDELETE statement: {:#?}", delete_ast[0]);
}
