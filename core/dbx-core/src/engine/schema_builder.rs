//! Schema Builder - Fluent API for building Arrow Schemas
//!
//! This module provides a type-safe, fluent API for building Arrow schemas
//! without manually constructing Field objects.

use arrow::datatypes::{DataType, Field, Schema, TimeUnit};

/// Schema Builder for constructing Arrow schemas using a fluent API
///
/// # Example
///
/// ```rust
/// use dbx_core::SchemaBuilder;
/// use arrow::datatypes::DataType;
///
/// let schema = SchemaBuilder::new()
///     .id("id")
///     .text("name")
///     .int32("age").nullable()
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SchemaBuilder {
    fields: Vec<Field>,
}

impl SchemaBuilder {
    /// Create a new SchemaBuilder
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let builder = SchemaBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
        }
    }

    /// Add a column with explicit type and nullability
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    /// use arrow::datatypes::DataType;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .column("id", DataType::Int64, false)
    ///     .column("name", DataType::Utf8, true)
    ///     .build();
    /// ```
    pub fn column(mut self, name: &str, data_type: DataType, nullable: bool) -> Self {
        self.fields.push(Field::new(name, data_type, nullable));
        self
    }

    /// Build the final Schema
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .id("id")
    ///     .text("name")
    ///     .build();
    /// ```
    pub fn build(self) -> Schema {
        Schema::new(self.fields)
    }

    // ========== Type-specific convenience methods ==========

    /// Add an ID column (Int64, NOT NULL)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .id("id")
    ///     .build();
    /// ```
    pub fn id(self, name: &str) -> Self {
        self.column(name, DataType::Int64, false)
    }

    /// Add a text column (Utf8, nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .text("name")
    ///     .build();
    /// ```
    pub fn text(self, name: &str) -> Self {
        self.column(name, DataType::Utf8, true)
    }

    /// Add an Int32 column (nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .int32("age")
    ///     .build();
    /// ```
    pub fn int32(self, name: &str) -> Self {
        self.column(name, DataType::Int32, true)
    }

    /// Add an Int64 column (nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .int64("user_id")
    ///     .build();
    /// ```
    pub fn int64(self, name: &str) -> Self {
        self.column(name, DataType::Int64, true)
    }

    /// Add a Float64 column (nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .float64("salary")
    ///     .build();
    /// ```
    pub fn float64(self, name: &str) -> Self {
        self.column(name, DataType::Float64, true)
    }

    /// Add a Boolean column (nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .boolean("is_active")
    ///     .build();
    /// ```
    pub fn boolean(self, name: &str) -> Self {
        self.column(name, DataType::Boolean, true)
    }

    /// Add a Timestamp column (nullable by default)
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .timestamp("created_at")
    ///     .build();
    /// ```
    pub fn timestamp(self, name: &str) -> Self {
        self.column(name, DataType::Timestamp(TimeUnit::Millisecond, None), true)
    }

    // ========== Nullability control methods ==========

    /// Make the last added field nullable
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .int32("age").nullable()
    ///     .build();
    /// ```
    pub fn nullable(mut self) -> Self {
        if let Some(field) = self.fields.last_mut() {
            *field = Field::new(field.name(), field.data_type().clone(), true);
        }
        self
    }

    /// Make the last added field NOT NULL
    ///
    /// # Example
    ///
    /// ```rust
    /// use dbx_core::SchemaBuilder;
    ///
    /// let schema = SchemaBuilder::new()
    ///     .text("email").not_null()
    ///     .build();
    /// ```
    pub fn not_null(mut self) -> Self {
        if let Some(field) = self.fields.last_mut() {
            *field = Field::new(field.name(), field.data_type().clone(), false);
        }
        self
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
