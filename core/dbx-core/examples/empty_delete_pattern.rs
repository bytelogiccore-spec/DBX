// Empty pattern to let compiler tell us the fields
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    let delete_sql = "DELETE FROM users WHERE id = 1";
    let delete_ast = Parser::parse_sql(&dialect, delete_sql).unwrap();
    
    // Empty pattern - compiler will tell us what fields exist
    match &delete_ast[0] {
        Statement::Delete { .. } => {
            println!("DELETE matched");
        }
        _ => println!("Not DELETE"),
    }
}
