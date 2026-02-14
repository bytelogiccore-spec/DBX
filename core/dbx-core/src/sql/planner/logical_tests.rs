//! LogicalPlanner INSERT tests

#[cfg(test)]
mod tests {
    use crate::sql::planner::logical::LogicalPlanner;
    use crate::sql::planner::types::{Expr, LogicalPlan};
    use crate::sql::SqlParser;
    use crate::storage::columnar::ScalarValue;

    #[test]
    fn test_insert_single_row() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice')";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert {
                table,
                columns,
                values,
            } => {
                assert_eq!(table, "users");
                assert_eq!(columns, vec!["id", "name"]);
                assert_eq!(values.len(), 1);
                assert_eq!(values[0].len(), 2);

                // Check first value (id = 1)
                match &values[0][0] {
                    Expr::Literal(ScalarValue::Int64(i)) => assert_eq!(*i, 1),
                    _ => panic!("Expected Int64 literal for id"),
                }

                // Check second value (name = 'Alice')
                match &values[0][1] {
                    Expr::Literal(ScalarValue::Utf8(s)) => assert_eq!(s, "Alice"),
                    _ => panic!("Expected Utf8 literal for name"),
                }
            }
            _ => panic!("Expected INSERT plan"),
        }
    }

    #[test]
    fn test_insert_multiple_rows() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert {
                table,
                columns,
                values,
            } => {
                assert_eq!(table, "users");
                assert_eq!(columns, vec!["id", "name"]);
                assert_eq!(values.len(), 3);

                // Check first row
                match &values[0][0] {
                    Expr::Literal(ScalarValue::Int64(i)) => assert_eq!(*i, 1),
                    _ => panic!("Expected Int64 literal"),
                }
                match &values[0][1] {
                    Expr::Literal(ScalarValue::Utf8(s)) => assert_eq!(s, "Alice"),
                    _ => panic!("Expected Utf8 literal"),
                }

                // Check second row
                match &values[1][0] {
                    Expr::Literal(ScalarValue::Int64(i)) => assert_eq!(*i, 2),
                    _ => panic!("Expected Int64 literal"),
                }
                match &values[1][1] {
                    Expr::Literal(ScalarValue::Utf8(s)) => assert_eq!(s, "Bob"),
                    _ => panic!("Expected Utf8 literal"),
                }

                // Check third row
                match &values[2][0] {
                    Expr::Literal(ScalarValue::Int64(i)) => assert_eq!(*i, 3),
                    _ => panic!("Expected Int64 literal"),
                }
                match &values[2][1] {
                    Expr::Literal(ScalarValue::Utf8(s)) => assert_eq!(s, "Charlie"),
                    _ => panic!("Expected Utf8 literal"),
                }
            }
            _ => panic!("Expected INSERT plan"),
        }
    }

    #[test]
    fn test_insert_different_types() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO data (id, value, flag) VALUES (42, 3.14, true)";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert { values, .. } => {
                assert_eq!(values.len(), 1);
                assert_eq!(values[0].len(), 3);

                // Check Int64
                match &values[0][0] {
                    Expr::Literal(ScalarValue::Int64(i)) => assert_eq!(*i, 42),
                    _ => panic!("Expected Int64"),
                }

                // Check Float64
                match &values[0][1] {
                    Expr::Literal(ScalarValue::Float64(f)) => assert_eq!(*f, 3.14),
                    _ => panic!("Expected Float64"),
                }

                // Check Boolean
                match &values[0][2] {
                    Expr::Literal(ScalarValue::Boolean(b)) => assert_eq!(*b, true),
                    _ => panic!("Expected Boolean"),
                }
            }
            _ => panic!("Expected INSERT plan"),
        }
    }

    #[test]
    fn test_insert_without_columns() {
        let parser = SqlParser::new();
        let sql = "INSERT INTO users VALUES (1, 'Alice')";
        let statements = parser.parse(sql).unwrap();

        let planner = LogicalPlanner::new();
        let plan = planner.plan(&statements[0]).unwrap();

        match plan {
            LogicalPlan::Insert { columns, .. } => {
                // When no columns specified, columns should be empty
                assert!(columns.is_empty());
            }
            _ => panic!("Expected INSERT plan"),
        }
    }
}
