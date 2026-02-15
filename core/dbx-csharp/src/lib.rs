//! DBX CsBindgen Native C# Bindings
//!
//! High-performance native C# bindings using CsBindgen.

#![allow(clippy::missing_safety_doc)]

use dbx_core::Database as CoreDatabase;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::ptr;
use std::slice;

/// Opaque handle to a DBX database instance
pub enum DbxHandle {}

/// Opaque handle to a DBX transaction
pub enum DbxTransaction {}

/// Opaque handle to scan results
pub enum DbxScanResult {}

/// Opaque handle to string list
pub enum DbxStringList {}

// Internal structures (not exposed to C#)
struct DbxHandleInternal {
    db: CoreDatabase,
}

struct DbxTransactionInternal {
    db_handle: *mut DbxHandleInternal,
    operations: Vec<TxOperation>,
}

struct DbxScanResultInternal {
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

struct DbxStringListInternal {
    names: Vec<String>,
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
pub const DBX_ERR_INVALID_OP: c_int = -5;

// ═══════════════════════════════════════════════════════════════
// Constructors
// ═══════════════════════════════════════════════════════════════

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

/// Load a database from a snapshot file
#[no_mangle]
pub unsafe extern "C" fn dbx_load_from_file(path: *const c_char) -> *mut DbxHandle {
    if path.is_null() {
        return ptr::null_mut();
    }

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    match CoreDatabase::load_from_file(path_str) {
        Ok(db) => Box::into_raw(Box::new(DbxHandleInternal { db })) as *mut DbxHandle,
        Err(_) => ptr::null_mut(),
    }
}

// ═══════════════════════════════════════════════════════════════
// CRUD Operations
// ═══════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════
// Batch Operations
// ═══════════════════════════════════════════════════════════════

/// Insert multiple key-value pairs at once (batch)
#[no_mangle]
pub unsafe extern "C" fn dbx_insert_batch(
    handle: *mut DbxHandle,
    table: *const c_char,
    keys: *const *const u8,
    key_lens: *const usize,
    values: *const *const u8,
    value_lens: *const usize,
    count: usize,
) -> c_int {
    if handle.is_null()
        || table.is_null()
        || keys.is_null()
        || key_lens.is_null()
        || values.is_null()
        || value_lens.is_null()
    {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let k = slice::from_raw_parts(*keys.add(i), *key_lens.add(i)).to_vec();
        let v = slice::from_raw_parts(*values.add(i), *value_lens.add(i)).to_vec();
        rows.push((k, v));
    }

    match handle.db.insert_batch(table_str, rows) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Scan all key-value pairs in a table
#[no_mangle]
pub unsafe extern "C" fn dbx_scan(
    handle: *mut DbxHandle,
    table: *const c_char,
    out_result: *mut *mut DbxScanResult,
) -> c_int {
    if handle.is_null() || table.is_null() || out_result.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.scan(table_str) {
        Ok(entries) => {
            *out_result = Box::into_raw(Box::new(DbxScanResultInternal { entries }))
                as *mut DbxScanResult;
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Scan a range of keys [start_key, end_key)
#[no_mangle]
pub unsafe extern "C" fn dbx_range(
    handle: *mut DbxHandle,
    table: *const c_char,
    start_key: *const u8,
    start_key_len: usize,
    end_key: *const u8,
    end_key_len: usize,
    out_result: *mut *mut DbxScanResult,
) -> c_int {
    if handle.is_null()
        || table.is_null()
        || start_key.is_null()
        || end_key.is_null()
        || out_result.is_null()
    {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    let start = slice::from_raw_parts(start_key, start_key_len);
    let end = slice::from_raw_parts(end_key, end_key_len);

    match handle.db.range(table_str, start, end) {
        Ok(entries) => {
            *out_result = Box::into_raw(Box::new(DbxScanResultInternal { entries }))
                as *mut DbxScanResult;
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Get the number of entries in a scan result
#[no_mangle]
pub unsafe extern "C" fn dbx_scan_result_count(result: *const DbxScanResult) -> usize {
    if result.is_null() {
        return 0;
    }
    (*(result as *const DbxScanResultInternal)).entries.len()
}

/// Get a key from a scan result by index
#[no_mangle]
pub unsafe extern "C" fn dbx_scan_result_key(
    result: *const DbxScanResult,
    index: usize,
    out_key: *mut *const u8,
    out_key_len: *mut usize,
) -> c_int {
    if result.is_null() || out_key.is_null() || out_key_len.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let result = &*(result as *const DbxScanResultInternal);
    if index >= result.entries.len() {
        return DBX_ERR_NOT_FOUND;
    }

    *out_key = result.entries[index].0.as_ptr();
    *out_key_len = result.entries[index].0.len();
    DBX_OK
}

/// Get a value from a scan result by index
#[no_mangle]
pub unsafe extern "C" fn dbx_scan_result_value(
    result: *const DbxScanResult,
    index: usize,
    out_value: *mut *const u8,
    out_value_len: *mut usize,
) -> c_int {
    if result.is_null() || out_value.is_null() || out_value_len.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let result = &*(result as *const DbxScanResultInternal);
    if index >= result.entries.len() {
        return DBX_ERR_NOT_FOUND;
    }

    *out_value = result.entries[index].1.as_ptr();
    *out_value_len = result.entries[index].1.len();
    DBX_OK
}

/// Free a scan result
#[no_mangle]
pub unsafe extern "C" fn dbx_scan_result_free(result: *mut DbxScanResult) {
    if !result.is_null() {
        drop(Box::from_raw(result as *mut DbxScanResultInternal));
    }
}

// ═══════════════════════════════════════════════════════════════
// Utility Operations
// ═══════════════════════════════════════════════════════════════

/// Count rows in a table
#[no_mangle]
pub unsafe extern "C" fn dbx_count(
    handle: *mut DbxHandle,
    table: *const c_char,
    out_count: *mut usize,
) -> c_int {
    if handle.is_null() || table.is_null() || out_count.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

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
#[no_mangle]
pub unsafe extern "C" fn dbx_flush(handle: *mut DbxHandle) -> c_int {
    if handle.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    match handle.db.flush() {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Get all table names
#[no_mangle]
pub unsafe extern "C" fn dbx_table_names(
    handle: *mut DbxHandle,
    out_list: *mut *mut DbxStringList,
) -> c_int {
    if handle.is_null() || out_list.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    match handle.db.table_names() {
        Ok(names) => {
            *out_list = Box::into_raw(Box::new(DbxStringListInternal { names }))
                as *mut DbxStringList;
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Get the number of strings in a string list
#[no_mangle]
pub unsafe extern "C" fn dbx_string_list_count(list: *const DbxStringList) -> usize {
    if list.is_null() {
        return 0;
    }
    (*(list as *const DbxStringListInternal)).names.len()
}

/// Get a string from a string list by index
#[no_mangle]
pub unsafe extern "C" fn dbx_string_list_get(
    list: *const DbxStringList,
    index: usize,
    out_str: *mut *const u8,
    out_len: *mut usize,
) -> c_int {
    if list.is_null() || out_str.is_null() || out_len.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let list = &*(list as *const DbxStringListInternal);
    if index >= list.names.len() {
        return DBX_ERR_NOT_FOUND;
    }

    *out_str = list.names[index].as_ptr();
    *out_len = list.names[index].len();
    DBX_OK
}

/// Free a string list
#[no_mangle]
pub unsafe extern "C" fn dbx_string_list_free(list: *mut DbxStringList) {
    if !list.is_null() {
        drop(Box::from_raw(list as *mut DbxStringListInternal));
    }
}

/// Run garbage collection
#[no_mangle]
pub unsafe extern "C" fn dbx_gc(
    handle: *mut DbxHandle,
    out_deleted: *mut usize,
) -> c_int {
    if handle.is_null() || out_deleted.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    match handle.db.gc() {
        Ok(deleted) => {
            *out_deleted = deleted;
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Check if the database is encrypted. Returns 1 if encrypted, 0 if not.
#[no_mangle]
pub unsafe extern "C" fn dbx_is_encrypted(handle: *mut DbxHandle) -> c_int {
    if handle.is_null() {
        return 0;
    }
    let handle = &*(handle as *const DbxHandleInternal);
    if handle.db.is_encrypted() { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════
// SQL Operations
// ═══════════════════════════════════════════════════════════════

/// Execute a SQL statement. Returns the number of affected/returned rows.
#[no_mangle]
pub unsafe extern "C" fn dbx_execute_sql(
    handle: *mut DbxHandle,
    sql: *const c_char,
    out_affected: *mut usize,
) -> c_int {
    if handle.is_null() || sql.is_null() || out_affected.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let sql_str = match CStr::from_ptr(sql).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.execute_sql(sql_str) {
        Ok(batches) => {
            *out_affected = batches.iter().map(|b| b.num_rows()).sum::<usize>();
            DBX_OK
        }
        Err(_) => DBX_ERR_DATABASE,
    }
}

// ═══════════════════════════════════════════════════════════════
// Index Operations
// ═══════════════════════════════════════════════════════════════

/// Create an index on a table column
#[no_mangle]
pub unsafe extern "C" fn dbx_create_index(
    handle: *mut DbxHandle,
    table: *const c_char,
    column: *const c_char,
) -> c_int {
    if handle.is_null() || table.is_null() || column.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };
    let column_str = match CStr::from_ptr(column).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.create_index(table_str, column_str) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Drop an index from a table column
#[no_mangle]
pub unsafe extern "C" fn dbx_drop_index(
    handle: *mut DbxHandle,
    table: *const c_char,
    column: *const c_char,
) -> c_int {
    if handle.is_null() || table.is_null() || column.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };
    let column_str = match CStr::from_ptr(column).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.drop_index(table_str, column_str) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Check if an index exists. Returns 1 if exists, 0 if not.
#[no_mangle]
pub unsafe extern "C" fn dbx_has_index(
    handle: *mut DbxHandle,
    table: *const c_char,
    column: *const c_char,
) -> c_int {
    if handle.is_null() || table.is_null() || column.is_null() {
        return 0;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let table_str = match CStr::from_ptr(table).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let column_str = match CStr::from_ptr(column).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if handle.db.has_index(table_str, column_str) { 1 } else { 0 }
}

// ═══════════════════════════════════════════════════════════════
// Snapshot Operations
// ═══════════════════════════════════════════════════════════════

/// Save the in-memory database to a file
#[no_mangle]
pub unsafe extern "C" fn dbx_save_to_file(
    handle: *mut DbxHandle,
    path: *const c_char,
) -> c_int {
    if handle.is_null() || path.is_null() {
        return DBX_ERR_NULL_PTR;
    }

    let handle = &*(handle as *const DbxHandleInternal);

    let path_str = match CStr::from_ptr(path).to_str() {
        Ok(s) => s,
        Err(_) => return DBX_ERR_INVALID_UTF8,
    };

    match handle.db.save_to_file(path_str) {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

// ═══════════════════════════════════════════════════════════════
// MVCC Operations
// ═══════════════════════════════════════════════════════════════

/// Get the current MVCC timestamp
#[no_mangle]
pub unsafe extern "C" fn dbx_current_timestamp(handle: *mut DbxHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    let handle = &*(handle as *const DbxHandleInternal);
    handle.db.current_timestamp()
}

/// Allocate a new commit timestamp
#[no_mangle]
pub unsafe extern "C" fn dbx_allocate_commit_ts(handle: *mut DbxHandle) -> u64 {
    if handle.is_null() {
        return 0;
    }
    let handle = &*(handle as *const DbxHandleInternal);
    handle.db.allocate_commit_ts()
}

/// Insert a versioned key-value pair (MVCC)
#[no_mangle]
pub unsafe extern "C" fn dbx_insert_versioned(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
    commit_ts: u64,
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

    match handle
        .db
        .insert_versioned(table_str, key_slice, Some(value_slice), commit_ts)
    {
        Ok(_) => DBX_OK,
        Err(_) => DBX_ERR_DATABASE,
    }
}

/// Read a specific version of a key (Snapshot Read)
#[no_mangle]
pub unsafe extern "C" fn dbx_get_snapshot(
    handle: *mut DbxHandle,
    table: *const c_char,
    key: *const u8,
    key_len: usize,
    read_ts: u64,
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

    match handle.db.get_snapshot(table_str, key_slice, read_ts) {
        Ok(Some(Some(value))) => {
            let len = value.len();
            let ptr = Box::into_raw(value.into_boxed_slice()) as *mut u8;
            *out_value = ptr;
            *out_len = len;
            DBX_OK
        }
        Ok(Some(None)) | Ok(None) => DBX_ERR_NOT_FOUND,
        Err(_) => DBX_ERR_DATABASE,
    }
}

// ═══════════════════════════════════════════════════════════════
// Memory Management
// ═══════════════════════════════════════════════════════════════

/// Free a value returned by dbx_get or dbx_get_snapshot
#[no_mangle]
pub unsafe extern "C" fn dbx_free_value(value: *mut u8, len: usize) {
    if !value.is_null() {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(value, len)));
    }
}

/// Close a database
#[no_mangle]
pub unsafe extern "C" fn dbx_close(handle: *mut DbxHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut DbxHandleInternal);
    }
}

// ═══════════════════════════════════════════════════════════════
// Transaction API
// ═══════════════════════════════════════════════════════════════

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

    type InsertBatch = std::collections::HashMap<String, Vec<(Vec<u8>, Vec<u8>)>>;
    let mut insert_batches: InsertBatch = std::collections::HashMap::new();
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

    for (table, rows) in insert_batches {
        if db_handle.db.insert_batch(&table, rows).is_err() {
            return DBX_ERR_DATABASE;
        }
    }

    for (table, key) in delete_ops {
        if db_handle.db.delete(&table, &key).is_err() {
            return DBX_ERR_DATABASE;
        }
    }

    DBX_OK
}

/// Rollback a transaction
#[no_mangle]
pub unsafe extern "C" fn dbx_transaction_rollback(tx: *mut DbxTransaction) {
    if !tx.is_null() {
        drop(Box::from_raw(tx as *mut DbxTransactionInternal));
    }
}
