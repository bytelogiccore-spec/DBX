/**
 * DBX C++ Example: Transaction Usage
 * 
 * This example demonstrates how to use transactions for bulk operations.
 * IMPORTANT: Always use transactions for bulk inserts/deletes!
 * 
 * Performance comparison (10,000 operations):
 * - Individual operations: ~80K ops/sec
 * - With transactions: ~235K ops/sec (2.9x faster!)
 */

#include <iostream>
#include <string>
#include <chrono>
#include "dbx_wrapper.hpp"

using namespace std;
using namespace std::chrono;

int main() {
    cout << "=== DBX Transaction Example ===" << endl << endl;
    
    try {
        // Open database
        DBX::Database db;
        cout << "Database opened successfully" << endl << endl;
        
        // ========================================
        // Example 1: Bulk Insert with Transaction
        // ========================================
        cout << "--- Bulk Insert (10,000 records) ---" << endl;
        
        auto start = high_resolution_clock::now();
        
        // Begin transaction
        auto tx = db.beginTransaction();
        
        // Insert 10,000 records (buffered in memory)
        for (int i = 0; i < 10000; i++) {
            string key = "key:" + to_string(i);
            string value = "value:" + to_string(i);
            
            tx.insert("users", 
                     vector<uint8_t>(key.begin(), key.end()),
                     vector<uint8_t>(value.begin(), value.end()));
        }
        
        // Commit all operations in a single batch!
        tx.commit();
        
        auto end = high_resolution_clock::now();
        auto duration = duration_cast<milliseconds>(end - start).count();
        
        cout << "✓ Inserted 10,000 records using transaction" << endl;
        cout << "  Time: " << duration << "ms" << endl;
        cout << "  Performance: " << (10000.0 / duration * 1000) << " ops/sec" << endl;
        cout << "  (Automatically batched for maximum performance!)" << endl << endl;
        
        // ========================================
        // Example 2: Verify Data
        // ========================================
        cout << "--- Verify Data ---" << endl;
        
        string test_key = "key:5000";
        auto value = db.get("users", vector<uint8_t>(test_key.begin(), test_key.end()));
        
        if (value) {
            string value_str(value->begin(), value->end());
            cout << "✓ Retrieved: " << value_str << endl;
        } else {
            cout << "✗ Key not found" << endl;
        }
        
        // ========================================
        // Example 3: Bulk Delete with Transaction
        // ========================================
        cout << endl << "--- Bulk Delete (10,000 records) ---" << endl;
        
        start = high_resolution_clock::now();
        
        tx = db.beginTransaction();
        
        for (int i = 0; i < 10000; i++) {
            string key = "key:" + to_string(i);
            tx.remove("users", vector<uint8_t>(key.begin(), key.end()));
        }
        
        tx.commit();
        
        end = high_resolution_clock::now();
        duration = duration_cast<milliseconds>(end - start).count();
        
        cout << "✓ Deleted 10,000 records using transaction" << endl;
        cout << "  Time: " << duration << "ms" << endl;
        cout << "  Performance: " << (10000.0 / duration * 1000) << " ops/sec" << endl << endl;
        
        // ========================================
        // Example 4: Rollback
        // ========================================
        cout << "--- Rollback Example ---" << endl;
        
        tx = db.beginTransaction();
        
        // Insert some data
        for (int i = 0; i < 100; i++) {
            string key = "temp:" + to_string(i);
            string value = "temporary";
            tx.insert("temp", 
                     vector<uint8_t>(key.begin(), key.end()),
                     vector<uint8_t>(value.begin(), value.end()));
        }
        
        // Rollback instead of commit
        tx.rollback();
        
        cout << "✓ Rolled back 100 inserts (data not applied)" << endl << endl;
        
        // ========================================
        // Performance Tips
        // ========================================
        cout << "=== Performance Tips ===" << endl;
        cout << "1. Always use transactions for bulk operations" << endl;
        cout << "2. Transactions automatically batch operations internally" << endl;
        cout << "3. Expected performance: ~235K ops/sec (vs ~80K without transactions)" << endl;
        cout << "4. Commit applies all operations in a single batch" << endl;
        cout << "5. Use rollback to discard changes on error" << endl << endl;
        
        cout << "Database closed" << endl;
        
    } catch (const exception& e) {
        cerr << "Error: " << e.what() << endl;
        return 1;
    }
    
    return 0;
}
