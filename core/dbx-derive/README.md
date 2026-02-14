# dbx-derive

[![Crates.io](https://img.shields.io/crates/v/dbx-derive.svg)](https://crates.io/crates/dbx-derive)
[![docs.rs](https://docs.rs/dbx-derive/badge.svg)](https://docs.rs/dbx-derive)

Procedural macros for the DBX database engine.

## Usage

```rust
use dbx_derive::Table;

#[derive(Table)]
#[dbx(table_name = "users")]
pub struct User {
    #[dbx(primary_key)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}
```

This generates:
- `User::TABLE_NAME` — table name constant
- `User::schema()` — Arrow Schema definition
- `FromRow` trait implementation

## License

MIT License
