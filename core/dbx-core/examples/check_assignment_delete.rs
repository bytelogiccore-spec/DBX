// Check Assignment and Delete structure
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    
    // Test UPDATE - focus on Assignment structure
    println!("=== UPDATE Assignment Structure ===");
    let update_sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
    let update_ast = Parser::parse_sql(&dialect, update_sql).unwrap();
    
    if let Statement::Update { assignments, .. } = &update_ast[0] {
        println!("Assignment: {:#?}", assignments[0]);
    }
    
    // Test DELETE - focus on field names
    println!("\n=== DELETE Field Names ===");
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    
    if let Statement::Delete { .. } = &delete_ast[0] {
        println!("DELETE statement fields:");
        println!("{:#?}", delete_ast[0]);
    }
}
