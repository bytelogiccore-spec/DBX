// Query Builder API implementation for Database

use crate::engine::Database;
use crate::engine::query_builder::QueryBuilder;

impl Database {
    /// Create a new QueryBuilder for building SQL queries
    ///
    /// This method returns a QueryBuilder that allows you to construct
    /// SQL queries using a fluent API without writing raw SQL strings.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::Database;
    /// use arrow::datatypes::{DataType, Field, Schema};
    ///
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    ///
    /// // Create a table first
    /// let schema = Schema::new(vec![
    ///     Field::new("id", DataType::Int64, false),
    ///     Field::new("name", DataType::Utf8, true),
    ///     Field::new("age", DataType::Int32, true),
    /// ]);
    /// db.create_table("users", schema)?;
    ///
    /// // Query using the builder
    /// let results = db.query_builder()
    ///     .select(&["id", "name"])
    ///     .from("users")
    ///     .where_("age", ">", "18")
    ///     .order_by("name", "ASC")
    ///     .limit(10)
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_builder(&self) -> QueryBuilder<'_> {
        QueryBuilder::new(self)
    }
}
