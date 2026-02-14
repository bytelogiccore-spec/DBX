/**
 * DBX C Example - Basic CRUD Operations
 */

#include <stdio.h>
#include <string.h>
#include "dbx.h"

int main() {
    printf("Opening in-memory database...\n");
    DbxHandle* db = dbx_open_in_memory();
    
    // Or open a file-based database:
    // DbxHandle* db = dbx_open("my_database.db");
    
    if (db == NULL) {
        fprintf(stderr, "Failed to open database\n");
        return 1;
    }
    
    // Insert some data
    printf("\nInserting data...\n");
    const char* table = "users";
    
    int result = dbx_insert(db, table, 
                           (const uint8_t*)"user:1", 6,
                           (const uint8_t*)"Alice", 5);
    if (result != DBX_OK) {
        fprintf(stderr, "Insert failed\n");
        dbx_close(db);
        return 1;
    }
    
    dbx_insert(db, table,
              (const uint8_t*)"user:2", 6,
              (const uint8_t*)"Bob", 3);
    
    dbx_insert(db, table,
              (const uint8_t*)"user:3", 6,
              (const uint8_t*)"Charlie", 7);
    
    // Get data
    printf("\nRetrieving data...\n");
    uint8_t* value = NULL;
    size_t value_len = 0;
    
    result = dbx_get(db, table,
                    (const uint8_t*)"user:1", 6,
                    &value, &value_len);
    
    if (result == DBX_OK && value != NULL) {
        printf("user:1 = %.*s\n", (int)value_len, value);
        dbx_free_value(value, value_len);
    }
    
    // Count rows
    size_t count = 0;
    result = dbx_count(db, table, &count);
    if (result == DBX_OK) {
        printf("\nTotal users: %zu\n", count);
    }
    
    // Delete a row
    printf("\nDeleting user:2...\n");
    result = dbx_delete(db, table, (const uint8_t*)"user:2", 6);
    if (result == DBX_OK) {
        printf("user:2 successfully deleted\n");
    }
    
    // Count again
    result = dbx_count(db, table, &count);
    if (result == DBX_OK) {
        printf("Total users after deletion: %zu\n", count);
    }
    
    // Flush to disk
    printf("\nFlushing to disk...\n");
    dbx_flush(db);
    
    printf("\n✓ All operations completed successfully!\n");
    
    // Close database
    printf("\nClosing database...\n");
    dbx_close(db);
    
    return 0;
}
