/**
 * SmartDatabase - Auto-batching wrapper for DBX
 * 
 * Automatically batches multiple get() calls into a single getBatch() call
 * using request coalescing pattern.
 * 
 * Usage:
 *   const smartDb = new SmartDatabase(db);
 *   const results = await Promise.all([
 *     smartDb.get('table', key1),
 *     smartDb.get('table', key2),
 *     smartDb.get('table', key3),
 *   ]);
 *   // → Internally calls getBatch() once instead of get() three times
 */

class SmartDatabase {
    constructor(db) {
        this._db = db;
        this._pendingGets = new Map();  // table → [{ key, resolve, reject }]
        this._batchScheduled = false;
    }

    /**
     * Get a value by key (auto-batched)
     * @param {string} table - Table name
     * @param {Buffer} key - Key to get
     * @returns {Promise<Buffer|null>} Value or null if not found
     */
    get(table, key) {
        return new Promise((resolve, reject) => {
            // Add request to pending queue
            if (!this._pendingGets.has(table)) {
                this._pendingGets.set(table, []);
            }
            this._pendingGets.get(table).push({ key, resolve, reject });

            // Schedule batch flush if not already scheduled
            if (!this._batchScheduled) {
                this._batchScheduled = true;
                // Execute on next event loop tick
                setImmediate(() => this._flushBatch());
            }
        });
    }

    /**
     * Flush all pending get requests as batches
     * @private
     */
    _flushBatch() {
        this._batchScheduled = false;

        for (const [table, requests] of this._pendingGets) {
            const keys = requests.map(r => r.key);

            try {
                // Execute batch get (synchronous)
                const results = this._db.getBatch(table, keys);

                // Resolve each request with its result
                results.forEach((result, i) => {
                    requests[i].resolve(result);
                });
            } catch (err) {
                // Reject all requests on error
                requests.forEach(r => r.reject(err));
            }
        }

        this._pendingGets.clear();
    }

    // Proxy all other methods to the underlying database
    insert(table, key, value) {
        return this._db.insert(table, key, value);
    }

    delete(table, key) {
        return this._db.delete(table, key);
    }

    scan(table) {
        return this._db.scan(table);
    }

    scanZeroCopy(table) {
        return this._db.scanZeroCopy(table);
    }

    insertBatch(table, rows) {
        return this._db.insertBatch(table, rows);
    }

    deleteBatch(table, keys) {
        return this._db.deleteBatch(table, keys);
    }

    range(table, startKey, endKey) {
        return this._db.range(table, startKey, endKey);
    }

    count(table) {
        return this._db.count(table);
    }

    flush() {
        return this._db.flush();
    }

    tableNames() {
        return this._db.tableNames();
    }

    gc() {
        return this._db.gc();
    }

    isEncrypted() {
        return this._db.isEncrypted();
    }

    executeSql(sql) {
        return this._db.executeSql(sql);
    }

    createIndex(table, column) {
        return this._db.createIndex(table, column);
    }

    dropIndex(table, column) {
        return this._db.dropIndex(table, column);
    }

    hasIndex(table, column) {
        return this._db.hasIndex(table, column);
    }

    saveToFile(path) {
        return this._db.saveToFile(path);
    }

    currentTimestamp() {
        return this._db.currentTimestamp();
    }

    allocateCommitTs() {
        return this._db.allocateCommitTs();
    }

    insertVersioned(table, key, value, commitTs) {
        return this._db.insertVersioned(table, key, value, commitTs);
    }

    getSnapshot(table, key, readTs) {
        return this._db.getSnapshot(table, key, readTs);
    }

    beginTransaction() {
        return this._db.beginTransaction();
    }

    close() {
        return this._db.close();
    }

    // Static factory methods
    static openInMemory() {
        const { Database } = require('./');
        return new SmartDatabase(Database.openInMemory());
    }

    static open(path) {
        const { Database } = require('./');
        return new SmartDatabase(Database.open(path));
    }

    static loadFromFile(path) {
        const { Database } = require('./');
        return new SmartDatabase(Database.loadFromFile(path));
    }
}

module.exports = { SmartDatabase };
