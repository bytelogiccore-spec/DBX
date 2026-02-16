//! Integration tests for SQL Trigger and Procedure parsers

use dbx_core::automation::{
    TriggerOperation, TriggerTiming, parse_create_procedure, parse_create_trigger,
    parse_drop_procedure, parse_drop_trigger,
};

#[test]
fn test_parse_create_trigger() {
    let sql = r#"
        CREATE TRIGGER audit_trigger
        AFTER INSERT ON users
        FOR EACH ROW
        WHEN (NEW.age > 18)
        BEGIN
            INSERT INTO audit_logs VALUES (NEW.id, 'INSERT');
        END;
    "#;

    let trigger = parse_create_trigger(sql).unwrap();
    assert_eq!(trigger.name, "audit_trigger");
    assert_eq!(trigger.timing, TriggerTiming::After);
    assert_eq!(trigger.operation, TriggerOperation::Insert);
    assert_eq!(trigger.table, "users");
    assert!(trigger.condition.is_some());
}

#[test]
fn test_parse_drop_trigger() {
    let sql = "DROP TRIGGER audit_trigger;";
    let name = parse_drop_trigger(sql).unwrap();
    assert_eq!(name, "audit_trigger");
}

#[test]
fn test_parse_create_procedure() {
    let sql = r#"
        CREATE PROCEDURE update_balance (user_id INT, amount DECIMAL)
        BEGIN
            UPDATE accounts SET balance = balance + amount WHERE id = user_id;
            INSERT INTO transactions VALUES (user_id, amount);
        END;
    "#;

    let proc = parse_create_procedure(sql).unwrap();
    assert_eq!(proc.name, "update_balance");
    assert_eq!(proc.parameters.len(), 2);
    assert_eq!(proc.body.len(), 2);
}

#[test]
fn test_parse_drop_procedure() {
    let sql = "DROP PROCEDURE update_balance;";
    let name = parse_drop_procedure(sql).unwrap();
    assert_eq!(name, "update_balance");
}

#[test]
fn test_parse_create_trigger_before_update() {
    let sql = r#"
        CREATE TRIGGER validate_price
        BEFORE UPDATE ON products
        FOR EACH ROW
        BEGIN
            UPDATE validation_log SET count = count + 1;
        END;
    "#;

    let trigger = parse_create_trigger(sql).unwrap();
    assert_eq!(trigger.name, "validate_price");
    assert_eq!(trigger.timing, TriggerTiming::Before);
    assert_eq!(trigger.operation, TriggerOperation::Update);
    assert_eq!(trigger.table, "products");
}

#[test]
fn test_parse_create_procedure_no_params() {
    let sql = r#"
        CREATE PROCEDURE reset_stats ()
        BEGIN
            UPDATE stats SET count = 0;
            DELETE FROM temp_data;
        END;
    "#;

    let proc = parse_create_procedure(sql).unwrap();
    assert_eq!(proc.name, "reset_stats");
    assert_eq!(proc.parameters.len(), 0);
    assert_eq!(proc.body.len(), 2);
}
