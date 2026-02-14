#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::planner::types::{Expr, LogicalPlan};

    #[test]
    fn test_update_parsing() {
        let planner = LogicalPlanner::new();
        let parser = SqlParser::new();
        
        // Simple UPDATE
        let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
        let statements = parser.parse(sql).unwrap();
        let plan = planner.plan(&statements[0]).unwrap();
        
        match plan {
            LogicalPlan::Update { table, assignments, filter } => {
                assert_eq!(table, "users");
                assert_eq!(assignments.len(), 1);
                assert_eq!(assignments[0].0, "name");
                assert!(filter.is_some());
            }
            _ => panic!("Expected UPDATE plan"),
        }
    }

    #[test]
    fn test_update_multiple_columns() {
        let planner = LogicalPlanner::new();
        let parser = SqlParser::new();
        
        let sql = "UPDATE users SET name = 'Bob', age = 30";
        let statements = parser.parse(sql).unwrap();
        let plan = planner.plan(&statements[0]).unwrap();
        
        match plan {
            LogicalPlan::Update { table, assignments, filter } => {
                assert_eq!(table, "users");
                assert_eq!(assignments.len(), 2);
                assert!(filter.is_none());
            }
            _ => panic!("Expected UPDATE plan"),
        }
    }

    #[test]
    fn test_delete_parsing() {
        let planner = LogicalPlanner::new();
        let parser = SqlParser::new();
        
        // Simple DELETE
        let sql = "DELETE FROM users WHERE id = 1";
        let statements = parser.parse(sql).unwrap();
        let plan = planner.plan(&statements[0]).unwrap();
        
        match plan {
            LogicalPlan::Delete { table, filter } => {
                assert_eq!(table, "users");
                assert!(filter.is_some());
            }
            _ => panic!("Expected DELETE plan"),
        }
    }

    #[test]
    fn test_delete_without_where() {
        let planner = LogicalPlanner::new();
        let parser = SqlParser::new();
        
        let sql = "DELETE FROM users";
        let statements = parser.parse(sql).unwrap();
        let plan = planner.plan(&statements[0]).unwrap();
        
        match plan {
            LogicalPlan::Delete { table, filter } => {
                assert_eq!(table, "users");
                assert!(filter.is_none());
            }
            _ => panic!("Expected DELETE plan"),
        }
    }
}
