using System;
using System.Runtime.InteropServices;

namespace DBX.Client
{
    internal static class NativeMethods
    {
        private const string LibName = "dbx_ffi";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr dbx_open([MarshalAs(UnmanagedType.LPStr)] string path);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr dbx_open_in_memory();

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void dbx_close(IntPtr ctx);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr dbx_get_table([MarshalAs(UnmanagedType.LPStr)] string table);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void dbx_free_table(IntPtr handle);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_insert_fast(
            IntPtr ctx,
            IntPtr tableHandle,
            byte[] key,
            UIntPtr keyLen,
            byte[] value,
            UIntPtr valueLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_get(
            IntPtr ctx,
            IntPtr tableHandle,
            byte[] key,
            UIntPtr keyLen,
            out IntPtr outVal,
            out UIntPtr outLen);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern void dbx_free_buffer(IntPtr ptr, UIntPtr len);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern IntPtr dbx_begin(IntPtr ctx);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_commit(IntPtr tx);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_insert_batch(
            IntPtr ctx,
            IntPtr tableHandle,
            IntPtr[] keys,
            UIntPtr[] keyLens,
            IntPtr[] values,
            UIntPtr[] valueLens,
            UIntPtr count);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_set_durability(IntPtr ctx, int level);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        public static extern int dbx_insert_batch_packed(
            IntPtr ctx,
            IntPtr tableHandle,
            byte[] keyBuf,
            byte[] valBuf,
            UIntPtr[] keyLens,
            UIntPtr[] valLens,
            UIntPtr count);
    }
}
