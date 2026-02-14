/**
 * DBX C++ Wrapper
 * 
 * Modern C++ interface for the DBX database.
 */

#ifndef DBX_HPP
#define DBX_HPP

#include "dbx.h"
#include <string>
#include <vector>
#include <memory>
#include <optional>
#include <stdexcept>

namespace dbx {

class DatabaseError : public std::runtime_error {
public:
    explicit DatabaseError(const std::string& message)
        : std::runtime_error(message) {}
};

class Database {
public:
    /**
     * Open a database at the specified path
     */
    explicit Database(const std::string& path) {
        handle_ = dbx_open(path.c_str());
        if (handle_ == nullptr) {
            throw DatabaseError("Failed to open database: " + path);
        }
    }
    
    /**
     * Open an in-memory database
     */
    static Database openInMemory() {
        return Database(true);
    }
    
    /**
     * Destructor - automatically closes the database
     */
    ~Database() {
        if (handle_ != nullptr) {
            dbx_close(handle_);
        }
    }
    
    // Disable copy
    Database(const Database&) = delete;
    Database& operator=(const Database&) = delete;
    
    // Enable move
    Database(Database&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }
    
    Database& operator=(Database&& other) noexcept {
        if (this != &other) {
            if (handle_ != nullptr) {
                dbx_close(handle_);
            }
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }
    
    /**
     * Insert a key-value pair into a table
     */
    void insert(const std::string& table, 
                const std::vector<uint8_t>& key,
                const std::vector<uint8_t>& value) {
        int result = dbx_insert(handle_, table.c_str(),
                               key.data(), key.size(),
                               value.data(), value.size());
        if (result != DBX_OK) {
            throw DatabaseError("Insert failed");
        }
    }
    
    /**
     * Insert a key-value pair (string overload)
     */
    void insert(const std::string& table,
                const std::string& key,
                const std::string& value) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        std::vector<uint8_t> value_vec(value.begin(), value.end());
        insert(table, key_vec, value_vec);
    }
    
    /**
     * Get a value by key from a table
     */
    std::optional<std::vector<uint8_t>> get(const std::string& table,
                                            const std::vector<uint8_t>& key) {
        uint8_t* value = nullptr;
        size_t value_len = 0;
        
        int result = dbx_get(handle_, table.c_str(),
                            key.data(), key.size(),
                            &value, &value_len);
        
        if (result == DBX_ERR_NOT_FOUND) {
            return std::nullopt;
        }
        
        if (result != DBX_OK) {
            throw DatabaseError("Get failed");
        }
        
        std::vector<uint8_t> result_vec(value, value + value_len);
        dbx_free_value(value, value_len);
        
        return result_vec;
    }
    
    /**
     * Get a value (string overload)
     */
    std::optional<std::string> getString(const std::string& table,
                                         const std::string& key) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        auto value = get(table, key_vec);
        
        if (!value) {
            return std::nullopt;
        }
        
        return std::string(value->begin(), value->end());
    }
    
    /**
     * Delete a key from a table
     */
    void remove(const std::string& table, const std::vector<uint8_t>& key) {
        int result = dbx_delete(handle_, table.c_str(),
                               key.data(), key.size());
        if (result != DBX_OK) {
            throw DatabaseError("Delete failed");
        }
    }
    
    /**
     * Delete a key (string overload)
     */
    void remove(const std::string& table, const std::string& key) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        remove(table, key_vec);
    }
    
    /**
     * Count rows in a table
     */
    size_t count(const std::string& table) {
        size_t count = 0;
        int result = dbx_count(handle_, table.c_str(), &count);
        if (result != DBX_OK) {
            throw DatabaseError("Count failed");
        }
        return count;
    }
    
    /**
     * Flush database to disk
     */
    void flush() {
        int result = dbx_flush(handle_);
        if (result != DBX_OK) {
            throw DatabaseError("Flush failed");
        }
    }
    
    /**
     * Close the database
     */
    void close() {
        if (handle_ != nullptr) {
            dbx_close(handle_);
            handle_ = nullptr;
        }
    }

private:
    // Private constructor for in-memory database
    explicit Database(bool /* in_memory */) {
        handle_ = dbx_open_in_memory();
        if (handle_ == nullptr) {
            throw DatabaseError("Failed to open in-memory database");
        }
    }
    
    DbxHandle* handle_;
};

} // namespace dbx

#endif // DBX_HPP
