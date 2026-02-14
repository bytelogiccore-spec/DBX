// Temporary test to check sqlparser::ast::Statement::Insert structure
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
    let ast = Parser::parse_sql(&dialect, sql).unwrap();
    
    match &ast[0] {
        Statement::Insert(insert) => {
            println!("Insert structure: {:#?}", insert);
        }
        _ => println!("Not an insert"),
    }
}
