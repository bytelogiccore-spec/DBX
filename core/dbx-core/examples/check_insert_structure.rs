// Test to check sqlparser Statement::Insert structure
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() {
    let dialect = GenericDialect {};
    let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
    let ast = Parser::parse_sql(&dialect, sql).unwrap();

    println!("Parsed statement: {:#?}", ast[0]);

    match &ast[0] {
        Statement::Insert { .. } => println!("Matched as Statement::Insert {{ .. }}"),
        Statement::Insert(insert) => println!("Matched as Statement::Insert(insert)"),
        _ => println!("Did NOT match INSERT!"),
    }
}
