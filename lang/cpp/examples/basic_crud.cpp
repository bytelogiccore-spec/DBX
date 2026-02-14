/**
 * DBX C++ Example - Basic CRUD Operations
 */

#include <iostream>
#include "dbx.hpp"

using namespace dbx;

int main() {
    try {
        std::cout << "Opening in-memory database..." << std::endl;
        auto db = Database::openInMemory();
        
        // Or open a file-based database:
        // Database db("my_database.db");
        
        // Insert some data
        std::cout << "\nInserting data..." << std::endl;
        db.insert("users", "user:1", "Alice");
        db.insert("users", "user:2", "Bob");
        db.insert("users", "user:3", "Charlie");
        
        // Get data
        std::cout << "\nRetrieving data..." << std::endl;
        if (auto value = db.getString("users", "user:1")) {
            std::cout << "user:1 = " << *value << std::endl;
        }
        
        if (auto value = db.getString("users", "user:2")) {
            std::cout << "user:2 = " << *value << std::endl;
        }
        
        // Count rows
        size_t count = db.count("users");
        std::cout << "\nTotal users: " << count << std::endl;
        
        // Delete a row
        std::cout << "\nDeleting user:2..." << std::endl;
        db.remove("users", "user:2");
        
        // Verify deletion
        if (!db.getString("users", "user:2")) {
            std::cout << "user:2 successfully deleted" << std::endl;
        }
        
        // Count again
        count = db.count("users");
        std::cout << "Total users after deletion: " << count << std::endl;
        
        // Flush to disk
        std::cout << "\nFlushing to disk..." << std::endl;
        db.flush();
        
        std::cout << "\n✓ All operations completed successfully!" << std::endl;
        
        // Database is automatically closed by destructor
        
    } catch (const DatabaseError& e) {
        std::cerr << "Database error: " << e.what() << std::endl;
        return 1;
    }
    
    return 0;
}
