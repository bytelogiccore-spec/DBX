#ifndef DBX_H
#define DBX_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Error codes
 */
#define DBX_OK 0

#define DBX_ERR_NULL_PTR -1

#define DBX_ERR_INVALID_UTF8 -2

#define DBX_ERR_DATABASE -3

#define DBX_ERR_NOT_FOUND -4

/**
 * Opaque handle to a DBX database instance
 */
typedef struct DbxHandle {
  Database db;
} DbxHandle;

/**
 * Open a database at the given path
 */
struct DbxHandle *dbx_open(const char *path);

/**
 * Open an in-memory database
 */
struct DbxHandle *dbx_open_in_memory(void);

/**
 * Insert a key-value pair into a table
 */
int dbx_insert(struct DbxHandle *handle,
               const char *table,
               const uint8_t *key,
               uintptr_t key_len,
               const uint8_t *value,
               uintptr_t value_len);

/**
 * Get a value by key from a table
 */
int dbx_get(struct DbxHandle *handle,
            const char *table,
            const uint8_t *key,
            uintptr_t key_len,
            uint8_t **out_value,
            uintptr_t *out_len);

/**
 * Delete a key from a table
 */
int dbx_delete(struct DbxHandle *handle, const char *table, const uint8_t *key, uintptr_t key_len);

/**
 * Count rows in a table
 */
int dbx_count(struct DbxHandle *handle, const char *table, uintptr_t *out_count);

/**
 * Flush database to disk
 */
int dbx_flush(struct DbxHandle *handle);

/**
 * Free a value returned by dbx_get
 */
void dbx_free_value(uint8_t *value, uintptr_t len);

/**
 * Close the database and free resources
 */
void dbx_close(struct DbxHandle *handle);

/**
 * Get the last error message
 */
const char *dbx_last_error(void);

/**
 * Opaque handle to zero-copy scan results
 */
typedef struct DbxZeroCopyScanResult DbxZeroCopyScanResult;

/**
 * Zero-copy scan - returns serialized flat buffer
 */
int dbx_scan_zero_copy(struct DbxHandle *handle,
                       const char *table,
                       struct DbxZeroCopyScanResult **out_result);

/**
 * Get raw data pointer from zero-copy scan result
 */
int dbx_zero_copy_result_data(const struct DbxZeroCopyScanResult *result,
                               const uint8_t **out_data,
                               uintptr_t *out_len);

/**
 * Get the number of entries in a zero-copy scan result
 */
uintptr_t dbx_zero_copy_result_count(const struct DbxZeroCopyScanResult *result);

/**
 * Free a zero-copy scan result
 */
void dbx_zero_copy_result_free(struct DbxZeroCopyScanResult *result);

#endif /* DBX_H */
