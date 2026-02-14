use csbindgen::Builder;
use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let output_dir = format!("{}/../../lang/dotnet/DBX.Native", crate_dir);
    
    Builder::default()
        .input_extern_file("src/lib.rs")
        .csharp_dll_name("dbx_csharp")
        .csharp_namespace("DBX.Native")
        .csharp_class_name("NativeMethods")
        .csharp_class_accessibility("public")
        .generate_csharp_file(&format!("{}/NativeMethods.g.cs", output_dir))
        .unwrap();
    
    println!("cargo:rerun-if-changed=src/lib.rs");
}
