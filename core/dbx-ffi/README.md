# dbx-ffi

[![Crates.io](https://img.shields.io/crates/v/dbx-ffi.svg)](https://crates.io/crates/dbx-ffi)

C FFI bindings for the DBX embedded database engine.

This crate provides a C-compatible interface (`cdylib` + `staticlib`) to the `dbx-core` engine, enabling integration with C, C++, C#, Python, and Node.js.

## Exported Functions

| Function | Description |
|----------|-------------|
| `dbx_open(path)` | Open a file-based database |
| `dbx_open_in_memory()` | Open an in-memory database |
| `dbx_insert(db, table, key, value)` | Insert a key-value pair |
| `dbx_get(db, table, key)` | Get value by key |
| `dbx_delete(db, table, key)` | Delete a key |
| `dbx_close(db)` | Close and free resources |
| `dbx_begin_transaction(db)` | Start a transaction |
| `dbx_transaction_commit(tx)` | Commit a transaction |

## Building

```bash
cargo build --release -p dbx-ffi
```

Produces `dbx_ffi.dll` (Windows) / `libdbx_ffi.so` (Linux) / `libdbx_ffi.dylib` (macOS).

## License

MIT License
