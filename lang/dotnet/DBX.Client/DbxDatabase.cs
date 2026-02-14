using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;

namespace DBX.Client
{
    public enum DurabilityLevel
    {
        Full = 0,
        Lazy = 1,
        None = 2
    }

    public class DbxDatabase : IDisposable
    {
        private IntPtr _handle;
        private bool _disposed;
        private readonly ConcurrentDictionary<string, IntPtr> _tableHandles = new ConcurrentDictionary<string, IntPtr>();

        public DbxDatabase(string path)
        {
            _handle = NativeMethods.dbx_open(path);
            if (_handle == IntPtr.Zero) throw new Exception("DB Open Fail");
        }

        private DbxDatabase(IntPtr handle)
        {
            _handle = handle;
        }

        public static DbxDatabase CreateInMemory()
        {
            var h = NativeMethods.dbx_open_in_memory();
            if (h == IntPtr.Zero) throw new Exception("Memory DB Open Fail");
            return new DbxDatabase(h);
        }

        public void SetDurability(DurabilityLevel level)
        {
            NativeMethods.dbx_set_durability(_handle, (int)level);
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        internal IntPtr GetTableHandle(string table)
        {
            if (_tableHandles.TryGetValue(table, out IntPtr h)) return h;
            h = NativeMethods.dbx_get_table(table);
            _tableHandles.TryAdd(table, h);
            return h;
        }

        public void Insert(string table, byte[] key, byte[] value)
        {
            NativeMethods.dbx_insert_fast(_handle, GetTableHandle(table), key, (UIntPtr)key.Length, value, (UIntPtr)value.Length);
        }

        // Optimized for fixed-size pre-allocated buffers
        internal unsafe void InternalBatchInsert(IntPtr tableHandle, (byte[] key, byte[] value)[] buffer, int count)
        {
            if (count == 0) return;

            IntPtr[] keys = new IntPtr[count];
            UIntPtr[] keyLens = new UIntPtr[count];
            IntPtr[] values = new IntPtr[count];
            UIntPtr[] valueLens = new UIntPtr[count];
            GCHandle[] pinnings = new GCHandle[count * 2];

            try
            {
                for (int i = 0; i < count; i++)
                {
                    var row = buffer[i];
                    var hK = GCHandle.Alloc(row.key, GCHandleType.Pinned);
                    var hV = GCHandle.Alloc(row.value, GCHandleType.Pinned);
                    pinnings[i * 2] = hK;
                    pinnings[i * 2 + 1] = hV;

                    keys[i] = hK.AddrOfPinnedObject();
                    keyLens[i] = (UIntPtr)row.key.Length;
                    values[i] = hV.AddrOfPinnedObject();
                    valueLens[i] = (UIntPtr)row.value.Length;
                }

                // Call Native FFI via IntPtr arrays
                NativeMethods.dbx_insert_batch(_handle, tableHandle, keys, keyLens, values, valueLens, (UIntPtr)count);
            }
            finally
            {
                // Batch-free all handles
                for (int i = 0; i < pinnings.Length; i++) pinnings[i].Free();
            }
        }

        public byte[] Get(string table, byte[] key)
        {
            int res = NativeMethods.dbx_get(_handle, GetTableHandle(table), key, (UIntPtr)key.Length, out IntPtr v, out UIntPtr l);
            if (res <= 0) return null;
            byte[] b = new byte[(int)l];
            Marshal.Copy(v, b, 0, (int)l);
            NativeMethods.dbx_free_buffer(v, l);
            return b;
        }

        public DbxTransaction BeginTransaction() => new DbxTransaction(this, NativeMethods.dbx_begin(_handle));

        public void Dispose()
        {
            if (!_disposed)
            {
                foreach (var h in _tableHandles.Values) NativeMethods.dbx_free_table(h);
                if (_handle != IntPtr.Zero) NativeMethods.dbx_close(_handle);
                _disposed = true;
            }
        }
    }

    public class DbxTransaction : IDisposable
    {
        private readonly DbxDatabase _db;
        private IntPtr _handle;
        private bool _done;
        private string _table = "benchmark";
        
        // Use fixed-size Array instead of List to avoid growth overhead
        private readonly (byte[] key, byte[] value)[] _buffer = new (byte[], byte[])[10000];
        private int _count = 0;

        internal DbxTransaction(DbxDatabase db, IntPtr handle) { _db = db; _handle = handle; }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public void Insert(string table, byte[] key, byte[] value)
        {
            _table = table;
            _buffer[_count++] = (key, value);
        }

        private void Flush()
        {
            if (_count == 0) return;
            _db.InternalBatchInsert(_db.GetTableHandle(_table), _buffer, _count);
            _count = 0;
        }

        public void Commit()
        {
            if (_done) return;
            Flush(); // Final batch send
            NativeMethods.dbx_commit(_handle);
            _done = true;
        }
        public void Dispose() { if (!_done) Commit(); }
    }
}
