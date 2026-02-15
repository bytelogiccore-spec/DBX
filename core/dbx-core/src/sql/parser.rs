use crate::error::{DbxError, DbxResult};
use sqlparser::ast::Statement;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

/// SQL 파서 — sqlparser-rs
pub struct SqlParser {
    dialect: GenericDialect,
}

impl SqlParser {
    /// 새 SQL 파서 생성
    pub fn new() -> Self {
        Self {
            dialect: GenericDialect {},
        }
    }

    /// SQL 문자열을 AST로 파싱
    pub fn parse(&self, sql: &str) -> DbxResult<Vec<Statement>> {
        Parser::parse_sql(&self.dialect, sql).map_err(|e| DbxError::SqlParse {
            message: e.to_string(),
            sql: sql.to_string(),
        })
    }
}

impl Default for SqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::ast::{SelectItem, SetExpr};

    #[test]
    fn test_parse_simple_select() {
        let parser = SqlParser::new();
        let statements = parser.parse("SELECT * FROM users").unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                if let SetExpr::Select(select) = query.body.as_ref() {
                    assert_eq!(select.projection.len(), 1);
                    assert!(matches!(select.projection[0], SelectItem::Wildcard(_)));
                }
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_select_with_where() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT id, name FROM users WHERE id = 1")
            .unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                if let SetExpr::Select(select) = query.body.as_ref() {
                    assert_eq!(select.projection.len(), 2);
                    assert!(select.selection.is_some());
                }
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_insert() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("INSERT INTO users (id, name) VALUES (1, 'Alice')")
            .unwrap();
        assert_eq!(statements.len(), 1);
        assert!(matches!(statements[0], Statement::Insert { .. }));
    }

    #[test]
    fn test_parse_update() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("UPDATE users SET name = 'Bob' WHERE id = 1")
            .unwrap();
        assert_eq!(statements.len(), 1);
        assert!(matches!(statements[0], Statement::Update { .. }));
    }

    #[test]
    fn test_parse_delete() {
        let parser = SqlParser::new();
        let statements = parser.parse("DELETE FROM users WHERE id = 1").unwrap();
        assert_eq!(statements.len(), 1);
        assert!(matches!(statements[0], Statement::Delete(_)));
    }

    #[test]
    fn test_parse_create_table() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("CREATE TABLE users (id INT PRIMARY KEY, name TEXT)")
            .unwrap();
        assert_eq!(statements.len(), 1);
        assert!(matches!(statements[0], Statement::CreateTable(_)));
    }

    #[test]
    fn test_parse_drop_table() {
        let parser = SqlParser::new();
        let statements = parser.parse("DROP TABLE users").unwrap();
        assert_eq!(statements.len(), 1);
        assert!(matches!(statements[0], Statement::Drop { .. }));
    }

    #[test]
    fn test_parse_select_with_join() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT u.id, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id")
            .unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                if let SetExpr::Select(select) = query.body.as_ref() {
                    assert_eq!(select.from.len(), 1);
                    assert!(!select.from[0].joins.is_empty());
                }
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_select_with_group_by() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT category, COUNT(*) FROM products GROUP BY category")
            .unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                if let SetExpr::Select(select) = query.body.as_ref() {
                    match &select.group_by {
                        sqlparser::ast::GroupByExpr::Expressions(exprs, _) => {
                            assert!(!exprs.is_empty());
                        }
                        sqlparser::ast::GroupByExpr::All(_) => {}
                    }
                }
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_select_with_order_by() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT * FROM users ORDER BY name DESC")
            .unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                assert!(query.order_by.is_some());
                if let Some(order_by) = &query.order_by {
                    assert!(!order_by.exprs.is_empty());
                }
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_select_with_limit() {
        let parser = SqlParser::new();
        let statements = parser.parse("SELECT * FROM users LIMIT 10").unwrap();
        assert_eq!(statements.len(), 1);

        match &statements[0] {
            Statement::Query(query) => {
                assert!(query.limit.is_some());
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_multiple_statements() {
        let parser = SqlParser::new();
        let statements = parser
            .parse("SELECT * FROM users; SELECT * FROM orders;")
            .unwrap();
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_parse_invalid_sql() {
        let parser = SqlParser::new();
        let result = parser.parse("SELECT * FROM");
        assert!(result.is_err());
    }
}
