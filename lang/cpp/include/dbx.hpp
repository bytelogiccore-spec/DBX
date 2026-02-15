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
#include <cstdint>
#include <utility>

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
     * Load a database from a snapshot file
     */
    static Database loadFromFile(const std::string& path) {
        Database db;
        db.handle_ = dbx_load_from_file(path.c_str());
        if (db.handle_ == nullptr) {
            throw DatabaseError("Failed to load database from: " + path);
        }
        return db;
    }
    
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
    
    // ═══════════════════════════════════════════════════
    // CRUD Operations
    // ═══════════════════════════════════════════════════
    
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
    
    void insert(const std::string& table,
                const std::string& key,
                const std::string& value) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        std::vector<uint8_t> value_vec(value.begin(), value.end());
        insert(table, key_vec, value_vec);
    }
    
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
    
    std::optional<std::string> getString(const std::string& table,
                                         const std::string& key) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        auto value = get(table, key_vec);
        
        if (!value) {
            return std::nullopt;
        }
        
        return std::string(value->begin(), value->end());
    }
    
    void remove(const std::string& table, const std::vector<uint8_t>& key) {
        int result = dbx_delete(handle_, table.c_str(),
                               key.data(), key.size());
        if (result != DBX_OK) {
            throw DatabaseError("Delete failed");
        }
    }
    
    void remove(const std::string& table, const std::string& key) {
        std::vector<uint8_t> key_vec(key.begin(), key.end());
        remove(table, key_vec);
    }

    // ═══════════════════════════════════════════════════
    // Batch Operations
    // ═══════════════════════════════════════════════════

    void insertBatch(const std::string& table,
                     const std::vector<std::pair<std::vector<uint8_t>, std::vector<uint8_t>>>& rows) {
        std::vector<const uint8_t*> keys, values;
        std::vector<size_t> key_lens, value_lens;
        keys.reserve(rows.size());
        values.reserve(rows.size());
        key_lens.reserve(rows.size());
        value_lens.reserve(rows.size());

        for (const auto& [k, v] : rows) {
            keys.push_back(k.data());
            key_lens.push_back(k.size());
            values.push_back(v.data());
            value_lens.push_back(v.size());
        }

        int result = dbx_insert_batch(handle_, table.c_str(),
                                      keys.data(), key_lens.data(),
                                      values.data(), value_lens.data(),
                                      rows.size());
        if (result != DBX_OK) {
            throw DatabaseError("Batch insert failed");
        }
    }

    std::vector<std::pair<std::vector<uint8_t>, std::vector<uint8_t>>>
    scan(const std::string& table) {
        DbxScanResult* result = nullptr;
        int rc = dbx_scan(handle_, table.c_str(), &result);
        if (rc != DBX_OK) {
            throw DatabaseError("Scan failed");
        }

        size_t count = dbx_scan_result_count(result);
        std::vector<std::pair<std::vector<uint8_t>, std::vector<uint8_t>>> entries;
        entries.reserve(count);

        for (size_t i = 0; i < count; ++i) {
            const uint8_t* key = nullptr;
            size_t key_len = 0;
            const uint8_t* value = nullptr;
            size_t value_len = 0;
            dbx_scan_result_key(result, i, &key, &key_len);
            dbx_scan_result_value(result, i, &value, &value_len);
            entries.emplace_back(
                std::vector<uint8_t>(key, key + key_len),
                std::vector<uint8_t>(value, value + value_len)
            );
        }

        dbx_scan_result_free(result);
        return entries;
    }

    std::vector<std::pair<std::vector<uint8_t>, std::vector<uint8_t>>>
    range(const std::string& table,
          const std::vector<uint8_t>& startKey,
          const std::vector<uint8_t>& endKey) {
        DbxScanResult* result = nullptr;
        int rc = dbx_range(handle_, table.c_str(),
                           startKey.data(), startKey.size(),
                           endKey.data(), endKey.size(),
                           &result);
        if (rc != DBX_OK) {
            throw DatabaseError("Range scan failed");
        }

        size_t count = dbx_scan_result_count(result);
        std::vector<std::pair<std::vector<uint8_t>, std::vector<uint8_t>>> entries;
        entries.reserve(count);

        for (size_t i = 0; i < count; ++i) {
            const uint8_t* key = nullptr;
            size_t key_len = 0;
            const uint8_t* value = nullptr;
            size_t value_len = 0;
            dbx_scan_result_key(result, i, &key, &key_len);
            dbx_scan_result_value(result, i, &value, &value_len);
            entries.emplace_back(
                std::vector<uint8_t>(key, key + key_len),
                std::vector<uint8_t>(value, value + value_len)
            );
        }

        dbx_scan_result_free(result);
        return entries;
    }

    // ═══════════════════════════════════════════════════
    // Utility Operations
    // ═══════════════════════════════════════════════════
    
    size_t count(const std::string& table) {
        size_t cnt = 0;
        int result = dbx_count(handle_, table.c_str(), &cnt);
        if (result != DBX_OK) {
            throw DatabaseError("Count failed");
        }
        return cnt;
    }
    
    void flush() {
        int result = dbx_flush(handle_);
        if (result != DBX_OK) {
            throw DatabaseError("Flush failed");
        }
    }

    std::vector<std::string> tableNames() {
        DbxStringList* list = nullptr;
        int rc = dbx_table_names(handle_, &list);
        if (rc != DBX_OK) {
            throw DatabaseError("Failed to get table names");
        }

        size_t cnt = dbx_string_list_count(list);
        std::vector<std::string> names;
        names.reserve(cnt);

        for (size_t i = 0; i < cnt; ++i) {
            const uint8_t* str = nullptr;
            size_t len = 0;
            dbx_string_list_get(list, i, &str, &len);
            names.emplace_back(reinterpret_cast<const char*>(str), len);
        }

        dbx_string_list_free(list);
        return names;
    }

    size_t gc() {
        size_t deleted = 0;
        int result = dbx_gc(handle_, &deleted);
        if (result != DBX_OK) {
            throw DatabaseError("GC failed");
        }
        return deleted;
    }

    bool isEncrypted() {
        return dbx_is_encrypted(handle_) != 0;
    }

    // ═══════════════════════════════════════════════════
    // SQL Operations
    // ═══════════════════════════════════════════════════

    size_t executeSql(const std::string& sql) {
        size_t affected = 0;
        int result = dbx_execute_sql(handle_, sql.c_str(), &affected);
        if (result != DBX_OK) {
            throw DatabaseError("SQL execution failed");
        }
        return affected;
    }

    // ═══════════════════════════════════════════════════
    // Index Operations
    // ═══════════════════════════════════════════════════

    void createIndex(const std::string& table, const std::string& column) {
        int result = dbx_create_index(handle_, table.c_str(), column.c_str());
        if (result != DBX_OK) {
            throw DatabaseError("Create index failed");
        }
    }

    void dropIndex(const std::string& table, const std::string& column) {
        int result = dbx_drop_index(handle_, table.c_str(), column.c_str());
        if (result != DBX_OK) {
            throw DatabaseError("Drop index failed");
        }
    }

    bool hasIndex(const std::string& table, const std::string& column) {
        return dbx_has_index(handle_, table.c_str(), column.c_str()) != 0;
    }

    // ═══════════════════════════════════════════════════
    // Snapshot Operations
    // ═══════════════════════════════════════════════════

    void saveToFile(const std::string& path) {
        int result = dbx_save_to_file(handle_, path.c_str());
        if (result != DBX_OK) {
            throw DatabaseError("Save failed");
        }
    }

    // ═══════════════════════════════════════════════════
    // MVCC Operations
    // ═══════════════════════════════════════════════════

    uint64_t currentTimestamp() {
        return dbx_current_timestamp(handle_);
    }

    uint64_t allocateCommitTs() {
        return dbx_allocate_commit_ts(handle_);
    }

    void insertVersioned(const std::string& table,
                         const std::vector<uint8_t>& key,
                         const std::vector<uint8_t>& value,
                         uint64_t commitTs) {
        int result = dbx_insert_versioned(handle_, table.c_str(),
                                          key.data(), key.size(),
                                          value.data(), value.size(),
                                          commitTs);
        if (result != DBX_OK) {
            throw DatabaseError("Versioned insert failed");
        }
    }

    std::optional<std::vector<uint8_t>> getSnapshot(const std::string& table,
                                                     const std::vector<uint8_t>& key,
                                                     uint64_t readTs) {
        uint8_t* value = nullptr;
        size_t value_len = 0;

        int result = dbx_get_snapshot(handle_, table.c_str(),
                                      key.data(), key.size(),
                                      readTs, &value, &value_len);

        if (result == DBX_ERR_NOT_FOUND) {
            return std::nullopt;
        }
        if (result != DBX_OK) {
            throw DatabaseError("Snapshot read failed");
        }

        std::vector<uint8_t> result_vec(value, value + value_len);
        dbx_free_value(value, value_len);
        return result_vec;
    }

    // ═══════════════════════════════════════════════════
    // Transaction
    // ═══════════════════════════════════════════════════

    class Transaction {
    public:
        Transaction(const Transaction&) = delete;
        Transaction& operator=(const Transaction&) = delete;

        Transaction(Transaction&& other) noexcept : tx_(other.tx_) {
            other.tx_ = nullptr;
        }

        ~Transaction() {
            if (tx_ != nullptr) {
                dbx_transaction_rollback(tx_);
            }
        }

        void insert(const std::string& table,
                     const std::vector<uint8_t>& key,
                     const std::vector<uint8_t>& value) {
            int result = dbx_transaction_insert(tx_, table.c_str(),
                                                 key.data(), key.size(),
                                                 value.data(), value.size());
            if (result != DBX_OK) {
                throw DatabaseError("Transaction insert failed");
            }
        }

        void insert(const std::string& table,
                     const std::string& key,
                     const std::string& value) {
            std::vector<uint8_t> k(key.begin(), key.end());
            std::vector<uint8_t> v(value.begin(), value.end());
            insert(table, k, v);
        }

        void remove(const std::string& table,
                     const std::vector<uint8_t>& key) {
            int result = dbx_transaction_delete(tx_, table.c_str(),
                                                 key.data(), key.size());
            if (result != DBX_OK) {
                throw DatabaseError("Transaction delete failed");
            }
        }

        void commit() {
            int result = dbx_transaction_commit(tx_);
            if (result != DBX_OK) {
                throw DatabaseError("Transaction commit failed");
            }
            tx_ = nullptr; // prevent rollback in destructor
        }

        void rollback() {
            if (tx_ != nullptr) {
                dbx_transaction_rollback(tx_);
                tx_ = nullptr;
            }
        }

    private:
        friend class Database;
        explicit Transaction(DbxTransaction* tx) : tx_(tx) {}
        DbxTransaction* tx_;
    };

    Transaction beginTransaction() {
        DbxTransaction* tx = dbx_begin_transaction(handle_);
        if (tx == nullptr) {
            throw DatabaseError("Failed to begin transaction");
        }
        return Transaction(tx);
    }

    // ═══════════════════════════════════════════════════
    // Lifecycle
    // ═══════════════════════════════════════════════════
    
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

    // Default constructor for factory methods
    Database() : handle_(nullptr) {}
    
    DbxHandle* handle_;
};

} // namespace dbx

#endif // DBX_HPP

