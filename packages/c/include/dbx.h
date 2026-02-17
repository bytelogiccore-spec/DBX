/**
 * DBX C FFI Bindings
 * 
 * C interface for the DBX high-performance database.
 */

#ifndef DBX_H
#define DBX_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>

/* Opaque handles */
typedef struct DbxHandle DbxHandle;
typedef struct DbxTransaction DbxTransaction;
typedef struct DbxScanResult DbxScanResult;
typedef struct DbxStringList DbxStringList;

/* Error codes */
#define DBX_OK 0
#define DBX_ERR_NULL_PTR -1
#define DBX_ERR_INVALID_UTF8 -2
#define DBX_ERR_DATABASE -3
#define DBX_ERR_NOT_FOUND -4
#define DBX_ERR_INVALID_OP -5

/* ========================================
 * Constructors
 * ======================================== */

DbxHandle* dbx_open(const char* path);
DbxHandle* dbx_open_in_memory(void);
DbxHandle* dbx_load_from_file(const char* path);

/* ========================================
 * CRUD Operations
 * ======================================== */

int dbx_insert(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    const uint8_t* value,
    size_t value_len
);

int dbx_get(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    uint8_t** out_value,
    size_t* out_len
);

int dbx_delete(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len
);

/* ========================================
 * Batch Operations
 * ======================================== */

int dbx_insert_batch(
    DbxHandle* handle,
    const char* table,
    const uint8_t** keys,
    const size_t* key_lens,
    const uint8_t** values,
    const size_t* value_lens,
    size_t count
);

int dbx_scan(
    DbxHandle* handle,
    const char* table,
    DbxScanResult** out_result
);

int dbx_range(
    DbxHandle* handle,
    const char* table,
    const uint8_t* start_key,
    size_t start_key_len,
    const uint8_t* end_key,
    size_t end_key_len,
    DbxScanResult** out_result
);

/* Scan result accessors */
size_t dbx_scan_result_count(const DbxScanResult* result);
int dbx_scan_result_key(const DbxScanResult* result, size_t index,
                        const uint8_t** out_key, size_t* out_key_len);
int dbx_scan_result_value(const DbxScanResult* result, size_t index,
                          const uint8_t** out_value, size_t* out_value_len);
void dbx_scan_result_free(DbxScanResult* result);

/* ========================================
 * Utility Operations
 * ======================================== */

int dbx_count(DbxHandle* handle, const char* table, size_t* out_count);
int dbx_flush(DbxHandle* handle);
int dbx_table_names(DbxHandle* handle, DbxStringList** out_list);
int dbx_gc(DbxHandle* handle, size_t* out_deleted);
int dbx_is_encrypted(DbxHandle* handle);

/* String list accessors */
size_t dbx_string_list_count(const DbxStringList* list);
int dbx_string_list_get(const DbxStringList* list, size_t index,
                        const uint8_t** out_str, size_t* out_len);
void dbx_string_list_free(DbxStringList* list);

/* ========================================
 * SQL Operations
 * ======================================== */

int dbx_execute_sql(
    DbxHandle* handle,
    const char* sql,
    size_t* out_affected
);

/* ========================================
 * Index Operations
 * ======================================== */

int dbx_create_index(DbxHandle* handle, const char* table, const char* column);
int dbx_drop_index(DbxHandle* handle, const char* table, const char* column);
int dbx_has_index(DbxHandle* handle, const char* table, const char* column);

/* ========================================
 * Snapshot Operations
 * ======================================== */

int dbx_save_to_file(DbxHandle* handle, const char* path);

/* ========================================
 * MVCC Operations
 * ======================================== */

uint64_t dbx_current_timestamp(DbxHandle* handle);
uint64_t dbx_allocate_commit_ts(DbxHandle* handle);

int dbx_insert_versioned(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    const uint8_t* value,
    size_t value_len,
    uint64_t commit_ts
);

int dbx_get_snapshot(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    uint64_t read_ts,
    uint8_t** out_value,
    size_t* out_len
);

/* ========================================
 * Memory Management
 * ======================================== */

void dbx_free_value(uint8_t* value, size_t len);
void dbx_close(DbxHandle* handle);
const char* dbx_last_error(void);

/* ========================================
 * Transaction API
 * ======================================== */

DbxTransaction* dbx_begin_transaction(DbxHandle* handle);

int dbx_transaction_insert(
    DbxTransaction* tx,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    const uint8_t* value,
    size_t value_len
);

int dbx_transaction_delete(
    DbxTransaction* tx,
    const char* table,
    const uint8_t* key,
    size_t key_len
);

int dbx_transaction_commit(DbxTransaction* tx);
void dbx_transaction_rollback(DbxTransaction* tx);


#ifdef __cplusplus
}
#endif

#endif /* DBX_H */
