//! Schema Builder API implementation for Database

use crate::engine::Database;
use crate::engine::schema_builder::SchemaBuilder;
use crate::DbxResult;
use arrow::datatypes::Schema;

impl Database {
    /// Create a table using SchemaBuilder
    ///
    /// This method provides a convenient way to create tables using the fluent
    /// SchemaBuilder API without manually constructing Arrow schemas.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// db.create_table_with_builder("users", |builder| {
    ///     builder
    ///         .id("id")
    ///         .text("name")
    ///         .text("email")
    ///         .int32("age").nullable()
    /// })?;
    ///
    /// assert!(db.table_exists("users"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_table_with_builder<F>(&self, name: &str, builder_fn: F) -> DbxResult<()>
    where
        F: FnOnce(SchemaBuilder) -> SchemaBuilder,
    {
        let schema = builder_fn(SchemaBuilder::new()).build();
        self.create_table(name, schema)
    }
}
