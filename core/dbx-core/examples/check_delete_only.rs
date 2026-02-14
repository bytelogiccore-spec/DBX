// Check DELETE statement structure in detail
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    
    // Test DELETE
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    
    // Print all fields
    match &delete_ast[0] {
        Statement::Delete { .. } => {
            println!("DELETE statement structure:");
            println!("{:#?}", delete_ast[0]);
        }
        _ => println!("Not a DELETE statement"),
    }
}
