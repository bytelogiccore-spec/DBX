"""DBX Database wrapper for Python"""

import ctypes
import os
import platform
from pathlib import Path
from typing import Optional


class Database:
    """DBX Database wrapper
    
    Example:
        >>> db = Database("./my_db")
        >>> db.insert("users", b"user:1", b"Alice")
        >>> value = db.get("users", b"user:1")
        >>> print(value)
        b'Alice'
    """
    
    def __init__(self, path: str):
        """Open a database at the given path
        
        Args:
            path: Path to the database directory
        """
        self._lib = self._load_library()
        self._setup_functions()
        
        path_bytes = path.encode('utf-8')
        self._handle = self._lib.dbx_open(path_bytes)
        
        if not self._handle:
            raise RuntimeError(f"Failed to open database at {path}")
    
    @classmethod
    def open_in_memory(cls) -> 'Database':
        """Open an in-memory database
        
        Returns:
            Database instance
        """
        db = cls.__new__(cls)
        db._lib = db._load_library()
        db._setup_functions()
        db._handle = db._lib.dbx_open_in_memory()
        
        if not db._handle:
            raise RuntimeError("Failed to open in-memory database")
        
        return db
    
    def insert(self, table: str, key: bytes, value: bytes) -> None:
        """Insert a key-value pair into a table
        
        Args:
            table: Table name
            key: Key as bytes
            value: Value as bytes
        
        Raises:
            RuntimeError: If insert fails
        """
        table_bytes = table.encode('utf-8')
        key_array = (ctypes.c_uint8 * len(key)).from_buffer_copy(key)
        value_array = (ctypes.c_uint8 * len(value)).from_buffer_copy(value)
        
        result = self._lib.dbx_insert(
            self._handle,
            table_bytes,
            key_array, len(key),
            value_array, len(value)
        )
        
        if result != 0:
            raise RuntimeError(f"Insert failed with error code: {result}")
    
    def get(self, table: str, key: bytes) -> Optional[bytes]:
        """Get a value by key from a table
        
        Args:
            table: Table name
            key: Key as bytes
        
        Returns:
            Value as bytes, or None if not found
        
        Raises:
            RuntimeError: If get fails
        """
        table_bytes = table.encode('utf-8')
        key_array = (ctypes.c_uint8 * len(key)).from_buffer_copy(key)
        out_value = ctypes.POINTER(ctypes.c_uint8)()
        out_len = ctypes.c_size_t()
        
        result = self._lib.dbx_get(
            self._handle,
            table_bytes,
            key_array, len(key),
            ctypes.byref(out_value),
            ctypes.byref(out_len)
        )
        
        if result == -4:  # DBX_ERR_NOT_FOUND
            return None
        elif result != 0:
            raise RuntimeError(f"Get failed with error code: {result}")
        
        # Copy the data
        value = bytes(ctypes.cast(out_value, ctypes.POINTER(ctypes.c_uint8 * out_len.value)).contents)
        
        # Free the memory
        self._lib.dbx_free_value(out_value, out_len)
        
        return value
    
    def delete(self, table: str, key: bytes) -> None:
        """Delete a key from a table
        
        Args:
            table: Table name
            key: Key as bytes
        
        Raises:
            RuntimeError: If delete fails
        """
        table_bytes = table.encode('utf-8')
        key_array = (ctypes.c_uint8 * len(key)).from_buffer_copy(key)
        
        result = self._lib.dbx_delete(
            self._handle,
            table_bytes,
            key_array, len(key)
        )
        
        if result != 0:
            raise RuntimeError(f"Delete failed with error code: {result}")
    
    def count(self, table: str) -> int:
        """Count rows in a table
        
        Args:
            table: Table name
        
        Returns:
            Number of rows
        
        Raises:
            RuntimeError: If count fails
        """
        table_bytes = table.encode('utf-8')
        out_count = ctypes.c_size_t()
        
        result = self._lib.dbx_count(
            self._handle,
            table_bytes,
            ctypes.byref(out_count)
        )
        
        if result != 0:
            raise RuntimeError(f"Count failed with error code: {result}")
        
        return out_count.value
    
    def flush(self) -> None:
        """Flush database to disk
        
        Raises:
            RuntimeError: If flush fails
        """
        result = self._lib.dbx_flush(self._handle)
        
        if result != 0:
            raise RuntimeError(f"Flush failed with error code: {result}")
    
    def close(self) -> None:
        """Close the database and free resources"""
        if self._handle:
            self._lib.dbx_close(self._handle)
            self._handle = None
    
    def __enter__(self):
        """Context manager entry"""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        self.close()
    
    def __del__(self):
        """Destructor"""
        self.close()
    
    def _load_library(self) -> ctypes.CDLL:
        """Load the DBX shared library
        
        Returns:
            ctypes.CDLL instance
        """
        # Determine library name based on platform
        system = platform.system()
        if system == "Windows":
            lib_name = "dbx_ffi.dll"
        elif system == "Darwin":
            lib_name = "libdbx_ffi.dylib"
        else:
            lib_name = "libdbx_ffi.so"
        
        # Try to find the library
        search_paths = [
            # Development build
            Path(__file__).parent.parent.parent.parent / "core" / "dbx-ffi" / "target" / "release" / lib_name,
            # Installed package
            Path(__file__).parent / lib_name,
            # System library
            lib_name,
        ]
        
        for path in search_paths:
            try:
                return ctypes.CDLL(str(path))
            except OSError:
                continue
        
        raise RuntimeError(f"Could not find DBX library ({lib_name})")
    
    def _setup_functions(self) -> None:
        """Setup function signatures"""
        # dbx_open
        self._lib.dbx_open.argtypes = [ctypes.c_char_p]
        self._lib.dbx_open.restype = ctypes.c_void_p
        
        # dbx_open_in_memory
        self._lib.dbx_open_in_memory.argtypes = []
        self._lib.dbx_open_in_memory.restype = ctypes.c_void_p
        
        # dbx_insert
        self._lib.dbx_insert.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_uint8),  # key
            ctypes.c_size_t,  # key_len
            ctypes.POINTER(ctypes.c_uint8),  # value
            ctypes.c_size_t,  # value_len
        ]
        self._lib.dbx_insert.restype = ctypes.c_int
        
        # dbx_get
        self._lib.dbx_get.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_uint8),  # key
            ctypes.c_size_t,  # key_len
            ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)),  # out_value
            ctypes.POINTER(ctypes.c_size_t),  # out_len
        ]
        self._lib.dbx_get.restype = ctypes.c_int
        
        # dbx_delete
        self._lib.dbx_delete.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_uint8),  # key
            ctypes.c_size_t,  # key_len
        ]
        self._lib.dbx_delete.restype = ctypes.c_int
        
        # dbx_count
        self._lib.dbx_count.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_size_t),  # out_count
        ]
        self._lib.dbx_count.restype = ctypes.c_int
        
        # dbx_flush
        self._lib.dbx_flush.argtypes = [ctypes.c_void_p]
        self._lib.dbx_flush.restype = ctypes.c_int
        
        # dbx_free_value
        self._lib.dbx_free_value.argtypes = [
            ctypes.POINTER(ctypes.c_uint8),
            ctypes.c_size_t
        ]
        self._lib.dbx_free_value.restype = None
        
        # dbx_close
        self._lib.dbx_close.argtypes = [ctypes.c_void_p]
        self._lib.dbx_close.restype = None
        
        # Transaction API
        # dbx_begin_transaction
        self._lib.dbx_begin_transaction.argtypes = [ctypes.c_void_p]
        self._lib.dbx_begin_transaction.restype = ctypes.c_void_p
        
        # dbx_transaction_insert
        self._lib.dbx_transaction_insert.argtypes = [
            ctypes.c_void_p,  # tx
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_uint8),  # key
            ctypes.c_size_t,  # key_len
            ctypes.POINTER(ctypes.c_uint8),  # value
            ctypes.c_size_t,  # value_len
        ]
        self._lib.dbx_transaction_insert.restype = ctypes.c_int
        
        # dbx_transaction_delete
        self._lib.dbx_transaction_delete.argtypes = [
            ctypes.c_void_p,  # tx
            ctypes.c_char_p,  # table
            ctypes.POINTER(ctypes.c_uint8),  # key
            ctypes.c_size_t,  # key_len
        ]
        self._lib.dbx_transaction_delete.restype = ctypes.c_int
        
        # dbx_transaction_commit
        self._lib.dbx_transaction_commit.argtypes = [ctypes.c_void_p]
        self._lib.dbx_transaction_commit.restype = ctypes.c_int
        
        # dbx_transaction_rollback
        self._lib.dbx_transaction_rollback.argtypes = [ctypes.c_void_p]
        self._lib.dbx_transaction_rollback.restype = None
