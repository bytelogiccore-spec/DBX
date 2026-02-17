using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace DBX.Dotnet
{
    /// <summary>
    /// High-performance native DBX database using CsBindgen
    /// </summary>
    public unsafe class Database : IDisposable
    {
        private DbxHandle* _handle;
        private bool _disposed;

        private Database(DbxHandle* handle)
        {
            _handle = handle;
        }

        // ═══════════════════════════════════════════════════
        // Constructors
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Open an in-memory database
        /// </summary>
        public static Database OpenInMemory()
        {
            var handle = NativeMethods.dbx_open_in_memory();
            if (handle == null)
                throw new Exception("Failed to open in-memory database");
            return new Database(handle);
        }

        /// <summary>
        /// Open a database at the given path
        /// </summary>
        public static Database Open(string path)
        {
            var pathBytes = Encoding.UTF8.GetBytes(path + "\0");
            fixed (byte* pathPtr = pathBytes)
            {
                var handle = NativeMethods.dbx_open(pathPtr);
                if (handle == null)
                    throw new Exception($"Failed to open database at {path}");
                return new Database(handle);
            }
        }

        /// <summary>
        /// Load a database from a snapshot file
        /// </summary>
        public static Database LoadFromFile(string path)
        {
            var pathBytes = Encoding.UTF8.GetBytes(path + "\0");
            fixed (byte* pathPtr = pathBytes)
            {
                var handle = NativeMethods.dbx_load_from_file(pathPtr);
                if (handle == null)
                    throw new Exception($"Failed to load database from {path}");
                return new Database(handle);
            }
        }

        // ═══════════════════════════════════════════════════
        // CRUD Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Insert a key-value pair into a table
        /// </summary>
        public void Insert(string table, byte[] key, byte[] value)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            fixed (byte* valuePtr = value)
            {
                var result = NativeMethods.dbx_insert(
                    _handle, tablePtr,
                    keyPtr, (nuint)key.Length,
                    valuePtr, (nuint)value.Length
                );
                
                if (result != 0)
                    throw new Exception($"Insert failed with error code: {result}");
            }
        }

        /// <summary>
        /// Get a value by key from a table
        /// </summary>
        public byte[]? Get(string table, byte[] key)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            byte* outValue = null;
            nuint outLen = 0;
            
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            {
                var result = NativeMethods.dbx_get(
                    _handle, tablePtr,
                    keyPtr, (nuint)key.Length,
                    &outValue, &outLen
                );
                
                if (result == -4) // DBX_ERR_NOT_FOUND
                    return null;
                
                if (result != 0)
                    throw new Exception($"Get failed with error code: {result}");
                
                if (outValue == null)
                    return null;
                
                try
                {
                    var value = new byte[outLen];
                    Marshal.Copy((IntPtr)outValue, value, 0, (int)outLen);
                    return value;
                }
                finally
                {
                    NativeMethods.dbx_free_value(outValue, outLen);
                }
            }
        }

        /// <summary>
        /// Delete a key from a table
        /// </summary>
        public void Delete(string table, byte[] key)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            {
                var result = NativeMethods.dbx_delete(
                    _handle, tablePtr,
                    keyPtr, (nuint)key.Length
                );
                
                if (result != 0)
                    throw new Exception($"Delete failed with error code: {result}");
            }
        }

        /// <summary>
        /// Insert multiple key-value pairs in a batch
        /// </summary>
        public void InsertBatch(string table, List<KeyValuePair<byte[], byte[]>> rows)
        {
            ThrowIfDisposed();

            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            var count = rows.Count;
            var keyPtrs = new IntPtr[count];
            var keyLens = new nuint[count];
            var valPtrs = new IntPtr[count];
            var valLens = new nuint[count];
            var handles = new System.Runtime.InteropServices.GCHandle[count * 2];

            try
            {
                for (int i = 0; i < count; i++)
                {
                    handles[i * 2] = System.Runtime.InteropServices.GCHandle.Alloc(rows[i].Key, System.Runtime.InteropServices.GCHandleType.Pinned);
                    handles[i * 2 + 1] = System.Runtime.InteropServices.GCHandle.Alloc(rows[i].Value, System.Runtime.InteropServices.GCHandleType.Pinned);
                    keyPtrs[i] = handles[i * 2].AddrOfPinnedObject();
                    keyLens[i] = (nuint)rows[i].Key.Length;
                    valPtrs[i] = handles[i * 2 + 1].AddrOfPinnedObject();
                    valLens[i] = (nuint)rows[i].Value.Length;
                }

                fixed (byte* tablePtr = tableBytes)
                fixed (IntPtr* kp = keyPtrs)
                fixed (nuint* kl = keyLens)
                fixed (IntPtr* vp = valPtrs)
                fixed (nuint* vl = valLens)
                {
                    var result = NativeMethods.dbx_insert_batch(
                        _handle, tablePtr,
                        (byte**)kp, kl,
                        (byte**)vp, vl,
                        (nuint)count
                    );

                    if (result != 0)
                        throw new Exception($"Batch insert failed with error code: {result}");
                }
            }
            finally
            {
                foreach (var h in handles)
                {
                    if (h.IsAllocated) h.Free();
                }
            }
        }

        /// <summary>
        /// Scan all key-value pairs in a table
        /// </summary>
        public List<KeyValuePair<byte[], byte[]>> Scan(string table)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            DbxScanResult* scanResult = null;
            
            fixed (byte* tablePtr = tableBytes)
            {
                var rc = NativeMethods.dbx_scan(_handle, tablePtr, &scanResult);
                if (rc != 0)
                    throw new Exception($"Scan failed with error code: {rc}");
            }

            var count = NativeMethods.dbx_scan_result_count(scanResult);
            var entries = new List<KeyValuePair<byte[], byte[]>>((int)count);

            for (nuint i = 0; i < count; i++)
            {
                byte* keyPtr = null;
                nuint keyLen = 0;
                byte* valPtr = null;
                nuint valLen = 0;

                NativeMethods.dbx_scan_result_key(scanResult, i, &keyPtr, &keyLen);
                NativeMethods.dbx_scan_result_value(scanResult, i, &valPtr, &valLen);

                var key = new byte[keyLen];
                var val = new byte[valLen];
                Marshal.Copy((IntPtr)keyPtr, key, 0, (int)keyLen);
                Marshal.Copy((IntPtr)valPtr, val, 0, (int)valLen);

                entries.Add(new KeyValuePair<byte[], byte[]>(key, val));
            }

            NativeMethods.dbx_scan_result_free(scanResult);
            return entries;
        }

        /// <summary>
        /// Scan a range of keys [startKey, endKey)
        /// </summary>
        public List<KeyValuePair<byte[], byte[]>> Range(string table, byte[] startKey, byte[] endKey)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            DbxScanResult* scanResult = null;

            fixed (byte* tablePtr = tableBytes)
            fixed (byte* startPtr = startKey)
            fixed (byte* endPtr = endKey)
            {
                var rc = NativeMethods.dbx_range(
                    _handle, tablePtr,
                    startPtr, (nuint)startKey.Length,
                    endPtr, (nuint)endKey.Length,
                    &scanResult
                );
                if (rc != 0)
                    throw new Exception($"Range scan failed with error code: {rc}");
            }

            var count = NativeMethods.dbx_scan_result_count(scanResult);
            var entries = new List<KeyValuePair<byte[], byte[]>>((int)count);

            for (nuint i = 0; i < count; i++)
            {
                byte* keyPtr = null;
                nuint keyLen = 0;
                byte* valPtr = null;
                nuint valLen = 0;

                NativeMethods.dbx_scan_result_key(scanResult, i, &keyPtr, &keyLen);
                NativeMethods.dbx_scan_result_value(scanResult, i, &valPtr, &valLen);

                var key = new byte[keyLen];
                var val = new byte[valLen];
                Marshal.Copy((IntPtr)keyPtr, key, 0, (int)keyLen);
                Marshal.Copy((IntPtr)valPtr, val, 0, (int)valLen);

                entries.Add(new KeyValuePair<byte[], byte[]>(key, val));
            }

            NativeMethods.dbx_scan_result_free(scanResult);
            return entries;
        }

        /// <summary>
        /// Zero-copy scan - returns a result that owns the serialized data
        /// </summary>
        public ZeroCopyScanResult ScanZeroCopy(string table)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            DbxZeroCopyScanResult* scanResult = null;
            
            fixed (byte* tablePtr = tableBytes)
            {
                var rc = NativeMethods.dbx_scan_zero_copy(_handle, tablePtr, &scanResult);
                if (rc != 0)
                    throw new Exception($"Zero-copy scan failed with error code: {rc}");
            }

            return new ZeroCopyScanResult(scanResult);
        }

        // ═══════════════════════════════════════════════════
        // Utility Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Count the number of rows in a table
        /// </summary>
        public ulong Count(string table)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            nuint outCount = 0;
            
            fixed (byte* tablePtr = tableBytes)
            {
                var result = NativeMethods.dbx_count(_handle, tablePtr, &outCount);
                if (result != 0)
                    throw new Exception($"Count failed with error code: {result}");
            }

            return (ulong)outCount;
        }

        /// <summary>
        /// Flush database to disk
        /// </summary>
        public void Flush()
        {
            ThrowIfDisposed();
            var result = NativeMethods.dbx_flush(_handle);
            if (result != 0)
                throw new Exception($"Flush failed with error code: {result}");
        }

        /// <summary>
        /// Get all table names
        /// </summary>
        public List<string> TableNames()
        {
            ThrowIfDisposed();

            DbxStringList* list = null;
            var rc = NativeMethods.dbx_table_names(_handle, &list);
            if (rc != 0)
                throw new Exception($"Table names failed with error code: {rc}");

            var count = NativeMethods.dbx_string_list_count(list);
            var names = new List<string>((int)count);

            for (nuint i = 0; i < count; i++)
            {
                byte* strPtr = null;
                nuint strLen = 0;
                NativeMethods.dbx_string_list_get(list, i, &strPtr, &strLen);
                names.Add(Encoding.UTF8.GetString(strPtr, (int)strLen));
            }

            NativeMethods.dbx_string_list_free(list);
            return names;
        }

        /// <summary>
        /// Run garbage collection
        /// </summary>
        public ulong Gc()
        {
            ThrowIfDisposed();
            nuint deleted = 0;
            var result = NativeMethods.dbx_gc(_handle, &deleted);
            if (result != 0)
                throw new Exception($"GC failed with error code: {result}");
            return (ulong)deleted;
        }

        /// <summary>
        /// Check if the database is encrypted
        /// </summary>
        public bool IsEncrypted()
        {
            ThrowIfDisposed();
            return NativeMethods.dbx_is_encrypted(_handle) != 0;
        }

        // ═══════════════════════════════════════════════════
        // SQL Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Execute a SQL statement
        /// </summary>
        public ulong ExecuteSql(string sql)
        {
            ThrowIfDisposed();
            
            var sqlBytes = Encoding.UTF8.GetBytes(sql + "\0");
            nuint affected = 0;
            
            fixed (byte* sqlPtr = sqlBytes)
            {
                var result = NativeMethods.dbx_execute_sql(_handle, sqlPtr, &affected);
                if (result != 0)
                    throw new Exception($"SQL execution failed with error code: {result}");
            }

            return (ulong)affected;
        }

        // ═══════════════════════════════════════════════════
        // DDL API Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Drop a table
        /// </summary>
        public void DropTable(string tableName)
        {
            ThrowIfDisposed();
            var tableBytes = Encoding.UTF8.GetBytes(tableName + "\0");
            
            fixed (byte* tablePtr = tableBytes)
            {
                var result = NativeMethods.dbx_drop_table(_handle, tablePtr);
                if (result != 0)
                    throw new Exception($"Drop table failed with error code: {result}");
            }
        }

        /// <summary>
        /// Check if a table exists
        /// </summary>
        public bool TableExists(string tableName)
        {
            ThrowIfDisposed();
            var tableBytes = Encoding.UTF8.GetBytes(tableName + "\0");
            
            fixed (byte* tablePtr = tableBytes)
            {
                return NativeMethods.dbx_table_exists(_handle, tablePtr) != 0;
            }
        }

        /// <summary>
        /// List all tables
        /// </summary>
        public List<string> ListTables()
        {
            ThrowIfDisposed();

            DbxStringList* list = null;
            var rc = NativeMethods.dbx_list_tables(_handle, &list);
            if (rc != 0)
                throw new Exception($"List tables failed with error code: {rc}");

            var count = NativeMethods.dbx_string_list_count(list);
            var tables = new List<string>((int)count);

            for (nuint i = 0; i < count; i++)
            {
                byte* strPtr = null;
                nuint strLen = 0;
                NativeMethods.dbx_string_list_get(list, i, &strPtr, &strLen);
                tables.Add(Encoding.UTF8.GetString(strPtr, (int)strLen));
            }

            NativeMethods.dbx_string_list_free(list);
            return tables;
        }

        // ═══════════════════════════════════════════════════
        // Index Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Create an index on a table column
        /// </summary>
        public void CreateIndex(string table, string column)
        {
            ThrowIfDisposed();
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            var columnBytes = Encoding.UTF8.GetBytes(column + "\0");
            
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* columnPtr = columnBytes)
            {
                var result = NativeMethods.dbx_create_index(_handle, tablePtr, columnPtr);
                if (result != 0)
                    throw new Exception($"Create index failed with error code: {result}");
            }
        }

        /// <summary>
        /// Drop an index from a table column
        /// </summary>
        public void DropIndex(string table, string column)
        {
            ThrowIfDisposed();
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            var columnBytes = Encoding.UTF8.GetBytes(column + "\0");
            
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* columnPtr = columnBytes)
            {
                var result = NativeMethods.dbx_drop_index(_handle, tablePtr, columnPtr);
                if (result != 0)
                    throw new Exception($"Drop index failed with error code: {result}");
            }
        }

        /// <summary>
        /// Check if an index exists on a table column
        /// </summary>
        public bool HasIndex(string table, string column)
        {
            ThrowIfDisposed();
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            var columnBytes = Encoding.UTF8.GetBytes(column + "\0");
            
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* columnPtr = columnBytes)
            {
                return NativeMethods.dbx_has_index(_handle, tablePtr, columnPtr) != 0;
            }
        }

        // ═══════════════════════════════════════════════════
        // Snapshot Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Save the database to a file
        /// </summary>
        public void SaveToFile(string path)
        {
            ThrowIfDisposed();
            var pathBytes = Encoding.UTF8.GetBytes(path + "\0");
            
            fixed (byte* pathPtr = pathBytes)
            {
                var result = NativeMethods.dbx_save_to_file(_handle, pathPtr);
                if (result != 0)
                    throw new Exception($"Save failed with error code: {result}");
            }
        }

        // ═══════════════════════════════════════════════════
        // MVCC Operations
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Get the current MVCC timestamp
        /// </summary>
        public ulong CurrentTimestamp()
        {
            ThrowIfDisposed();
            return NativeMethods.dbx_current_timestamp(_handle);
        }

        /// <summary>
        /// Allocate a new commit timestamp
        /// </summary>
        public ulong AllocateCommitTs()
        {
            ThrowIfDisposed();
            return NativeMethods.dbx_allocate_commit_ts(_handle);
        }

        /// <summary>
        /// Insert a versioned key-value pair (MVCC)
        /// </summary>
        public void InsertVersioned(string table, byte[] key, byte[] value, ulong commitTs)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            fixed (byte* valuePtr = value)
            {
                var result = NativeMethods.dbx_insert_versioned(
                    _handle, tablePtr,
                    keyPtr, (nuint)key.Length,
                    valuePtr, (nuint)value.Length,
                    commitTs
                );
                
                if (result != 0)
                    throw new Exception($"Versioned insert failed with error code: {result}");
            }
        }

        /// <summary>
        /// Read a specific version of a key (Snapshot Read)
        /// </summary>
        public byte[]? GetSnapshot(string table, byte[] key, ulong readTs)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            byte* outValue = null;
            nuint outLen = 0;
            
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            {
                var result = NativeMethods.dbx_get_snapshot(
                    _handle, tablePtr,
                    keyPtr, (nuint)key.Length,
                    readTs,
                    &outValue, &outLen
                );
                
                if (result == -4) // DBX_ERR_NOT_FOUND
                    return null;
                
                if (result != 0)
                    throw new Exception($"Snapshot read failed with error code: {result}");
                
                if (outValue == null)
                    return null;
                
                try
                {
                    var value = new byte[outLen];
                    Marshal.Copy((IntPtr)outValue, value, 0, (int)outLen);
                    return value;
                }
                finally
                {
                    NativeMethods.dbx_free_value(outValue, outLen);
                }
            }
        }

        // ═══════════════════════════════════════════════════
        // Transaction & Lifecycle
        // ═══════════════════════════════════════════════════

        /// <summary>
        /// Begin a transaction
        /// </summary>
        public Transaction BeginTransaction()
        {
            ThrowIfDisposed();
            return new Transaction(this, _handle);
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(Database));
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != null)
                {
                    NativeMethods.dbx_close(_handle);
                    _handle = null;
                }
                _disposed = true;
            }
        }
    }

    /// <summary>
    /// DBX transaction for batch operations
    /// </summary>
    public unsafe class Transaction : IDisposable
    {
        private readonly Database _database;
        private DbxTransaction* _tx;
        private bool _disposed;

        internal Transaction(Database database, DbxHandle* handle)
        {
            _database = database;
            _tx = NativeMethods.dbx_begin_transaction(handle);
            if (_tx == null)
                throw new Exception("Failed to begin transaction");
        }

        /// <summary>
        /// Insert a key-value pair (buffered)
        /// </summary>
        public void Insert(string table, byte[] key, byte[] value)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            fixed (byte* valuePtr = value)
            {
                var result = NativeMethods.dbx_transaction_insert(
                    _tx, tablePtr,
                    keyPtr, (nuint)key.Length,
                    valuePtr, (nuint)value.Length
                );
                
                if (result != 0)
                    throw new Exception($"Transaction insert failed with error code: {result}");
            }
        }

        /// <summary>
        /// Delete a key (buffered)
        /// </summary>
        public void Delete(string table, byte[] key)
        {
            ThrowIfDisposed();
            
            var tableBytes = Encoding.UTF8.GetBytes(table + "\0");
            fixed (byte* tablePtr = tableBytes)
            fixed (byte* keyPtr = key)
            {
                var result = NativeMethods.dbx_transaction_delete(
                    _tx, tablePtr,
                    keyPtr, (nuint)key.Length
                );
                
                if (result != 0)
                    throw new Exception($"Transaction delete failed with error code: {result}");
            }
        }

        /// <summary>
        /// Commit the transaction
        /// </summary>
        public void Commit()
        {
            ThrowIfDisposed();
            
            var result = NativeMethods.dbx_transaction_commit(_tx);
            if (result != 0)
                throw new Exception($"Transaction commit failed with error code: {result}");
            
            _tx = null;
            _disposed = true;
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(Transaction));
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                // Transaction is auto-freed on commit
                _disposed = true;
            }
        }
    }

    /// <summary>
    /// Zero-copy scan result - owns serialized flat buffer data
    /// </summary>
    public unsafe class ZeroCopyScanResult : IDisposable
    {
        private DbxZeroCopyScanResult* _handle;
        private bool _disposed;

        internal ZeroCopyScanResult(DbxZeroCopyScanResult* handle)
        {
            _handle = handle;
        }

        /// <summary>
        /// Get the number of key-value pairs
        /// </summary>
        public int Count
        {
            get
            {
                ThrowIfDisposed();
                return (int)NativeMethods.dbx_zero_copy_result_count(_handle);
            }
        }

        /// <summary>
        /// Get raw data as Span (zero-copy access, .NET 6+ only)
        /// </summary>
#if NET6_0_OR_GREATER
        public unsafe Span<byte> AsSpan()
        {
            ThrowIfDisposed();
            
            byte* dataPtr = null;
            nuint dataLen = 0;
            
            var rc = NativeMethods.dbx_zero_copy_result_data(_handle, &dataPtr, &dataLen);
            if (rc != 0)
                throw new Exception($"Failed to get zero-copy data: {rc}");
            
            return new Span<byte>(dataPtr, (int)dataLen);
        }
#endif

        /// <summary>
        /// Get raw data as byte array (requires copy for safety in .NET Standard 2.0)
        /// </summary>
        public byte[] GetRawData()
        {
            ThrowIfDisposed();
            
            byte* dataPtr = null;
            nuint dataLen = 0;
            
            var rc = NativeMethods.dbx_zero_copy_result_data(_handle, &dataPtr, &dataLen);
            if (rc != 0)
                throw new Exception($"Failed to get zero-copy data: {rc}");
            
            var result = new byte[dataLen];
            Marshal.Copy((IntPtr)dataPtr, result, 0, (int)dataLen);
            return result;
        }

        /// <summary>
        /// Parse the flat buffer into key-value pairs (requires copy)
        /// </summary>
        public List<KeyValuePair<byte[], byte[]>> ToPairs()
        {
            ThrowIfDisposed();
            
#if NET6_0_OR_GREATER
            var data = AsSpan();
#else
            var data = GetRawData();
#endif
            var count = Count;
            var pairs = new List<KeyValuePair<byte[], byte[]>>(count);
            
            int offset = 0;
#if NET6_0_OR_GREATER
            for (int i = 0; i < count; i++)
            {
                // Read key length
                if (offset + 4 > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var keyLen = BitConverter.ToUInt32(data.Slice(offset, 4));
                offset += 4;
                
                // Read key
                if (offset + (int)keyLen > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var key = data.Slice(offset, (int)keyLen).ToArray();
                offset += (int)keyLen;
                
                // Read value length
                if (offset + 4 > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var valueLen = BitConverter.ToUInt32(data.Slice(offset, 4));
                offset += 4;
                
                // Read value
                if (offset + (int)valueLen > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var value = data.Slice(offset, (int)valueLen).ToArray();
                offset += (int)valueLen;
                
                pairs.Add(new KeyValuePair<byte[], byte[]>(key, value));
            }
#else
            for (int i = 0; i < count; i++)
            {
                // Read key length
                if (offset + 4 > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var keyLen = BitConverter.ToUInt32(data, offset);
                offset += 4;
                
                // Read key
                if (offset + (int)keyLen > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var key = new byte[keyLen];
                Array.Copy(data, offset, key, 0, (int)keyLen);
                offset += (int)keyLen;
                
                // Read value length
                if (offset + 4 > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var valueLen = BitConverter.ToUInt32(data, offset);
                offset += 4;
                
                // Read value
                if (offset + (int)valueLen > data.Length)
                    throw new InvalidOperationException("Invalid data format");
                
                var value = new byte[valueLen];
                Array.Copy(data, offset, value, 0, (int)valueLen);
                offset += (int)valueLen;
                
                pairs.Add(new KeyValuePair<byte[], byte[]>(key, value));
            }
#endif
            
            return pairs;
        }

        private void ThrowIfDisposed()
        {
            if (_disposed)
                throw new ObjectDisposedException(nameof(ZeroCopyScanResult));
        }

        public void Dispose()
        {
            if (!_disposed)
            {
                if (_handle != null)
                {
                    NativeMethods.dbx_zero_copy_result_free(_handle);
                    _handle = null;
                }
                _disposed = true;
            }
        }
    }
}
