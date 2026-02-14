using System;
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
                    _handle,
                    tablePtr,
                    keyPtr,
                    (nuint)key.Length,
                    valuePtr,
                    (nuint)value.Length
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
                    _handle,
                    tablePtr,
                    keyPtr,
                    (nuint)key.Length,
                    &outValue,
                    &outLen
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
                    _handle,
                    tablePtr,
                    keyPtr,
                    (nuint)key.Length
                );
                
                if (result != 0)
                    throw new Exception($"Delete failed with error code: {result}");
            }
        }

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
                    _tx,
                    tablePtr,
                    keyPtr,
                    (nuint)key.Length,
                    valuePtr,
                    (nuint)value.Length
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
                    _tx,
                    tablePtr,
                    keyPtr,
                    (nuint)key.Length
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
}
