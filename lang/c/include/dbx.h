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

/* Opaque handle to a DBX database instance */
typedef struct DbxHandle DbxHandle;

/* Error codes */
#define DBX_OK 0
#define DBX_ERR_NULL_PTR -1
#define DBX_ERR_INVALID_UTF8 -2
#define DBX_ERR_DATABASE -3
#define DBX_ERR_NOT_FOUND -4

/**
 * Open a database at the given path
 * 
 * @param path Path to the database file (null-terminated string)
 * @return Database handle or NULL on error
 */
DbxHandle* dbx_open(const char* path);

/**
 * Open an in-memory database
 * 
 * @return Database handle or NULL on error
 */
DbxHandle* dbx_open_in_memory(void);

/**
 * Insert a key-value pair into a table
 * 
 * @param handle Database handle
 * @param table Table name (null-terminated string)
 * @param key Key data
 * @param key_len Length of key data
 * @param value Value data
 * @param value_len Length of value data
 * @return DBX_OK on success, error code otherwise
 */
int dbx_insert(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    const uint8_t* value,
    size_t value_len
);

/**
 * Get a value by key from a table
 * 
 * @param handle Database handle
 * @param table Table name (null-terminated string)
 * @param key Key data
 * @param key_len Length of key data
 * @param out_value Pointer to receive value data (must be freed with dbx_free_value)
 * @param out_len Pointer to receive value length
 * @return DBX_OK on success, DBX_ERR_NOT_FOUND if key not found, error code otherwise
 */
int dbx_get(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len,
    uint8_t** out_value,
    size_t* out_len
);

/**
 * Delete a key from a table
 * 
 * @param handle Database handle
 * @param table Table name (null-terminated string)
 * @param key Key data
 * @param key_len Length of key data
 * @return DBX_OK on success, error code otherwise
 */
int dbx_delete(
    DbxHandle* handle,
    const char* table,
    const uint8_t* key,
    size_t key_len
);

/**
 * Count rows in a table
 * 
 * @param handle Database handle
 * @param table Table name (null-terminated string)
 * @param out_count Pointer to receive row count
 * @return DBX_OK on success, error code otherwise
 */
int dbx_count(
    DbxHandle* handle,
    const char* table,
    size_t* out_count
);

/**
 * Flush database to disk
 * 
 * @param handle Database handle
 * @return DBX_OK on success, error code otherwise
 */
int dbx_flush(DbxHandle* handle);

/**
 * Free a value returned by dbx_get
 * 
 * @param value Value pointer to free
 * @param len Length of value
 */
void dbx_free_value(uint8_t* value, size_t len);

/**
 * Close the database and free resources
 * 
 * @param handle Database handle
 */
void dbx_close(DbxHandle* handle);

/**
 * Get the last error message
 * 
 * @return Error message string (static, do not free)
 */
const char* dbx_last_error(void);


/* ========================================
 * Transaction API
 * ========================================
 * 
 * IMPORTANT: Always use transactions for bulk operations!
 * Performance: ~235K ops/sec (vs ~80K without transactions)
 */

/* Opaque handle to a transaction */
typedef struct DbxTransaction DbxTransaction;

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
