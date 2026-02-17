/**
 * DBX C Example: Transaction Usage
 * 
 * This example demonstrates how to use transactions for bulk operations.
 * IMPORTANT: Always use transactions for bulk inserts/deletes!
 * 
 * Performance comparison (10,000 operations):
 * - Individual operations: ~80K ops/sec
 * - With transactions: ~235K ops/sec (2.9x faster!)
 */

#include <stdio.h>
#include <string.h>
#include "dbx.h"

int main() {
    printf("=== DBX Transaction Example ===\n\n");
    
    // Open database
    DbxHandle* db = dbx_open_in_memory();
    if (!db) {
        fprintf(stderr, "Failed to open database\n");
        return 1;
    }
    
    printf("Database opened successfully\n\n");
    
    // ========================================
    // Example 1: Bulk Insert with Transaction
    // ========================================
    printf("--- Bulk Insert (10,000 records) ---\n");
    
    // Begin transaction
    DbxTransaction* tx = dbx_begin_transaction(db);
    if (!tx) {
        fprintf(stderr, "Failed to begin transaction\n");
        dbx_close(db);
        return 1;
    }
    
    // Insert 10,000 records (buffered in memory)
    for (int i = 0; i < 10000; i++) {
        char key[32], value[32];
        snprintf(key, sizeof(key), "key:%d", i);
        snprintf(value, sizeof(value), "value:%d", i);
        
        int result = dbx_transaction_insert(
            tx,
            "users",
            (const uint8_t*)key, strlen(key),
            (const uint8_t*)value, strlen(value)
        );
        
        if (result != DBX_OK) {
            fprintf(stderr, "Failed to insert key %d\n", i);
            dbx_transaction_rollback(tx);
            dbx_close(db);
            return 1;
        }
    }
    
    // Commit all operations in a single batch!
    int result = dbx_transaction_commit(tx);
    if (result != DBX_OK) {
        fprintf(stderr, "Failed to commit transaction\n");
        dbx_close(db);
        return 1;
    }
    
    printf("✓ Inserted 10,000 records using transaction\n");
    printf("  (Automatically batched for maximum performance!)\n\n");
    
    // ========================================
    // Example 2: Verify Data
    // ========================================
    printf("--- Verify Data ---\n");
    
    const char* test_key = "key:5000";
    uint8_t* value_out = NULL;
    size_t value_len = 0;
    
    result = dbx_get(
        db,
        "users",
        (const uint8_t*)test_key, strlen(test_key),
        &value_out, &value_len
    );
    
    if (result == DBX_OK && value_out) {
        printf("✓ Retrieved: %.*s\n", (int)value_len, value_out);
        dbx_free_value(value_out, value_len);
    } else {
        fprintf(stderr, "Failed to get value\n");
    }
    
    // ========================================
    // Example 3: Bulk Delete with Transaction
    // ========================================
    printf("\n--- Bulk Delete (10,000 records) ---\n");
    
    tx = dbx_begin_transaction(db);
    if (!tx) {
        fprintf(stderr, "Failed to begin transaction\n");
        dbx_close(db);
        return 1;
    }
    
    for (int i = 0; i < 10000; i++) {
        char key[32];
        snprintf(key, sizeof(key), "key:%d", i);
        
        result = dbx_transaction_delete(
            tx,
            "users",
            (const uint8_t*)key, strlen(key)
        );
        
        if (result != DBX_OK) {
            fprintf(stderr, "Failed to delete key %d\n", i);
            dbx_transaction_rollback(tx);
            dbx_close(db);
            return 1;
        }
    }
    
    result = dbx_transaction_commit(tx);
    if (result != DBX_OK) {
        fprintf(stderr, "Failed to commit delete transaction\n");
        dbx_close(db);
        return 1;
    }
    
    printf("✓ Deleted 10,000 records using transaction\n\n");
    
    // ========================================
    // Performance Tips
    // ========================================
    printf("=== Performance Tips ===\n");
    printf("1. Always use transactions for bulk operations\n");
    printf("2. Transactions automatically batch operations internally\n");
    printf("3. Expected performance: ~235K ops/sec (vs ~80K without transactions)\n");
    printf("4. Commit applies all operations in a single batch\n\n");
    
    // Close database
    dbx_close(db);
    printf("Database closed\n");
    
    return 0;
}
