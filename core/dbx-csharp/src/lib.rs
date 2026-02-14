//! DBX CsBindgen Native C# Bindings
//!
//! High-performance native C# bindings using CsBindgen.

use dbx_core::Database as CoreDatabase;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;

/// Opaque handle to a DBX database instance
pub enum DbxHandle {}

/// Opaque handle to a DBX transaction
pub enum DbxTransaction {}

// Internal structures (not exposed to C#)
struct DbxHandleInternal {
    db: CoreDatabase,
}

struct DbxTransactionInternal {
    db_handle: *mut DbxHandleInternal,
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

/// Open an in-memory database
#[no_mangle]
pub extern "C" fn dbx_open_in_memory() -> *mut DbxHandle {
    match CoreDatabase::open_in_memory() {
        Ok(db) => Box::into_raw(Box::new(DbxHandleInternal { db })) as *mut DbxHandle,
        Err(_) => ptr::null_mut(),
    }
}

/// Open a database at the given path
#[no_mangle]
pub unsafe extern "C" fn dbx_open(path: *const c_char) -> *mut DbxHandle {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match CoreDatabase::open(std::path::Path::new(path_str)) {
        Ok(db) => Box::into_raw(Box::new(DbxHandleInternal { db })) as *mut DbxHandle,
        Err(_) => ptr::null_mut(),
    }
}

/// Insert a key-value pair into a table
#[no_mangle]
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

    let handle = &*(handle as *const DbxHandleInternal);

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
#[no_mangle]
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

    let handle = &*(handle as *const DbxHandleInternal);

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
#[no_mangle]
pub unsafe extern "C" fn dbx_delete(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
) -> c_int {
    if handle.is_null() || table.is_null() || key.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

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

/// Begin a transaction
#[no_mangle]
pub unsafe extern "C" fn dbx_begin_transaction(handle: *mut DbxHandle) -> *mut DbxTransaction {
    if handle.is_null() {
        return ptr::null_mut();
    }

    Box::into_raw(Box::new(DbxTransactionInternal {
        db_handle: handle as *mut DbxHandleInternal,
        operations: Vec::new(),
    })) as *mut DbxTransaction
}

/// Insert a key-value pair in a transaction (buffered)
#[no_mangle]
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

    let tx = &mut *(tx as *mut DbxTransactionInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_vec = slice::from_raw_parts(key, key_len).to_vec();
    let value_vec = slice::from_raw_parts(value, value_len).to_vec();

    tx.operations.push(TxOperation::Insert {
        table: table_str,
        key: key_vec,
        value: value_vec,
    });

    DBX_OK
}

/// Delete a key in a transaction (buffered)
#[no_mangle]
pub unsafe extern "C" fn dbx_transaction_delete(
    tx: *mut DbxTransaction,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
) -> c_int {
    if tx.is_null() || table.is_null() || key.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let tx = &mut *(tx as *mut DbxTransactionInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let key_vec = slice::from_raw_parts(key, key_len).to_vec();

    tx.operations.push(TxOperation::Delete {
        table: table_str,
        key: key_vec,
    });

    DBX_OK
}

/// Commit a transaction
#[no_mangle]
pub unsafe extern "C" fn dbx_transaction_commit(tx: *mut DbxTransaction) -> c_int {
    if tx.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let tx = Box::from_raw(tx as *mut DbxTransactionInternal);
    let db_handle = &*tx.db_handle;

    // Group operations by table for batch processing
    let mut insert_batches: std::collections::HashMap<String, Vec<(Vec<u8>, Vec<u8>)>> =
        std::collections::HashMap::new();
    let mut delete_ops: Vec<(String, Vec<u8>)> = Vec::new();

    for op in tx.operations {
        match op {
            TxOperation::Insert { table, key, value } => {
                insert_batches.entry(table).or_default().push((key, value));
            }
            TxOperation::Delete { table, key } => {
                delete_ops.push((table, key));
            }
        }
    }

    // Apply batch inserts
    for (table, rows) in insert_batches {
        if let Err(_) = db_handle.db.insert_batch(&table, rows) {
            return DBX_ERR_DATABASE;
        }
    }

    // Apply deletes
    for (table, key) in delete_ops {
        if let Err(_) = db_handle.db.delete(&table, &key) {
            return DBX_ERR_DATABASE;
        }
    }

    DBX_OK
}

/// Free a value returned by dbx_get
#[no_mangle]
pub unsafe extern "C" fn dbx_free_value(value: *mut u8, len: usize) {
    if !value.is_null() {
        let _ = Box::from_raw(slice::from_raw_parts_mut(value, len));
    }
}

/// Close a database
#[no_mangle]
pub unsafe extern "C" fn dbx_close(handle: *mut DbxHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DbxHandleInternal);
    }
}
