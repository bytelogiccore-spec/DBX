// Test to check sqlparser Statement::Update and Statement::Delete structure
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    
    // Test UPDATE
    println!("=== UPDATE Statement ===");
    let update_sql = "UPDATE users SET name = 'Bob', age = 30 WHERE id = 1";
    let update_ast = Parser::parse_sql(&dialect, update_sql).unwrap();
    
    match &update_ast[0] {
        Statement::Update { table, assignments, selection, .. } => {
            println!("Table: {:?}", table);
            println!("Assignments: {:?}", assignments);
            println!("Selection (WHERE): {:?}", selection);
        }
        _ => println!("Not an UPDATE statement"),
    }
    
    // Test DELETE
    println!("\n=== DELETE Statement ===");
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    
    match &delete_ast[0] {
        Statement::Delete { tables, selection, .. } => {
            println!("Tables: {:?}", tables);
            println!("Selection (WHERE): {:?}", selection);
        }
        _ => println!("Not a DELETE statement"),
    }
}
