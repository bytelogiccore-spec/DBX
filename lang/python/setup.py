"""
DBX Python Bindings Setup
"""
import os
import sys
import subprocess
from pathlib import Path
from setuptools import setup
from setuptools.command.build_py import build_py


class BuildRustExtension(build_py):
    """Custom build command to compile Rust FFI library"""
    
    def run(self):
        # Build the Rust FFI library
        rust_project_root = Path(__file__).parent.parent.parent
        
        print("Building Rust FFI library...")
        try:
            subprocess.check_call(
                ["cargo", "build", "--release", "-p", "dbx-ffi"],
                cwd=rust_project_root
            )
        except subprocess.CalledProcessError as e:
            print(f"Error building Rust library: {e}", file=sys.stderr)
            sys.exit(1)
        
        # Copy the built library to the package directory
        target_dir = rust_project_root / "target" / "release"
        package_dir = Path(__file__).parent / "dbx_py"
        
        # Determine library name based on platform
        if sys.platform == "win32":
            lib_name = "dbx_ffi.dll"
        elif sys.platform == "darwin":
            lib_name = "libdbx_ffi.dylib"
        else:
            lib_name = "libdbx_ffi.so"
        
        src_lib = target_dir / lib_name
        dst_lib = package_dir / lib_name
        
        if src_lib.exists():
            print(f"Copying {src_lib} to {dst_lib}")
            import shutil
            shutil.copy2(src_lib, dst_lib)
        else:
            print(f"Warning: Library not found at {src_lib}", file=sys.stderr)
        
        # Continue with normal build
        super().run()


setup(
    cmdclass={
        'build_py': BuildRustExtension,
    },
    package_data={
        'dbx_py': ['*.dll', '*.so', '*.dylib'],
    },
    include_package_data=True,
)
