//! derive(Table) 매크로 테스트

use dbx_derive::Table;

#[derive(Table)]
#[dbx(table_name = "users")]
pub struct User {
    pub id: i64,
    pub name: String,
    pub age: i32,
    pub email: Option<String>,
}

#[test]
fn test_table_name() {
    assert_eq!(User::TABLE_NAME, "users");
}

#[test]
fn test_schema() {
    let schema = User::schema();
    assert_eq!(schema.fields().len(), 4);
    assert_eq!(schema.field(0).name(), "id");
    assert_eq!(schema.field(1).name(), "name");
    assert_eq!(schema.field(2).name(), "age");
    assert_eq!(schema.field(3).name(), "email");
}
