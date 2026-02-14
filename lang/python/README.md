# DBX Python Bindings

Python bindings for the DBX high-performance embedded database.

## Installation

### From Source

```bash
cd bindings/python
pip install -e .
```

This will automatically build the Rust FFI library and install the Python package.

## Quick Start

```python
from dbx_py import Database

# Open an in-memory database
db = Database.open_in_memory()

# Or open a file-based database
# db = Database("my_database.db")

# Insert data
db.insert("users", b"user:1", b"Alice")
db.insert("users", b"user:2", b"Bob")

# Get data
value = db.get("users", b"user:1")
print(value.decode('utf-8'))  # Output: Alice

# Count rows
count = db.count("users")
print(f"Total users: {count}")  # Output: Total users: 2

# Delete data
db.delete("users", b"user:2")

# Flush to disk
db.flush()

# Close database
db.close()
```

## Context Manager Support

```python
with Database("my_database.db") as db:
    db.insert("users", b"user:1", b"Alice")
    value = db.get("users", b"user:1")
    print(value.decode('utf-8'))
# Database is automatically closed
```

## API Reference

### `Database(path: str)`

Open a database at the specified path.

**Parameters:**
- `path` (str): Path to the database file

**Returns:**
- `Database`: Database instance

### `Database.open_in_memory() -> Database`

Open an in-memory database.

**Returns:**
- `Database`: Database instance

### `insert(table: str, key: bytes, value: bytes) -> None`

Insert a key-value pair into a table.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key data
- `value` (bytes): Value data

**Raises:**
- `RuntimeError`: If insertion fails

### `get(table: str, key: bytes) -> Optional[bytes]`

Get a value by key from a table.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key data

**Returns:**
- `bytes` or `None`: Value data if found, None otherwise

**Raises:**
- `RuntimeError`: If operation fails

### `delete(table: str, key: bytes) -> None`

Delete a key from a table.

**Parameters:**
- `table` (str): Table name
- `key` (bytes): Key data

**Raises:**
- `RuntimeError`: If deletion fails

### `count(table: str) -> int`

Count rows in a table.

**Parameters:**
- `table` (str): Table name

**Returns:**
- `int`: Number of rows

**Raises:**
- `RuntimeError`: If operation fails

### `flush() -> None`

Flush database to disk.

**Raises:**
- `RuntimeError`: If flush fails

### `close() -> None`

Close the database and free resources.

## Examples

See the `examples/` directory for more examples:
- `basic_crud.py`: Basic CRUD operations

## Requirements

- Python 3.8+
- Rust toolchain (for building from source)

## License

Dual-licensed under MIT or Apache-2.0.
