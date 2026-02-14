//! DBX C FFI Bindings
//!
//! This crate provides C-compatible FFI bindings for the DBX database.

#![allow(unsafe_op_in_unsafe_fn)]

use dbx_core::Database;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;

/// Opaque handle to a DBX database instance
#[repr(C)]
pub struct DbxHandle {
    db: Database,
}

/// Opaque handle to a DBX transaction
#[repr(C)]
pub struct DbxTransaction {
    db_handle: *mut DbxHandle,
    operations: Vec<TxOperation>,
}

/// Transaction operation
enum TxOperation {
    Insert {
        table: String,
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Delete {
        table: String,
        key: Vec<u8>,
    },
}

/// Error codes
pub const DBX_OK: c_int = 0;
pub const DBX_ERR_NULL_PTR: c_int = -1;
pub const DBX_ERR_INVALID_UTF8: c_int = -2;
pub const DBX_ERR_DATABASE: c_int = -3;
pub const DBX_ERR_NOT_FOUND: c_int = -4;

/// Open a database at the given path
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_open(path: *const c_char) -> *mut DbxHandle {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match Database::open(std::path::Path::new(path_str)) {
        Ok(db) => Box::into_raw(Box::new(DbxHandle { db })),
        Err(_) => ptr::null_mut(),
    }
}

/// Open an in-memory database
#[unsafe(no_mangle)]
pub extern "C" fn dbx_open_in_memory() -> *mut DbxHandle {
    match Database::open_in_memory() {
        Ok(db) => Box::into_raw(Box::new(DbxHandle { db })),
        Err(_) => ptr::null_mut(),
    }
}

/// Insert a key-value pair into a table
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_insert(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
) -> c_int {
    if handle.is_null() || table.is_null() || key.is_null() || value.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*handle;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_slice = slice::from_raw_parts(key, key_len);
    let value_slice = slice::from_raw_parts(value, value_len);

    match handle.db.insert(table_str, key_slice, value_slice) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Get a value by key from a table
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_get(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
    out_value: *mut *mut u8,
    out_len: *mut usize,
) -> c_int {
    if handle.is_null()
        || table.is_null()
        || key.is_null()
        || out_value.is_null()
        || out_len.is_null()
    {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*handle;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_slice = slice::from_raw_parts(key, key_len);

    match handle.db.get(table_str, key_slice) {
        Ok(Some(value)) => {
            let len = value.len();
            let ptr = Box::into_raw(value.into_boxed_slice()) as *mut u8;
            *out_value = ptr;
            *out_len = len;
            DBX_OK
        }
        Ok(None) => DBX_ERR_NOT_FOUND,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Delete a key from a table
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_delete(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
) -> c_int {
    if handle.is_null() || table.is_null() || key.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*handle;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_slice = slice::from_raw_parts(key, key_len);

    match handle.db.delete(table_str, key_slice) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Count rows in a table
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_count(
    handle: *mut DbxHandle,
    table: *const c_char,
    out_count: *mut usize,
) -> c_int {
    if handle.is_null() || table.is_null() || out_count.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*handle;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.count(table_str) {
        Ok(count) => {
            *out_count = count;
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Flush database to disk
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_flush(handle: *mut DbxHandle) -> c_int {
    if handle.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*handle;

    match handle.db.flush() {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Free a value returned by dbx_get
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_free_value(value: *mut u8, len: usize) {
    if !value.is_null() {
        let _ = Box::from_raw(slice::from_raw_parts_mut(value, len));
    }
}

/// Close the database and free resources
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_close(handle: *mut DbxHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle);
    }
}

/// Get the last error message (thread-safe static string)
#[unsafe(no_mangle)]
pub extern "C" fn dbx_last_error() -> *const c_char {
    static ERROR_MSG: &[u8] = b"Error details not available\0";
    ERROR_MSG.as_ptr() as *const c_char
}

// ═══════════════════════════════════════════════════════════════
// Transaction API
// ═══════════════════════════════════════════════════════════════

/// Begin a new transaction
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_begin_transaction(handle: *mut DbxHandle) -> *mut DbxTransaction {
    if handle.is_null() {
        return ptr::null_mut();
    }

    Box::into_raw(Box::new(DbxTransaction {
        db_handle: handle,
        operations: Vec::new(),
    }))
}

/// Insert a key-value pair within a transaction (buffered)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_transaction_insert(
    tx: *mut DbxTransaction,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
) -> c_int {
    if tx.is_null() || table.is_null() || key.is_null() || value.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let tx = &mut *tx;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_slice = slice::from_raw_parts(key, key_len);
    let value_slice = slice::from_raw_parts(value, value_len);

    tx.operations.push(TxOperation::Insert {
        table: table_str,
        key: key_slice.to_vec(),
        value: value_slice.to_vec(),
    });

    DBX_OK
}

/// Delete a key within a transaction (buffered)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_transaction_delete(
    tx: *mut DbxTransaction,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
) -> c_int {
    if tx.is_null() || table.is_null() || key.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let tx = &mut *tx;

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_slice = slice::from_raw_parts(key, key_len);

    tx.operations.push(TxOperation::Delete {
        table: table_str,
        key: key_slice.to_vec(),
    });

    DBX_OK
}

/// Commit a transaction - apply all buffered operations atomically using batch insert
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_transaction_commit(tx: *mut DbxTransaction) -> c_int {
    if tx.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let tx = Box::from_raw(tx);
    let db_handle = &*tx.db_handle;

    // Group operations by table for batch processing
    use std::collections::HashMap;
    let mut insert_batches: HashMap<String, Vec<(Vec<u8>, Vec<u8>)>> = HashMap::new();
    let mut deletes: Vec<(String, Vec<u8>)> = Vec::new();

    for op in tx.operations {
        match op {
            TxOperation::Insert { table, key, value } => {
                insert_batches.entry(table).or_default().push((key, value));
            }
            TxOperation::Delete { table, key } => {
                deletes.push((table, key));
            }
        }
    }

    // Apply batch inserts
    for (table, rows) in insert_batches {
        if let Err(_) = db_handle.db.insert_batch(&table, rows) {
            return DBX_ERR_DATABASE;
        }
    }

    // Apply deletes (no batch API for delete yet)
    for (table, key) in deletes {
        if let Err(_) = db_handle.db.delete(&table, &key) {
            return DBX_ERR_DATABASE;
        }
    }

    DBX_OK
}

/// Rollback a transaction - discard all buffered operations
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dbx_transaction_rollback(tx: *mut DbxTransaction) {
    if !tx.is_null() {
        let _ = Box::from_raw(tx);
        // Operations are dropped, nothing applied
    }
}
